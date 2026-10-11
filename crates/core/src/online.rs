//! Online play: what crosses the wire, and the bookkeeping either end keeps.
//!
//! The transport is Vagrancy's `rtc.js`, itself cut down from Floodline's
//! `quad_rtc.js`: WebRTC between browsers, introduced by a room code over
//! public Nostr relays (Trystero) or by a pasted code, in a star: every
//! joiner talks only to the host. This file never sees a socket; the page
//! hands it each message as text and sends what it leaves in the outbox.
//!
//! The scheme is a host's tick stream (PLAN.md P16). The host runs the round
//! on its own clock. For every tick it decides whose input counts, which is
//! the player whose turn it is, and sends that input to every joiner as one
//! `Step`. Each joiner runs the same round from the same start and the same
//! steps, so the copies agree tick for tick. A joiner's own keys go to the
//! host, which uses them on that joiner's turn. Golf is played one frog at a
//! time, so nobody ever waits on anybody but the player whose turn it is.
//!
//! A whack is judged by where the needle stood on the swinging player's own
//! screen (`Press::needle`), so the round trip to the host does not move it.
//! Every 60 ticks each joiner sends a checksum of its round, and the host
//! compares it with its own.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Bumped when a message changes shape; a joiner on another version is turned away.
pub const PROTO: u32 = 1;
/// Ticks between checksums.
pub const SUM_EVERY: u32 = 60;
/// Checksums the host keeps to compare against late joiners' reports.
pub const SUMS_KEPT: usize = 64;
/// A joiner more than this many ticks behind plays the backlog down to it
/// at once rather than one tick a tick.
pub const CATCH_UP_ABOVE: usize = 8;
pub const CATCH_UP_TO: usize = 2;
/// A joiner sends its held keys at least this often, in frames, even unchanged.
pub const HOLD_EVERY: u32 = 20;

/// Something the host decides that is not a key: it happens on every copy at
/// the same tick.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Cmd {
    TeeOff,
    NextHole,
    /// this player's person left; the CPU plays the seat from now on
    Cpu(usize),
}

/// One tick's input: held keys, keys pressed this tick, the camera's heading
/// (steering is relative to it), the needle a whack was judged at, and a
/// command.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Step {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub b: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub e: u32,
    #[serde(default, skip_serializing_if = "is_zero_i")]
    pub y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c: Option<Cmd>,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

fn is_zero_i(v: &i32) -> bool {
    *v == 0
}

/// A seat as the lobby shows it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Seat {
    /// human (at the host's keyboard), remote, open, cpu or off
    pub kind: String,
    pub frog: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Msg {
    /// Joiner to host, first.
    Hello { proto: u32, build: String },
    /// Host to joiner: the lobby, and which seat is the joiner's.
    Lobby {
        seats: Vec<Seat>,
        choice: usize,
        you: usize,
    },
    /// Joiner to host: the next or previous free frog for my seat.
    Pick { forward: bool },
    /// Host to joiner: the round begins. `players` as `Round::new` takes
    /// them, `you` the joiner's player, `owners` each player's seat kind.
    Start {
        players: Vec<(usize, bool)>,
        owners: Vec<String>,
        plan: Vec<usize>,
        seed: u64,
        you: usize,
    },
    /// Joiner to host: the keys I am holding, and my camera's heading.
    Hold { b: u32, y: i32 },
    /// Joiner to host: I pressed this action; for a whack, where my needle was.
    Press { a: u32, n: Option<i32> },
    /// Host to joiner: the ticks from `from` on.
    Steps { from: u32, s: Vec<Step> },
    /// Joiner to host: my round's checksum after `tick`.
    Sum { tick: u32, sum: u64 },
    /// Host to joiner: the copies disagreed after `tick`.
    Desync { tick: u32 },
    /// Either way: goodbye, and why (a key in copy.json's online words).
    Bye { why: String },
}

pub fn encode(m: &Msg) -> String {
    serde_json::to_string(m).unwrap_or_default()
}

pub fn decode(text: &str) -> Option<Msg> {
    serde_json::from_str(text).ok()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Host,
    Joiner,
}

/// Who plays a player in an online round, seen from this end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    /// at this keyboard
    Here,
    /// at another browser: the host knows which peer; a joiner does not need to
    There(u32),
    Cpu,
}

#[derive(Debug, Clone)]
pub struct Online {
    pub role: Role,
    pub room: String,
    pub build: String,
    /// a key in copy.json's online words: what the lobby says
    pub status: &'static str,
    pub outbox: Vec<(u32, String)>,
    /// host: peers that said hello, and their seats
    pub peers: BTreeMap<u32, usize>,
    /// host: what each peer holds, and the presses it has not had used
    pub holds: BTreeMap<u32, (u32, i32)>,
    pub presses: BTreeMap<u32, VecDeque<(u32, Option<i32>)>>,
    pub cmds: VecDeque<Cmd>,
    /// host: steps made since the last send, and the tick of the first
    pub fresh: Vec<Step>,
    pub fresh_from: u32,
    pub sums: BTreeMap<u32, u64>,
    /// the next tick to run (both ends)
    pub tick: u32,
    /// joiner: the host's peer id, the lobby it last sent, my seat, my player
    pub host: Option<u32>,
    pub lobby: Vec<Seat>,
    pub choice: usize,
    pub seat: usize,
    pub inbox: VecDeque<Step>,
    pub last_hold: (u32, i32),
    pub hold_age: u32,
    /// per player: who plays it (once a round has started)
    pub owners: Vec<Owner>,
    pub desync: Option<u32>,
}

impl Online {
    pub fn new(role: Role, room: &str, build: &str) -> Online {
        Online {
            role,
            room: room.to_string(),
            build: build.to_string(),
            status: if role == Role::Host {
                "online_hosting"
            } else {
                "online_joining"
            },
            outbox: Vec::new(),
            peers: BTreeMap::new(),
            holds: BTreeMap::new(),
            presses: BTreeMap::new(),
            cmds: VecDeque::new(),
            fresh: Vec::new(),
            fresh_from: 0,
            sums: BTreeMap::new(),
            tick: 0,
            host: None,
            lobby: Vec::new(),
            choice: 0,
            seat: 0,
            inbox: VecDeque::new(),
            last_hold: (0, 0),
            hold_age: 0,
            owners: Vec::new(),
            desync: None,
        }
    }

    pub fn send(&mut self, peer: u32, m: &Msg) {
        self.outbox.push((peer, encode(m)));
    }

    pub fn to_host(&mut self, m: &Msg) {
        if let Some(h) = self.host {
            self.send(h, m);
        }
    }

    /// Host: the input for a tick in which `owner` plays. A peer's presses
    /// are used one a tick, in order; a peer that is not playing has its
    /// presses dropped, so a key pressed while waiting does nothing later.
    pub fn input_for(&mut self, owner: Option<Owner>, here: Step) -> Step {
        let playing = match owner {
            Some(Owner::There(p)) => Some(p),
            _ => None,
        };
        for (p, q) in self.presses.iter_mut() {
            if Some(*p) != playing {
                q.clear();
            }
        }
        let mut s = match owner {
            Some(Owner::Here) => here,
            Some(Owner::There(p)) => {
                let (b, y) = self.holds.get(&p).copied().unwrap_or((0, here.y));
                let mut s = Step {
                    b,
                    y,
                    ..Step::default()
                };
                if let Some((a, n)) = self.presses.get_mut(&p).and_then(|q| q.pop_front()) {
                    s.e = a;
                    s.n = n;
                }
                s
            }
            _ => Step {
                y: here.y,
                ..Step::default()
            },
        };
        s.c = self.cmds.pop_front();
        s
    }

    /// Host: a step was run; keep it to send, and note the checksum.
    pub fn ran(&mut self, s: Step, sum: Option<u64>) {
        if self.fresh.is_empty() {
            self.fresh_from = self.tick;
        }
        self.fresh.push(s);
        if let Some(sum) = sum {
            if self.tick.is_multiple_of(SUM_EVERY) {
                self.sums.insert(self.tick, sum);
                while self.sums.len() > SUMS_KEPT {
                    let first = *self.sums.keys().next().unwrap();
                    self.sums.remove(&first);
                }
            }
        }
        self.tick += 1;
    }

    /// Host: send the steps made this frame to every joiner.
    pub fn flush_steps(&mut self) {
        if self.fresh.is_empty() {
            return;
        }
        let m = Msg::Steps {
            from: self.fresh_from,
            s: std::mem::take(&mut self.fresh),
        };
        let peers: Vec<u32> = self.peers.keys().copied().collect();
        for p in peers {
            self.send(p, &m);
        }
    }

    /// Joiner: how many queued steps to run this frame, given how many the
    /// clock says are due.
    pub fn steps_to_run(&self, due: usize) -> usize {
        let have = self.inbox.len();
        if have > CATCH_UP_ABOVE {
            have - CATCH_UP_TO
        } else {
            due.min(have)
        }
    }
}

/// A room code from a number the page supplies (its clock): five letters and
/// digits with none that read alike.
pub fn room_code(seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut r = crate::rng::Rng::new(seed);
    (0..5)
        .map(|_| ALPHABET[r.range(0, ALPHABET.len() as i32 - 1) as usize] as char)
        .collect()
}

/// A room code a person typed, tidied, or `None` if it cannot be one.
pub fn tidy_code(text: &str) -> Option<String> {
    let t: String = text
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if (4..=12).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_round_trips_and_a_quiet_step_is_small() {
        let msgs = vec![
            Msg::Hello {
                proto: PROTO,
                build: "abc".into(),
            },
            Msg::Lobby {
                seats: vec![Seat {
                    kind: "human".into(),
                    frog: 0,
                }],
                choice: 2,
                you: 1,
            },
            Msg::Pick { forward: true },
            Msg::Start {
                players: vec![(0, false), (3, true)],
                owners: vec!["human".into(), "cpu".into()],
                plan: vec![0, 1],
                seed: 9,
                you: 0,
            },
            Msg::Hold { b: 5, y: -300 },
            Msg::Press {
                a: 16,
                n: Some(32_000),
            },
            Msg::Steps {
                from: 7,
                s: vec![
                    Step::default(),
                    Step {
                        c: Some(Cmd::Cpu(2)),
                        ..Step::default()
                    },
                ],
            },
            Msg::Sum {
                tick: 120,
                sum: u64::MAX - 3,
            },
            Msg::Desync { tick: 60 },
            Msg::Bye {
                why: "online_full".into(),
            },
        ];
        for m in msgs {
            assert_eq!(decode(&encode(&m)), Some(m));
        }
        assert_eq!(
            encode(&Msg::Steps {
                from: 0,
                s: vec![Step::default()]
            }),
            r#"{"t":"Steps","from":0,"s":[{}]}"#
        );
        assert_eq!(decode("not json"), None);
    }

    #[test]
    fn the_playing_peer_s_presses_are_used_one_a_tick_and_others_are_dropped() {
        let mut o = Online::new(Role::Host, "ROOM", "b");
        o.holds.insert(7, (4, 99));
        o.presses
            .entry(7)
            .or_default()
            .extend([(16, None), (16, Some(500))]);
        o.presses.entry(8).or_default().push_back((32, None));
        let here = Step {
            b: 1,
            e: 16,
            y: 5,
            ..Step::default()
        };
        let a = o.input_for(Some(Owner::There(7)), here);
        assert_eq!((a.b, a.e, a.y, a.n), (4, 16, 99, None));
        let b = o.input_for(Some(Owner::There(7)), here);
        assert_eq!(
            (b.e, b.n),
            (16, Some(500)),
            "the second press, with its needle, a tick later"
        );
        assert!(o.presses[&8].is_empty(), "a waiting peer's press is dropped");
        let c = o.input_for(Some(Owner::Here), here);
        assert_eq!(c, here, "the host's own keys on the host's turn");
        let d = o.input_for(Some(Owner::Cpu), here);
        assert_eq!((d.b, d.e), (0, 0), "nobody's keys on a CPU's turn");
    }

    #[test]
    fn a_joiner_plays_one_due_step_a_tick_and_catches_up_a_backlog() {
        let mut o = Online::new(Role::Joiner, "ROOM", "b");
        o.inbox.extend([Step::default(); 3]);
        assert_eq!(o.steps_to_run(1), 1);
        assert_eq!(o.steps_to_run(5), 3);
        o.inbox.extend([Step::default(); 20]);
        assert_eq!(o.steps_to_run(1), 23 - CATCH_UP_TO);
    }

    #[test]
    fn room_codes_are_five_unambiguous_characters_and_typed_codes_are_tidied() {
        let a = room_code(1);
        assert_eq!(a.len(), 5);
        assert!(!a.contains('O') && !a.contains('0') && !a.contains('I') && !a.contains('1'));
        assert_ne!(room_code(1), room_code(2));
        assert_eq!(tidy_code(" ab3k9 "), Some("AB3K9".into()));
        assert_eq!(tidy_code("a-b"), None);
        assert_eq!(tidy_code("ab"), None);
    }
}
