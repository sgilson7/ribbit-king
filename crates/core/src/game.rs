//! The game as the page sees it: which screen is up, the setup of a round,
//! the fixed-step clock, the camera, the HUD and the screens as JSON, and
//! the render contract (what to upload and draw) as plain numbers.

use crate::balance::*;
use crate::content::{self, hex, parse_course, parse_frogs, FrogDef, HoleDef, Palette};
use crate::course::Course;
use crate::draw::{self, Particle, View};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, v3, V3};
use crate::mesh::{self, Mesh};
use crate::online::{self, Cmd, Msg, Online, Owner, Role, Seat, Step};
use crate::render::{self, Batch, Camera};
use crate::rng::Rng;
use crate::round::{heading_to, Input, Note, Phase, Round};
use crate::scenery;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Each action's bit; data/controls.json names the action for each key.
pub const ACTIONS: &[(&str, u32)] = &[
    ("left", 1),
    ("right", 2),
    ("forward", 4),
    ("back", 8),
    ("swing", 16),
    ("tongue", 32),
    ("loft", 64),
    ("cancel", 128),
    ("drag", 256),
];

pub fn bit(name: &str) -> u32 {
    ACTIONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Title,
    Setup,
    Intro,
    Play,
    Paused,
    Card,
    Final,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Title => "title",
            Mode::Setup => "setup",
            Mode::Intro => "intro",
            Mode::Play => "play",
            Mode::Paused => "paused",
            Mode::Card => "card",
            Mode::Final => "final",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Human,
    Cpu,
    Off,
    /// hosting online: waiting for someone to join into this seat
    Open,
    /// hosting online: taken by the browser with this peer id
    Remote(u32),
}

impl Slot {
    fn name(self) -> &'static str {
        match self {
            Slot::Human => "human",
            Slot::Cpu => "cpu",
            Slot::Off => "off",
            Slot::Open => "open",
            Slot::Remote(_) => "remote",
        }
    }

    /// Plays in a round when it starts.
    fn plays(self) -> bool {
        !matches!(self, Slot::Off | Slot::Open)
    }
}

/// The keys a joiner's presses may carry: the four that happen once.
const PRESS_ACTIONS: &[&str] = &["swing", "tongue", "loft", "cancel"];
/// The keys a joiner's held keys may carry.
const HOLD_ACTIONS: &[&str] = &["left", "right", "forward", "back"];
/// What the online lobby may say, in copy.json's online words.
const ONLINE_WORDS: &[&str] = &[
    "online_hosting",
    "online_joining",
    "online_in_lobby",
    "online_full",
    "online_build",
    "online_started",
    "online_left",
    "online_host_left",
    "online_no_relay",
    "online_no_path",
    "online_bad_code",
];

pub const SLOTS: usize = 4;
const SAYS_KEPT: usize = 4;

pub struct Game {
    frogs: Vec<FrogDef>,
    pal: Palette,
    holes: Vec<HoleDef>,
    pub mode: Mode,
    slots: [(Slot, usize); SLOTS],
    /// 0: the full round; k: hole k alone
    choice: usize,
    round: Option<Round>,
    plan: Vec<usize>,
    at: usize,
    course: Course,
    particles: Vec<Particle>,
    prng: Rng,
    pub last_us: Option<i64>,
    acc: i64,
    prev_bits: u32,
    pending: u32,
    frame: i32,
    intro_t: i32,
    eye: V3,
    look: V3,
    orbit: i32,
    aspect: i32,
    scene_version: u32,
    static_values: Vec<i32>,
    static_draws: Vec<Vec<u32>>,
    terrain: Vec<i32>,
    dyn_values: Vec<i32>,
    dyn_draws: Vec<Vec<u32>>,
    sounds: Vec<&'static str>,
    says: Vec<(u32, &'static str, usize, Option<i32>)>,
    /// bursts still to come: ticks to wait, kind, where (a firework's shell)
    later: Vec<(i32, &'static str, V3)>,
    say_serial: u32,
    guide: Vec<V3>,
    guide_key: (i32, i32, usize, V3),
    best: BTreeMap<String, i32>,
    best_round: Option<i32>,
    rounds: i32,
    online: Option<Online>,
    /// goodbyes still to send after leaving online play
    parting: Vec<(u32, String)>,
    /// something to tell the person on the title screen (a key in copy.json)
    notice: Option<&'static str>,
}

impl Game {
    pub fn new(frogs: &str, course: &str, palette: &str, saved: &str) -> Result<Game, String> {
        let frogs = parse_frogs(frogs)?;
        let pal = Palette::parse(palette)?;
        let holes = parse_course(course, &pal)?;
        for name in NAMED_COLOURS {
            if !pal.has(name) {
                return Err(format!("palette: no colour {name}"));
            }
        }
        let first = Course::build(&holes[0], &pal);
        let mut g = Game {
            frogs,
            pal,
            holes,
            mode: Mode::Title,
            slots: [(Slot::Human, 0), (Slot::Cpu, 1), (Slot::Cpu, 4), (Slot::Off, 2)],
            choice: 0,
            round: None,
            plan: Vec::new(),
            at: 0,
            course: first,
            particles: Vec::new(),
            prng: Rng::new(2003),
            last_us: None,
            acc: 0,
            prev_bits: 0,
            pending: 0,
            frame: 0,
            intro_t: 0,
            eye: v3(0, int(30), int(40)),
            look: V3::ZERO,
            orbit: 0,
            aspect: ONE * 16 / 9,
            scene_version: 1,
            static_values: Vec::new(),
            static_draws: Vec::new(),
            terrain: Vec::new(),
            dyn_values: Vec::new(),
            dyn_draws: Vec::new(),
            sounds: Vec::new(),
            says: Vec::new(),
            later: Vec::new(),
            say_serial: 0,
            guide: Vec::new(),
            guide_key: (0, 0, 0, V3::ZERO),
            best: BTreeMap::new(),
            best_round: None,
            rounds: 0,
            online: None,
            parting: Vec::new(),
            notice: None,
        };
        g.load_saved(saved);
        g.build_scene();
        g.render();
        Ok(g)
    }

    fn load_saved(&mut self, saved: &str) {
        let Ok(v) = serde_json::from_str::<Value>(saved) else {
            return;
        };
        if let Some(b) = v.get("best").and_then(|b| b.as_object()) {
            for (k, s) in b {
                if let Some(n) = s.as_i64() {
                    self.best.insert(k.clone(), n as i32);
                }
            }
        }
        self.best_round = v.get("best_round").and_then(|n| n.as_i64()).map(|n| n as i32);
        self.rounds = v.get("rounds").and_then(|n| n.as_i64()).unwrap_or(0) as i32;
    }

    pub fn saved(&self) -> String {
        json!({ "best": self.best, "best_round": self.best_round, "rounds": self.rounds }).to_string()
    }

    // ---- the screens --------------------------------------------------------

    pub fn begin(&mut self) {
        if self.mode == Mode::Title {
            self.notice = None;
            self.mode = Mode::Setup;
        }
    }

    /// Human, CPU, off, round again. The first slot is never off.
    pub fn cycle_slot(&mut self, i: usize) {
        if self.mode != Mode::Setup || i >= SLOTS || self.role() == Some(Role::Joiner) {
            return;
        }
        let hosting = self.role() == Some(Role::Host);
        let next = match self.slots[i].0 {
            Slot::Human => Slot::Cpu,
            Slot::Cpu if i == 0 => Slot::Human,
            Slot::Cpu => Slot::Off,
            Slot::Off if hosting => Slot::Open,
            Slot::Off | Slot::Open => Slot::Human,
            // a seat someone has joined into stays theirs
            Slot::Remote(p) => Slot::Remote(p),
        };
        self.slots[i].0 = next;
        if next != Slot::Off {
            // a slot coming back on takes a frog nobody else has
            let f = self.slots[i].1;
            if self.frog_taken(f, i) {
                self.slots[i].1 = self.free_frog(f, 1, i);
            }
        }
        self.lobby_changed();
    }

    fn frog_taken(&self, f: usize, except: usize) -> bool {
        self.slots
            .iter()
            .enumerate()
            .any(|(j, (k, g))| j != except && *k != Slot::Off && *g == f)
    }

    fn free_frog(&self, from: usize, dir: i32, slot: usize) -> usize {
        let n = self.frogs.len() as i32;
        let mut f = from as i32;
        for _ in 0..n {
            f = (f + dir).rem_euclid(n);
            if !self.frog_taken(f as usize, slot) {
                return f as usize;
            }
        }
        from
    }

    /// The next frog nobody else in the round has.
    pub fn cycle_frog(&mut self, i: usize, forward: bool) {
        if let Some(o) = &mut self.online {
            if o.role == Role::Joiner {
                // a joiner may change only its own frog, and the host decides
                if i == o.seat {
                    o.to_host(&Msg::Pick { forward });
                }
                return;
            }
        }
        if self.mode != Mode::Setup || i >= SLOTS || self.slots[i].0 == Slot::Off {
            return;
        }
        self.slots[i].1 = self.free_frog(self.slots[i].1, if forward { 1 } else { -1 }, i);
        self.lobby_changed();
    }

    pub fn cycle_course(&mut self, forward: bool) {
        if self.mode != Mode::Setup || self.role() == Some(Role::Joiner) {
            return;
        }
        let n = self.holes.len() as i32 + 1;
        self.choice = (self.choice as i32 + if forward { 1 } else { -1 }).rem_euclid(n) as usize;
        self.lobby_changed();
    }

    pub fn start_round(&mut self) {
        if self.mode != Mode::Setup || self.role() == Some(Role::Joiner) {
            return;
        }
        let seated: Vec<(Slot, usize)> = self.slots.iter().copied().filter(|(k, _)| k.plays()).collect();
        let players: Vec<(usize, bool)> = seated.iter().map(|(k, f)| (*f, *k == Slot::Cpu)).collect();
        self.plan = if self.choice == 0 {
            (0..self.holes.len()).collect()
        } else {
            vec![self.choice - 1]
        };
        let seed = 2003 + self.rounds as u64;
        self.round = Some(Round::new(players.clone(), seed));
        self.at = 0;
        if let Some(o) = &mut self.online {
            o.owners = seated
                .iter()
                .map(|(k, _)| match k {
                    Slot::Remote(p) => Owner::There(*p),
                    Slot::Cpu => Owner::Cpu,
                    _ => Owner::Here,
                })
                .collect();
            o.tick = 0;
            o.fresh.clear();
            o.sums.clear();
            o.cmds.clear();
            o.desync = None;
            let kinds: Vec<String> = seated.iter().map(|(k, _)| k.name().to_string()).collect();
            let peers: Vec<u32> = o.peers.keys().copied().collect();
            for p in peers {
                let you = seated
                    .iter()
                    .position(|(k, _)| *k == Slot::Remote(p))
                    .unwrap_or(0);
                let m = Msg::Start {
                    players: players.clone(),
                    owners: kinds.clone(),
                    plan: self.plan.clone(),
                    seed,
                    you,
                };
                o.send(p, &m);
            }
        }
        self.load_hole();
    }

    fn load_hole(&mut self) {
        let h = self.plan[self.at];
        self.course = Course::build(&self.holes[h], &self.pal);
        if let Some(r) = &mut self.round {
            r.start_hole(&self.course);
        }
        self.particles.clear();
        self.says.clear();
        self.intro_t = 0;
        self.orbit = 0;
        self.mode = Mode::Intro;
        self.build_scene();
    }

    /// The Tee off button. Online, the host's press is a command every copy
    /// runs at the same tick; a joiner's does nothing.
    pub fn tee_off(&mut self) {
        match self.role() {
            Some(Role::Host) => self.command(Cmd::TeeOff),
            Some(Role::Joiner) => {}
            None => self.do_tee_off(),
        }
    }

    fn command(&mut self, c: Cmd) {
        if let Some(o) = &mut self.online {
            o.cmds.push_back(c);
        }
    }

    fn do_tee_off(&mut self) {
        if self.mode == Mode::Intro {
            self.mode = Mode::Play;
            let notes = self
                .round
                .as_ref()
                .map(|r| vec![Note::Say("turn", r.active)])
                .unwrap_or_default();
            self.apply(notes);
        }
    }

    pub fn next_hole(&mut self) {
        match self.role() {
            Some(Role::Host) => self.command(Cmd::NextHole),
            Some(Role::Joiner) => {}
            None => self.do_next_hole(),
        }
    }

    fn do_next_hole(&mut self) {
        if self.mode != Mode::Card {
            return;
        }
        if self.at + 1 < self.plan.len() {
            self.at += 1;
            self.load_hole();
        } else {
            self.finish_round();
        }
    }

    fn finish_round(&mut self) {
        self.mode = Mode::Final;
        self.rounds += 1;
        if self.plan.len() == self.holes.len() {
            let totals: Vec<i32> = match &self.round {
                Some(r) => (0..r.players.len())
                    .filter(|i| self.is_local(*i))
                    .map(|i| r.players[i].scores.iter().sum())
                    .collect(),
                None => Vec::new(),
            };
            for total in totals {
                if self.best_round.map(|b| total < b).unwrap_or(true) {
                    self.best_round = Some(total);
                }
            }
        }
    }

    pub fn pause(&mut self) {
        // online, the round goes on for everyone else
        if self.mode == Mode::Play && self.online.is_none() {
            self.mode = Mode::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.mode == Mode::Paused {
            self.mode = Mode::Play;
            self.last_us = None;
        }
    }

    pub fn quit(&mut self) {
        self.leave_online();
        self.mode = Mode::Title;
        self.round = None;
        self.course = Course::build(&self.holes[0], &self.pal);
        self.particles.clear();
        self.build_scene();
    }

    pub fn again(&mut self) {
        if self.mode == Mode::Final && self.role() != Some(Role::Joiner) {
            self.mode = Mode::Setup;
            self.round = None;
            self.lobby_changed();
        }
    }

    // ---- online ---------------------------------------------------------------

    fn role(&self) -> Option<Role> {
        self.online.as_ref().map(|o| o.role)
    }

    /// Is player `i` played at this keyboard?
    fn is_local(&self, i: usize) -> bool {
        match &self.online {
            Some(o) => o.owners.get(i) == Some(&Owner::Here),
            None => self.round.as_ref().map(|r| !r.players[i].cpu).unwrap_or(false),
        }
    }

    /// Host a round online from the setup screen. Seats other than the first
    /// that a person would play become open for someone to join into. The
    /// seed (the page's clock) only picks the room code.
    pub fn host_online(&mut self, build: &str, seed: u64) -> String {
        if self.mode != Mode::Setup || self.online.is_some() {
            return self.online.as_ref().map(|o| o.room.clone()).unwrap_or_default();
        }
        let room = online::room_code(seed);
        self.online = Some(Online::new(Role::Host, &room, build));
        for i in 1..SLOTS {
            if self.slots[i].0 == Slot::Human {
                self.slots[i].0 = Slot::Open;
            }
        }
        self.notice = None;
        room
    }

    /// Join a room by its code (or, with an empty code, by a pasted
    /// invitation, which needs no room). False if the code cannot be one.
    pub fn join_online(&mut self, build: &str, code: &str) -> bool {
        let room = if code.is_empty() {
            String::new()
        } else {
            match online::tidy_code(code) {
                Some(c) => c,
                None => return false,
            }
        };
        self.leave_online();
        self.round = None;
        self.online = Some(Online::new(Role::Joiner, &room, build));
        self.mode = Mode::Setup;
        self.notice = None;
        true
    }

    /// Leave online play: goodbye to everyone, and the seats back to people.
    pub fn leave_online(&mut self) {
        let Some(mut o) = self.online.take() else { return };
        let why = if o.role == Role::Host {
            "online_host_left"
        } else {
            "online_left"
        };
        let peers: Vec<u32> = match o.role {
            Role::Host => o.peers.keys().copied().collect(),
            Role::Joiner => o.host.into_iter().collect(),
        };
        for p in peers {
            o.send(p, &Msg::Bye { why: why.into() });
        }
        // the goodbyes still go out
        self.online = None;
        self.parting = o.outbox;
        for slot in self.slots.iter_mut() {
            if matches!(slot.0, Slot::Open | Slot::Remote(_)) {
                slot.0 = Slot::Off;
            }
        }
    }

    /// The room code, while hosting or joining by one.
    pub fn room(&self) -> String {
        self.online.as_ref().map(|o| o.room.clone()).unwrap_or_default()
    }

    /// The transport reached another browser.
    pub fn net_peer(&mut self, peer: u32) {
        let Some(o) = &mut self.online else { return };
        if o.role == Role::Joiner && o.host.is_none() {
            o.host = Some(peer);
            let hello = Msg::Hello {
                proto: online::PROTO,
                build: o.build.clone(),
            };
            o.send(peer, &hello);
        }
    }

    /// The transport lost a browser.
    pub fn net_left(&mut self, peer: u32) {
        let Some(o) = &mut self.online else { return };
        match o.role {
            Role::Joiner => {
                if o.host == Some(peer) {
                    self.online = None;
                    self.notice = Some("online_host_left");
                    self.round = None;
                    self.mode = Mode::Title;
                    self.course = Course::build(&self.holes[0], &self.pal);
                    self.build_scene();
                }
            }
            Role::Host => {
                let Some(seat) = o.peers.remove(&peer) else { return };
                o.holds.remove(&peer);
                o.presses.remove(&peer);
                if self.round.is_some() {
                    // the CPU takes the seat, on every copy at the same tick
                    if let Some(i) = o.owners.iter().position(|w| *w == Owner::There(peer)) {
                        o.cmds.push_back(Cmd::Cpu(i));
                        o.owners[i] = Owner::Cpu;
                    }
                    self.slots[seat].0 = Slot::Cpu;
                } else {
                    self.slots[seat].0 = Slot::Open;
                    self.lobby_changed();
                }
            }
        }
    }

    /// The transport could not do something; a key from copy.json's online words.
    pub fn net_error(&mut self, key: &str) {
        if let (Some(o), Some(k)) = (&mut self.online, ONLINE_WORDS.iter().find(|w| **w == key)) {
            o.status = k;
        }
    }

    /// What to send, as `[[peer, text], ...]`, emptied.
    pub fn net_outbox(&mut self) -> String {
        let mut out = std::mem::take(&mut self.parting);
        if let Some(o) = &mut self.online {
            out.append(&mut o.outbox);
        }
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
    }

    /// A message from another browser.
    pub fn net_message(&mut self, peer: u32, text: &str) {
        let Some(m) = online::decode(text) else { return };
        match self.role() {
            Some(Role::Host) => self.host_hears(peer, m),
            Some(Role::Joiner) => self.joiner_hears(peer, m),
            None => {}
        }
    }

    fn host_hears(&mut self, peer: u32, m: Msg) {
        let started = self.round.is_some();
        let open = self.slots.iter().position(|(k, _)| *k == Slot::Open);
        let Some(o) = &mut self.online else { return };
        match m {
            Msg::Hello { proto, build } => {
                let refuse = if proto != online::PROTO || build != o.build {
                    Some("online_build")
                } else if started {
                    Some("online_started")
                } else if open.is_none() {
                    Some("online_full")
                } else {
                    None
                };
                if let Some(why) = refuse {
                    o.send(peer, &Msg::Bye { why: why.into() });
                    return;
                }
                let seat = open.unwrap_or(0);
                o.peers.insert(peer, seat);
                self.slots[seat].0 = Slot::Remote(peer);
                let f = self.slots[seat].1;
                if self.frog_taken(f, seat) {
                    self.slots[seat].1 = self.free_frog(f, 1, seat);
                }
                self.lobby_changed();
            }
            Msg::Pick { forward } => {
                if let Some(&seat) = o.peers.get(&peer) {
                    if self.mode == Mode::Setup {
                        self.slots[seat].1 =
                            self.free_frog(self.slots[seat].1, if forward { 1 } else { -1 }, seat);
                        self.lobby_changed();
                    }
                }
            }
            Msg::Hold { b, y } => {
                let held = HOLD_ACTIONS.iter().fold(0, |acc, n| acc | (b & bit(n)));
                if o.peers.contains_key(&peer) {
                    o.holds.insert(peer, (held, y));
                }
            }
            Msg::Press { a, n } => {
                if o.peers.contains_key(&peer) && PRESS_ACTIONS.iter().any(|w| bit(w) == a) {
                    o.presses.entry(peer).or_default().push_back((a, n));
                }
            }
            Msg::Sum { tick, sum } => {
                if let Some(mine) = o.sums.get(&tick) {
                    if *mine != sum && o.desync.is_none() {
                        o.desync = Some(tick);
                        let peers: Vec<u32> = o.peers.keys().copied().collect();
                        for p in peers {
                            o.send(p, &Msg::Desync { tick });
                        }
                    }
                }
            }
            Msg::Bye { .. } => self.net_left(peer),
            _ => {}
        }
    }

    fn joiner_hears(&mut self, peer: u32, m: Msg) {
        let Some(o) = &mut self.online else { return };
        if o.host != Some(peer) {
            return;
        }
        match m {
            Msg::Lobby { seats, choice, you } => {
                o.lobby = seats;
                o.choice = choice;
                o.seat = you;
                o.status = "online_in_lobby";
                if self.mode != Mode::Setup {
                    // the host is back in the lobby after a round
                    self.mode = Mode::Setup;
                    self.round = None;
                }
            }
            Msg::Start {
                players,
                owners,
                plan,
                seed,
                you,
            } => {
                o.owners = owners
                    .iter()
                    .enumerate()
                    .map(|(i, k)| match k.as_str() {
                        _ if i == you => Owner::Here,
                        "cpu" => Owner::Cpu,
                        _ => Owner::There(0),
                    })
                    .collect();
                o.inbox.clear();
                o.tick = 0;
                o.desync = None;
                if plan.is_empty() || plan.iter().any(|h| *h >= self.holes.len()) {
                    return;
                }
                self.plan = plan;
                self.round = Some(Round::new(players, seed));
                self.at = 0;
                self.load_hole();
            }
            Msg::Steps { from, s } => {
                let next = o.tick + o.inbox.len() as u32;
                // an ordered channel never skips; drop anything already had
                let skip = next.saturating_sub(from) as usize;
                if from <= next {
                    o.inbox.extend(s.into_iter().skip(skip));
                }
            }
            Msg::Desync { tick } => o.desync = Some(tick),
            Msg::Bye { why } => {
                let key = ONLINE_WORDS
                    .iter()
                    .find(|w| **w == why)
                    .copied()
                    .unwrap_or("online_host_left");
                self.online = None;
                self.notice = Some(key);
                self.round = None;
                self.mode = Mode::Title;
                self.course = Course::build(&self.holes[0], &self.pal);
                self.build_scene();
            }
            _ => {}
        }
    }

    /// Host: every joiner gets the lobby as it now stands.
    fn lobby_changed(&mut self) {
        let seats: Vec<Seat> = self
            .slots
            .iter()
            .map(|(k, f)| Seat {
                kind: k.name().to_string(),
                frog: *f,
            })
            .collect();
        let choice = self.choice;
        let Some(o) = &mut self.online else { return };
        if o.role != Role::Host {
            return;
        }
        let peers: Vec<(u32, usize)> = o.peers.iter().map(|(p, s)| (*p, *s)).collect();
        for (p, you) in peers {
            o.send(
                p,
                &Msg::Lobby {
                    seats: seats.clone(),
                    choice,
                    you,
                },
            );
        }
    }

    // ---- the clock ------------------------------------------------------------

    /// The page's clock in microseconds, the held action bits, the mouse's
    /// movement (CSS pixels ×100).
    pub fn advance(&mut self, now_us: i64, bits: u32, dx: i32, _dy: i32) {
        self.sounds.clear();
        let dt = match self.last_us {
            None => TICK_US,
            Some(last) => (now_us - last).clamp(0, MAX_CATCH_UP_US),
        };
        self.last_us = Some(now_us);
        self.acc += dt;
        self.pending |= bits & !self.prev_bits;
        self.prev_bits = bits;
        if bits & bit("drag") != 0 {
            self.orbit = fx::wrap(self.orbit - dx * DRAG_TURN / 100);
        }
        let mut due = 0;
        while self.acc >= TICK_US {
            self.acc -= TICK_US;
            due += 1;
        }
        let here_yaw = crate::geom::yaw_pitch_of(self.look.sub(self.eye)).0;
        match self.role() {
            None => {
                for _ in 0..due {
                    let e = std::mem::take(&mut self.pending);
                    self.step(Step {
                        b: bits,
                        e,
                        y: here_yaw,
                        n: None,
                        c: None,
                    });
                }
            }
            Some(Role::Host) => {
                for _ in 0..due {
                    let e = std::mem::take(&mut self.pending);
                    let here = Step {
                        b: bits,
                        e,
                        y: here_yaw,
                        n: None,
                        c: None,
                    };
                    self.host_tick(here);
                }
                if let Some(o) = &mut self.online {
                    o.flush_steps();
                }
            }
            Some(Role::Joiner) => {
                let e = std::mem::take(&mut self.pending);
                self.joiner_keys(bits, e, here_yaw);
                let n = self.online.as_ref().map(|o| o.steps_to_run(due)).unwrap_or(0);
                for _ in 0..n {
                    let Some(s) = self.online.as_mut().and_then(|o| o.inbox.pop_front()) else {
                        break;
                    };
                    self.step(s);
                    self.joiner_ran();
                }
                if n == 0 {
                    // nothing from the host yet: the camera and particles still move
                    self.idle_frames(due);
                }
            }
        }
        self.render();
    }

    /// Frames that run no round: the camera, the particles and the clock of
    /// the screens, as a step with nobody's keys would move them.
    fn idle_frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame = (self.frame + 1) % fx::TURN;
            for p in self.particles.iter_mut() {
                p.step();
            }
            self.particles.retain(|p| p.life > 0);
            self.fire_later();
            self.update_camera();
        }
    }

    /// Bursts whose time has come (a firework's shell).
    fn fire_later(&mut self) {
        let mut due = Vec::new();
        for l in self.later.iter_mut() {
            l.0 -= 1;
            if l.0 <= 0 {
                due.push((l.1, l.2));
            }
        }
        self.later.retain(|l| l.0 > 0);
        for (kind, at) in due {
            self.burst(kind, at);
            if !self.sounds.contains(&"bang") {
                self.sounds.push("bang");
            }
        }
    }

    /// Host: one tick of the stream. Before a round, the screens run as offline.
    fn host_tick(&mut self, here: Step) {
        let streaming =
            self.round.is_some() && matches!(self.mode, Mode::Intro | Mode::Play | Mode::Card | Mode::Final);
        if !streaming {
            self.step(Step { c: None, ..here });
            return;
        }
        let owner = match (&self.round, &self.online) {
            (Some(r), Some(o)) if matches!(self.mode, Mode::Intro | Mode::Play) => {
                o.owners.get(r.active).copied()
            }
            _ => None,
        };
        let s = match &mut self.online {
            Some(o) => o.input_for(owner, here),
            None => here,
        };
        self.step(s);
        let sum = self.round.as_ref().map(|r| r.checksum());
        if let Some(o) = &mut self.online {
            o.ran(s, sum);
        }
    }

    /// Joiner: send the host my keys. A press is sent as it happens, with
    /// the needle as my screen showed it; held keys when they change.
    fn joiner_keys(&mut self, bits: u32, edges: u32, yaw: i32) {
        let needle = match &self.round {
            Some(r) if r.phase == Phase::Meter => Some(r.needle),
            _ => None,
        };
        let Some(o) = &mut self.online else { return };
        for name in PRESS_ACTIONS {
            let b = bit(name);
            if edges & b != 0 {
                o.to_host(&Msg::Press {
                    a: b,
                    n: if *name == "swing" { needle } else { None },
                });
            }
        }
        let held = HOLD_ACTIONS.iter().fold(0, |acc, n| acc | (bits & bit(n)));
        o.hold_age += 1;
        if (held, yaw) != o.last_hold || o.hold_age >= online::HOLD_EVERY {
            o.last_hold = (held, yaw);
            o.hold_age = 0;
            o.to_host(&Msg::Hold { b: held, y: yaw });
        }
    }

    /// Joiner: a step from the host has been run; every so often, say what
    /// my copy of the round looks like.
    fn joiner_ran(&mut self) {
        let sum = self.round.as_ref().map(|r| r.checksum());
        let Some(o) = &mut self.online else { return };
        let t = o.tick;
        o.tick += 1;
        if let Some(sum) = sum {
            if t.is_multiple_of(online::SUM_EVERY) {
                o.to_host(&Msg::Sum { tick: t, sum });
            }
        }
    }

    fn step(&mut self, s: Step) {
        let (bits, edges) = (s.b, s.e);
        match s.c {
            Some(Cmd::TeeOff) => self.do_tee_off(),
            Some(Cmd::NextHole) => self.do_next_hole(),
            Some(Cmd::Cpu(i)) => {
                if let Some(r) = &mut self.round {
                    r.hand_to_cpu(i);
                }
                if let Some(o) = &mut self.online {
                    if let Some(w) = o.owners.get_mut(i) {
                        *w = Owner::Cpu;
                    }
                }
            }
            None => {}
        }
        // animation angles are frame × a rate; wrapping at a whole turn keeps them smooth
        self.frame = (self.frame + 1) % fx::TURN;
        for p in self.particles.iter_mut() {
            p.step();
        }
        self.particles.retain(|p| p.life > 0);
        self.fire_later();
        match self.mode {
            Mode::Intro => {
                self.intro_t += 1;
                if edges & bit("swing") != 0 && self.intro_t > secs(400) {
                    self.do_tee_off();
                }
            }
            Mode::Play => {
                let held = |n: &str| bits & bit(n) != 0;
                let edge = |n: &str| edges & bit(n) != 0;
                let input = Input {
                    left: held("left"),
                    right: held("right"),
                    forward: held("forward"),
                    back: held("back"),
                    swing: edge("swing"),
                    tongue: edge("tongue"),
                    loft: edge("loft"),
                    cancel: edge("cancel"),
                    cam_yaw: s.y,
                    needle: s.n,
                };
                let mut notes = Vec::new();
                if let Some(r) = &mut self.round {
                    let was = r.active;
                    r.tick(&self.course, input, &mut notes);
                    if r.active != was {
                        self.orbit = 0;
                    }
                }
                self.apply(notes);
                if self
                    .round
                    .as_ref()
                    .map(|r| r.phase == Phase::Done)
                    .unwrap_or(false)
                {
                    self.hole_done();
                }
            }
            _ => {}
        }
        if self.mode != Mode::Paused {
            self.update_camera();
        }
    }

    fn hole_done(&mut self) {
        let id = self.course.def.id.clone();
        let strokes: Vec<i32> = match &self.round {
            Some(r) => (0..r.players.len())
                .filter(|i| self.is_local(*i))
                .map(|i| r.players[i].strokes)
                .collect(),
            None => Vec::new(),
        };
        for s in strokes {
            if self.best.get(&id).map(|b| s < *b).unwrap_or(true) {
                self.best.insert(id.clone(), s);
            }
        }
        self.mode = Mode::Card;
    }

    fn apply(&mut self, notes: Vec<Note>) {
        for n in notes {
            match n {
                Note::Sound(s) => {
                    if !self.sounds.contains(&s) {
                        self.sounds.push(s);
                    }
                }
                Note::Say(key, who) => {
                    self.say_serial += 1;
                    self.says.push((self.say_serial, key, who, None));
                    if self.says.len() > SAYS_KEPT {
                        self.says.remove(0);
                    }
                }
                Note::SayN(key, who, n) => {
                    self.say_serial += 1;
                    self.says.push((self.say_serial, key, who, Some(n)));
                    if self.says.len() > SAYS_KEPT {
                        self.says.remove(0);
                    }
                }
                Note::Burst(kind, at) => self.burst(kind, at),
            }
        }
    }

    fn burst(&mut self, kind: &'static str, at: V3) {
        if kind == "firework" {
            // a rocket up from the bell, and its shell bursts a moment later
            for k in 0..6 {
                self.particles.push(Particle {
                    pos: at.add(v3(0, cm(40) * k, 0)),
                    vel: v3(0, mps(1400), 0),
                    life: secs(500) - k * 3,
                    max: secs(500),
                    rgb: self.pal.c("spark"),
                    size: cm(26) - k * cm(3),
                    glow: ONE,
                    fall: false,
                });
            }
            let shells = ["spark_a", "spark_b", "spark_c"];
            let shell = shells[self.prng.range(0, 2) as usize];
            self.later.push((secs(520), shell, at.add(v3(0, cm(650), 0))));
            return;
        }
        let c = |n: &str| self.pal.c(n);
        let (n, colours, speed, up, size, glow, fall, life): (
            i32,
            Vec<[i32; 3]>,
            i32,
            i32,
            i32,
            i32,
            bool,
            i32,
        ) = match kind {
            "splash" => (
                16,
                vec![c("splash"), self.course.theme.hazard],
                mps(500),
                mps(900),
                cm(40),
                ONE / 4,
                true,
                secs(900),
            ),
            "lava" => (
                18,
                vec![c("lava"), c("lava_hot"), c("smoke")],
                mps(500),
                mps(900),
                cm(45),
                ONE * 8 / 10,
                true,
                secs(1000),
            ),
            "blast" => (
                22,
                vec![c("blast"), c("blast_hot"), c("smoke")],
                mps(900),
                mps(500),
                cm(110),
                ONE * 7 / 10,
                false,
                secs(700),
            ),
            "dust" => (
                6,
                vec![self.course.theme.ground[1], self.course.theme.rock],
                mps(200),
                mps(150),
                cm(45),
                0,
                false,
                secs(500),
            ),
            "sparkle" => (
                14,
                vec![c("crown"), c("arrow"), c("white")],
                mps(600),
                mps(400),
                cm(22),
                ONE,
                false,
                secs(600),
            ),
            "gulp" => (
                5,
                vec![c("white"), c("guide")],
                mps(300),
                mps(200),
                cm(16),
                ONE,
                false,
                secs(350),
            ),
            "confetti" => (
                36,
                vec![c("lotus"), c("crown"), c("spring"), c("car_blue"), c("white")],
                mps(700),
                mps(1200),
                cm(24),
                ONE / 4,
                true,
                secs(1800),
            ),
            "puff" => (
                8,
                vec![c("white"), c("web")],
                mps(300),
                mps(200),
                cm(55),
                ONE / 5,
                false,
                secs(450),
            ),
            "pop" => (
                10,
                vec![c("bubble"), c("white"), c("crown")],
                mps(400),
                mps(300),
                cm(20),
                ONE,
                false,
                secs(450),
            ),
            "spark_a" | "spark_b" | "spark_c" => (
                26,
                vec![
                    c(match kind {
                        "spark_a" => "firework_a",
                        "spark_b" => "firework_b",
                        _ => "firework_c",
                    }),
                    c("white"),
                ],
                mps(1100),
                mps(200),
                cm(26),
                ONE,
                true,
                secs(1100),
            ),
            "drops" => (
                8,
                vec![c("splash")],
                mps(300),
                mps(500),
                cm(22),
                ONE / 4,
                true,
                secs(600),
            ),
            _ => return,
        };
        for k in 0..n {
            let a = self.prng.range(0, 359);
            let out = facing(deg(a), 0).scale(speed * self.prng.range(40, 100) / 100);
            let rise = up * self.prng.range(50, 100) / 100;
            self.particles.push(Particle {
                pos: at,
                vel: out.add(v3(0, rise, 0)),
                life: life * self.prng.range(70, 100) / 100,
                max: life,
                rgb: colours[k as usize % colours.len()],
                size: size * self.prng.range(70, 130) / 100,
                glow,
                fall,
            });
        }
    }

    // ---- the camera ---------------------------------------------------------

    fn update_camera(&mut self) {
        let c = &self.course;
        let up = |h: i32| v3(0, h, 0);
        let (goal_eye, goal_look, pct) = match (self.mode, &self.round) {
            (Mode::Intro, _) => {
                let k = smooth(ONE * self.intro_t.min(INTRO_TICKS) / INTRO_TICKS);
                let yaw = heading_to(c.tee, c.lotus);
                let f = facing(yaw, 0);
                let eye0 = c
                    .lotus
                    .add(f.scale(int(28)))
                    .add(up(int(16)))
                    .add(crate::geom::right_of(yaw).scale(int(14)));
                let eye1 = c.tee.sub(f.scale(CAM_AIM_BACK)).add(up(CAM_AIM_UP + FROG_RADIUS));
                let lift = fx::mul(fx::sin(fx::mul(k, fx::HALF)), int(26));
                let eye = eye0.lerp(eye1, k).add(up(lift));
                let look = c.lotus.lerp(c.tee.add(f.scale(int(14))), k);
                (eye, look, 100)
            }
            (Mode::Play | Mode::Paused, Some(r)) if r.phase != Phase::Done => {
                let p = &r.players[r.active];
                match r.phase {
                    Phase::Aim | Phase::Meter | Phase::Swing => {
                        let yaw = r.yaw + self.orbit;
                        let f = facing(yaw, 0);
                        let eye = p.lie.sub(f.scale(CAM_AIM_BACK)).add(up(CAM_AIM_UP));
                        let look = p.lie.add(facing(r.yaw, 0).scale(int(14))).add(up(ONE));
                        (eye, look, CAM_AIM_PCT)
                    }
                    Phase::Holed => {
                        let a = self.frame * deg(1) / 2 + self.orbit;
                        (
                            c.lotus.add(facing(a, 0).scale(int(11))).add(up(int(5))),
                            c.lotus.add(up(ONE)),
                            CAM_FOLLOW_PCT,
                        )
                    }
                    _ => {
                        let pos = p.body.pos;
                        let flat = v3(p.body.vel.x, 0, p.body.vel.z);
                        let dir = if flat.len() > ONE / 40 {
                            flat.norm()
                        } else {
                            v3(pos.x - self.eye.x, 0, pos.z - self.eye.z).norm()
                        };
                        let (dyaw, _) = crate::geom::yaw_pitch_of(dir);
                        let f = facing(dyaw + self.orbit, 0);
                        (
                            pos.sub(f.scale(CAM_FLY_BACK)).add(up(CAM_FLY_UP)),
                            pos.add(up(ONE / 2)),
                            CAM_FOLLOW_PCT,
                        )
                    }
                }
            }
            (Mode::Title | Mode::Setup, _) => {
                let mid = c.tee.lerp(c.lotus, ONE / 2);
                let a = self.frame * deg(1) / 8;
                (mid.add(facing(a, 0).scale(int(70))).add(up(int(38))), mid, 100)
            }
            _ => {
                let a = self.frame * deg(1) / 4 + self.orbit;
                (
                    c.lotus.add(facing(a, 0).scale(int(16))).add(up(int(7))),
                    c.lotus.add(up(ONE)),
                    CAM_FOLLOW_PCT,
                )
            }
        };
        let k = ONE * pct / 100;
        self.eye = self.eye.lerp(goal_eye, k);
        self.look = self.look.lerp(goal_look, k);
        let floor = c.surface(self.eye.x, self.eye.z) + cm(150);
        if self.eye.y < floor {
            self.eye.y = floor;
        }
    }

    fn camera(&self) -> Camera {
        let mut fwd = self.look.sub(self.eye).norm();
        if fwd == V3::ZERO {
            fwd = v3(0, 0, -ONE);
        }
        Camera {
            eye: self.eye,
            fwd,
            fov: FOV,
        }
    }

    // ---- what the page draws -----------------------------------------------

    fn build_scene(&mut self) {
        let batch = scenery::build(&self.course, &self.pal);
        let (values, draws) = batch.finish(render::STATIC_BUFFER);
        self.static_values = values;
        self.static_draws = draws;
        self.terrain = self.course.terrain.mesh(&self.course.def, &self.course.theme);
        self.scene_version += 1;
        // the camera starts where the screen wants it, not swinging in
        self.update_camera();
        let saved = (self.eye, self.look);
        for _ in 0..1 {
            self.update_camera();
        }
        let _ = saved;
    }

    fn leader(&self) -> Option<usize> {
        // first place now: the fewest strokes over the holes played and this
        // one so far; nobody leads while two or more are level
        let r = self.round.as_ref()?;
        if r.players.len() < 2 {
            return None;
        }
        let vs: Vec<i32> = r
            .players
            .iter()
            .map(|p| p.scores.iter().sum::<i32>() + p.strokes)
            .collect();
        let best = *vs.iter().min()?;
        let leaders: Vec<usize> = (0..vs.len()).filter(|&i| vs[i] == best).collect();
        if leaders.len() == 1 {
            Some(leaders[0])
        } else {
            None
        }
    }

    fn render(&mut self) {
        // the aiming guide: only for a person aiming, recomputed when the aim changes
        let show_guide = matches!(&self.round, Some(r) if self.mode == Mode::Play
            && matches!(r.phase, Phase::Aim | Phase::Meter)
            && self.is_local(r.active));
        if show_guide {
            let r = self.round.as_ref().unwrap();
            let key = (r.yaw, r.power, r.loft, r.players[r.active].lie);
            if key != self.guide_key || self.guide.is_empty() {
                self.guide = r.guide(&self.course);
                self.guide_key = key;
            }
        } else {
            self.guide.clear();
        }
        let leader = self.leader();
        let mut b = Batch::new(self.course.theme.ink);
        let view = View {
            course: &self.course,
            round: if matches!(self.mode, Mode::Title | Mode::Setup) {
                None
            } else {
                self.round.as_ref()
            },
            pal: &self.pal,
            frogs: &self.frogs,
            tick: self.frame,
            particles: &self.particles,
            guide: &self.guide,
            leader,
        };
        draw::scene(&mut b, &view);
        let (values, draws) = b.finish(render::DYNAMIC_BUFFER);
        self.dyn_values = values;
        self.dyn_draws = draws;
    }

    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_x100: i32) -> [i32; 2] {
        let w = (css_w.max(1) * dpr_x100 / 100).clamp(1, 4096);
        let h = (css_h.max(1) * dpr_x100 / 100).clamp(1, 4096);
        self.aspect = fx::ratio(w, h);
        [w, h]
    }

    pub fn view(&self) -> Vec<i32> {
        let cam = self.camera();
        let mut v = cam.values(self.aspect);
        let focus = match &self.round {
            Some(r) if self.mode == Mode::Play => r.players[r.active].body.pos,
            _ => self.course.lotus,
        };
        let floor = self.course.ground(focus.x, focus.z);
        v.extend([focus.x, focus.y, focus.z, floor]);
        v
    }

    pub fn static_instances(&self) -> Vec<i32> {
        self.static_values.clone()
    }

    pub fn instances(&self) -> Vec<i32> {
        self.dyn_values.clone()
    }

    pub fn terrain_vertices(&self) -> Vec<i32> {
        self.terrain.clone()
    }

    /// One draw per (buffer, mesh, pass), in pass order, the hole's still
    /// scenery before the moving things in each pass.
    pub fn draws(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for pass in 0..render::PASSES {
            if let Some(d) = self.static_draws.get(pass) {
                out.extend_from_slice(d);
            }
            if let Some(d) = self.dyn_draws.get(pass) {
                out.extend_from_slice(d);
            }
        }
        out
    }

    pub fn scene_version(&self) -> u32 {
        self.scene_version
    }

    pub fn scene_uniforms(&self) -> Vec<i32> {
        let t = &self.course.theme;
        let l = v3(ONE * 4 / 10, ONE * 8 / 10, ONE * 3 / 10).norm();
        let mut u = vec![l.x, l.y, l.z];
        u.extend(t.sky);
        u.extend(t.fog);
        u.extend([t.fog_near, t.fog_far]);
        u.extend(t.light);
        u.extend(t.window_dark);
        u.extend(t.ink);
        // the see-through cut: near the lens, wide at the lens, wide by the frog, short of it
        u.extend([ONE, ONE * 6 / 10, ONE * 14 / 10, ONE * 12 / 10]);
        u.push(ONE * 55 / 100);
        u
    }

    pub fn clear_color(&self) -> Vec<i32> {
        self.course.theme.sky.to_vec()
    }

    // ---- the HUD and the screens as JSON ---------------------------------------

    fn frog_json(&self, f: usize) -> Value {
        let d = &self.frogs[f];
        json!({ "id": d.id, "name": d.name, "kind": d.kind, "colour": hex(d.skin), "eye": hex(d.eye) })
    }

    pub fn hud_json(&self) -> String {
        let mut h = json!({
            "mode": self.mode.name(),
            "sounds": self.sounds,
            "notice": self.notice,
            "online": self.online.as_ref().map(|o| json!({
                "role": if o.role == Role::Host { "host" } else { "joiner" },
                "room": o.room,
                "status": o.status,
                "players": o.peers.len(),
                "desync": o.desync,
                "behind": o.inbox.len(),
            })),
        });
        let Some(r) = &self.round else {
            return h.to_string();
        };
        if !matches!(self.mode, Mode::Play | Mode::Paused | Mode::Intro) {
            return h.to_string();
        }
        let c = &self.course;
        let cam = self.camera();
        let p = &r.players[r.active];
        let d = &self.frogs[p.frog];
        let human = self.is_local(r.active);
        let phase = match r.phase {
            Phase::Aim => "aim",
            Phase::Meter => "meter",
            Phase::Swing => "swing",
            Phase::Flight => "flight",
            Phase::Settle => "settle",
            Phase::Penalty => "penalty",
            Phase::Holed => "holed",
            Phase::Done => "done",
        };
        let prompt = match (r.phase, human) {
            (Phase::Aim, true) => "prompt_aim",
            (Phase::Meter, true) => "prompt_meter",
            (Phase::Flight, true) => {
                if p.body.wings > 0 {
                    "prompt_wings"
                } else {
                    "prompt_flight"
                }
            }
            (Phase::Aim | Phase::Meter | Phase::Swing | Phase::Flight, false) => {
                if p.cpu {
                    "prompt_cpu"
                } else {
                    "prompt_online"
                }
            }
            _ => "",
        };
        let tags: Vec<Value> = r
            .players
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let spot = if i == r.active && matches!(r.phase, Phase::Aim | Phase::Meter | Phase::Swing) {
                    q.lie.add(v3(0, cm(30), 0))
                } else {
                    r.standing_spot(i)
                };
                let screen = cam.screen(
                    self.aspect,
                    spot.add(v3(0, cm(if i == r.active { 230 } else { 150 }), 0)),
                );
                let pars: i32 = self.plan[..q.scores.len()]
                    .iter()
                    .map(|&h| self.holes[h].par)
                    .sum();
                json!({
                    "name": self.frogs[q.frog].name,
                    "colour": hex(self.frogs[q.frog].skin),
                    "cpu": q.cpu,
                    "strokes": q.strokes,
                    "holed": q.holed,
                    "active": i == r.active,
                    "flies": q.flies,
                    "points": q.points + q.point_scores.iter().sum::<i32>(),
                    "vs": vs_par(q.scores.iter().sum::<i32>() - pars),
                    "on": screen.is_some() && !q.holed,
                    "x": pct(screen.map(|s| s.0).unwrap_or(0)),
                    "y": pct(screen.map(|s| s.1).unwrap_or(0)),
                })
            })
            .collect();
        let lotus_screen = cam.screen(self.aspect, c.lotus.add(v3(0, int(3), 0)));
        let says: Vec<Value> = self
            .says
            .iter()
            .map(|(id, key, who, n)| {
                let q = &r.players[*who];
                json!({ "id": id, "key": key, "who": self.frogs[q.frog].name, "colour": hex(self.frogs[q.frog].skin), "n": n })
            })
            .collect();
        let meter_on = matches!(r.phase, Phase::Aim | Phase::Meter | Phase::Swing) && self.mode == Mode::Play;
        let needle_deg = if r.phase == Phase::Aim {
            0
        } else {
            r.needle * 360 / fx::TURN
        };
        let hit = r.shot.map(|s| match s.hit {
            crate::round::Hit::Perfect => "hit_perfect",
            crate::round::Hit::Good => "hit_good",
            crate::round::Hit::Whiff => "hit_whiff",
        });
        h["hole"] = json!({
            "n": self.at + 1,
            "of": self.plan.len(),
            "name": c.def.name,
            "par": c.def.par,
        });
        h["phase"] = json!(phase);
        h["prompt"] = json!(prompt);
        h["player"] = json!({
            "name": d.name,
            "colour": hex(d.skin),
            "cpu": p.cpu,
            "strokes": p.strokes,
            "flies": p.flies,
            "points": p.points + p.point_scores.iter().sum::<i32>(),
            "tongue": p.flies >= FLIES_FOR_TONGUE,
            "jumps": p.jumps,
            "wings": if p.body.wings > 0 { json!(tenths(p.body.wings * 10 / TICKS_PER_SECOND)) } else { Value::Null },
            "frog": d.id,
        });
        h["dist"] = json!(c.to_lotus(p.body.pos) / ONE);
        h["power"] = json!(r.power);
        h["loft"] = json!(["loft_low", "loft_mid", "loft_high"][r.loft]);
        h["meter"] = json!({
            "show": meter_on,
            "spinning": r.phase == Phase::Meter,
            "bar": arc_path(BAR_FROM, BAR_TO, 80),
            "perfect": arc_path((BAR_FROM + BAR_TO) / 2 - PERFECT_HALF, (BAR_FROM + BAR_TO) / 2 + PERFECT_HALF, 80),
            "needle": point_on_dial(needle_deg, 90),
            "hit": hit,
        });
        h["players"] = json!(tags);
        h["lotus"] = json!({
            "on": lotus_screen.is_some(),
            "x": pct(lotus_screen.map(|s| s.0).unwrap_or(0)),
            "y": pct(lotus_screen.map(|s| s.1).unwrap_or(0)),
            "dist": c.to_lotus(p.body.pos) / ONE,
        });
        h["says"] = json!(says);
        h.to_string()
    }

    pub fn screen_json(&self) -> String {
        match self.mode {
            Mode::Setup => {
                // a joiner shows the host's lobby, with its own seat marked
                let (seats, choice): (Vec<(String, usize)>, usize) = match &self.online {
                    Some(o) if o.role == Role::Joiner => (
                        o.lobby
                            .iter()
                            .enumerate()
                            .map(|(i, st)| {
                                (
                                    if i == o.seat && st.kind == "remote" {
                                        "you".to_string()
                                    } else {
                                        st.kind.clone()
                                    },
                                    st.frog,
                                )
                            })
                            .collect(),
                        o.choice,
                    ),
                    _ => (
                        self.slots
                            .iter()
                            .map(|(k, f)| (k.name().to_string(), *f))
                            .collect(),
                        self.choice,
                    ),
                };
                let slots: Vec<Value> = seats
                    .iter()
                    .map(
                        |(k, f)| json!({ "kind": k, "frog": self.frog_json((*f).min(self.frogs.len() - 1)) }),
                    )
                    .collect();
                let online = self.online.as_ref().map(|o| {
                    json!({
                        "role": if o.role == Role::Host { "host" } else { "joiner" },
                        "room": o.room,
                        "status": o.status,
                        "players": o.peers.len(),
                        "seat": o.seat,
                        "joined": o.role == Role::Host || !o.lobby.is_empty(),
                    })
                });
                let course = if choice == 0 {
                    json!({ "key": "course_full", "name": "", "par": self.holes.iter().map(|h| h.par).sum::<i32>(), "holes": self.holes.len() })
                } else {
                    let h = &self.holes[(choice - 1).min(self.holes.len() - 1)];
                    json!({ "key": "course_one", "name": h.name, "par": h.par, "holes": 1 })
                };
                json!({ "slots": slots, "course": course, "best_round": self.best_round, "online": online })
                    .to_string()
            }
            Mode::Intro => {
                let c = &self.course;
                let names: Vec<Value> = self
                    .round
                    .as_ref()
                    .map(|r| r.players.iter().map(|p| self.frog_json(p.frog)).collect())
                    .unwrap_or_default();
                json!({
                    "n": self.at + 1,
                    "of": self.plan.len(),
                    "name": c.def.name,
                    "theme": c.def.theme,
                    "par": c.def.par,
                    "length": c.to_lotus(c.tee) / ONE,
                    "best": self.best.get(&c.def.id),
                    "players": names,
                })
                .to_string()
            }
            Mode::Card | Mode::Final => self.card_json(),
            _ => json!({}).to_string(),
        }
    }

    fn card_json(&self) -> String {
        let Some(r) = &self.round else {
            return json!({}).to_string();
        };
        let holes: Vec<Value> = self
            .plan
            .iter()
            .map(|&h| json!({ "name": self.holes[h].name, "par": self.holes[h].par }))
            .collect();
        let played = r.players[0].scores.len();
        let pars: i32 = self.plan[..played].iter().map(|&h| self.holes[h].par).sum();
        let mut rows: Vec<(i32, Value)> = r
            .players
            .iter()
            .map(|p| {
                let total: i32 = p.scores.iter().sum();
                let words: Vec<&str> = p
                    .scores
                    .iter()
                    .enumerate()
                    .map(|(k, s)| crate::round::score_word(*s, self.holes[self.plan[k]].par))
                    .collect();
                (
                    total,
                    json!({
                        "frog": self.frog_json(p.frog),
                        "cpu": p.cpu,
                        "scores": p.scores,
                        "words": words,
                        "total": total,
                        "vs": vs_par(total - pars),
                        "flies": p.flies,
                        "points": p.point_scores.iter().sum::<i32>(),
                    }),
                )
            })
            .collect();
        rows.sort_by_key(|(t, _)| *t);
        let best = rows.first().map(|(t, _)| *t).unwrap_or(0);
        let winners: Vec<Value> = rows
            .iter()
            .filter(|(t, _)| *t == best)
            .map(|(_, v)| v["frog"].clone())
            .collect();
        json!({
            "holes": holes,
            "played": played,
            "rows": rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "final": self.mode == Mode::Final || self.at + 1 >= self.plan.len(),
            "winners": winners,
            "pars": pars,
            "best_round": self.best_round,
        })
        .to_string()
    }

    /// The controls table, for the help screen: keys, then the copy key of what they do.
    pub fn codex_json(&self) -> String {
        json!({
            "frogs": self.frogs.iter().enumerate().map(|(i, _)| self.frog_json(i)).collect::<Vec<_>>(),
            "flies_for_tongue": FLIES_FOR_TONGUE,
            "jumps_max": JUMPS_MAX,
            "wings_seconds": WINGS_TICKS / TICKS_PER_SECOND,
            "holes": self.holes.iter().map(|h| json!({ "name": h.name, "par": h.par, "theme": h.theme })).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

/// Every palette colour the core names, checked at load.
const NAMED_COLOURS: &[&str] = &[
    "ink",
    "white",
    "pupil",
    "glint",
    "mouth",
    "wood",
    "wood_dark",
    "wood_light",
    "mallet",
    "mallet_band",
    "striker",
    "lotus",
    "lotus_light",
    "lotus_centre",
    "lotus_leaf",
    "beacon",
    "pad",
    "pad_dark",
    "pad_flower",
    "web",
    "web_post",
    "fly_body",
    "fly_wing",
    "fly_eye",
    "mushroom_cap",
    "mushroom_spot",
    "mushroom_stem",
    "spark",
    "wings",
    "wing_tip",
    "spring",
    "spring_dark",
    "arrow",
    "bubble",
    "crown",
    "crown_gem",
    "tongue",
    "guide",
    "guide_end",
    "blast",
    "blast_hot",
    "splash",
    "lava",
    "lava_hot",
    "smoke",
    "glow_lamp",
    "trunk",
    "bark_dark",
    "moss",
    "reed",
    "cattail",
    "rock",
    "rock_dark",
    "cactus",
    "cactus_dark",
    "palm_leaf",
    "bone",
    "basalt",
    "basalt_top",
    "volcano",
    "lamp_post",
    "car_blue",
    "tyre",
    "glass",
    "fountain",
    "window",
    "roof_box",
    "awning_a",
    "awning_b",
    "planter",
    "hydrant",
    "bench",
    "sign_a",
    "sign_b",
    "sign_c",
    "pit",
    "bumper_cap",
    "bumper_spot",
    "bumper_stem",
    "bumper_gill",
    "bubble_star",
    "firework_a",
    "firework_b",
    "firework_c",
    "snail_shell",
    "snail_shell_dark",
    "snail_body",
    "antlion",
    "antlion_jaw",
    "whirl",
    "toadstool_cap",
    "glowcap",
    "glowcap_b",
    "puffball",
    "shelf",
    "sand_dark",
    "water_jet",
];

fn smooth(t: i32) -> i32 {
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

/// A Q16 percentage as the page writes it in CSS: two decimals.
fn pct(q: i32) -> String {
    let hundredths = (q as i64 * 100 / ONE as i64) as i32;
    format!(
        "{}{}.{:02}",
        if hundredths < 0 { "-" } else { "" },
        hundredths.abs() / 100,
        hundredths.abs() % 100
    )
}

fn tenths(t: i32) -> String {
    format!("{}.{}", t / 10, t % 10)
}

fn vs_par(d: i32) -> String {
    match d {
        0 => "E".into(),
        d if d > 0 => format!("+{d}"),
        d => format!("{d}"),
    }
}

/// A point on the meter's dial (a 200-unit square, centre 100, 100), at an
/// angle clockwise from twelve and a radius, as SVG coordinates.
fn point_on_dial(deg_: i32, r: i32) -> [String; 2] {
    let a = fx::deg(deg_);
    let x = 100 * ONE + fx::sin(a) * r;
    let y = 100 * ONE - fx::cos(a) * r;
    [one_place(x), one_place(y)]
}

fn one_place(q: i32) -> String {
    let tenths = (q as i64 * 10 / ONE as i64) as i32;
    format!("{}.{}", tenths / 10, tenths.abs() % 10)
}

/// An SVG arc along the dial, clockwise from one angle to another.
fn arc_path(from: i32, to: i32, r: i32) -> String {
    let [x1, y1] = point_on_dial(from, r);
    let [x2, y2] = point_on_dial(to, r);
    let large = if to - from > 180 { 1 } else { 0 };
    format!("M {x1} {y1} A {r} {r} 0 {large} 1 {x2} {y2}")
}

pub fn numbers() -> String {
    let vertex_bytes = mesh::VERTEX_VALUES * render::BYTES_PER_VALUE;
    let instance_bytes = render::INSTANCE_VALUES * render::BYTES_PER_VALUE;
    let vec4 = 4 * render::BYTES_PER_VALUE;
    let v3b = 3 * render::BYTES_PER_VALUE;
    json!({
        "frac_bits": fx::FRAC_BITS,
        "ticks_per_second": TICKS_PER_SECOND,
        "actions": ACTIONS.iter().map(|(n, b)| (n.to_string(), json!(b))).collect::<serde_json::Map<_, _>>(),
        "meshes": Mesh::ALL.iter().map(|m| json!({
            "id": *m as u32,
            "vertices": mesh::vertices(*m).len() / mesh::VERTEX_VALUES,
        })).collect::<Vec<_>>(),
        "terrain_mesh": Mesh::Terrain as u32,
        "vertex": {
            "stride": vertex_bytes,
            "values": mesh::VERTEX_VALUES,
            "attributes": [["aPos", 3, 0], ["aNormal", 3, v3b], ["aTint", 3, 2 * v3b]],
        },
        "instance": {
            "stride": instance_bytes,
            "attributes": [["iA", 4, 0], ["iB", 4, vec4], ["iC", 4, 2 * vec4], ["iD", 4, 3 * vec4]],
        },
        "draw_values": render::DRAW_VALUES,
        "buffers": { "static": render::STATIC_BUFFER, "dynamic": render::DYNAMIC_BUFFER },
        "passes": { "solid": render::PASS_SOLID, "facade": render::PASS_FACADE, "ink": render::PASS_INK, "glow": render::PASS_GLOW },
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uFogRange", 2], ["uLight", 3], ["uWindowDark", 3], ["uInk", 3], ["uSee", 4], ["uSeeSolid", 1]],
        "view": { "matrix": [0, 16], "eye": [16, 19], "focus": [19, 23] },
        "slots": SLOTS,
    })
    .to_string()
}

pub fn mesh_vertices(id: u32) -> Vec<i32> {
    Mesh::ALL
        .iter()
        .find(|m| **m as u32 == id)
        .map(|m| mesh::vertices(*m))
        .unwrap_or_default()
}

/// The core's white noise for the page's sound: `n` samples in [−1, 1] as
/// Q16, from a seed.
pub fn noise(n: u32, seed: u32) -> Vec<i32> {
    let mut r = Rng::new(seed as u64);
    (0..n.min(1 << 20)).map(|_| r.range(-ONE, ONE)).collect()
}

#[allow(dead_code)]
fn unused(_: content::Rgb) {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::content::tests::files;

    pub fn game() -> Game {
        let f = files();
        Game::new(f.frogs, f.course, f.palette, "").unwrap()
    }

    const FRAME_US: i64 = 16_667;

    fn frames(g: &mut Game, n: i32, bits: u32) {
        for _ in 0..n {
            let now = g.last_us.unwrap_or(0) + FRAME_US;
            g.advance(now, bits, 0, 0);
        }
    }

    fn mode(g: &Game) -> String {
        serde_json::from_str::<Value>(&g.hud_json()).unwrap()["mode"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_title_leads_to_setup_then_the_first_hole_then_play() {
        let mut g = game();
        assert_eq!(mode(&g), "title");
        g.begin();
        assert_eq!(g.mode, Mode::Setup);
        g.start_round();
        assert_eq!(g.mode, Mode::Intro);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["name"], "Lily Swamp");
        assert_eq!(
            s["players"].as_array().unwrap().len(),
            3,
            "a human and two CPUs by default"
        );
        frames(&mut g, 40, bit("swing"));
        frames(&mut g, 1, 0);
        frames(&mut g, 1, bit("swing"));
        assert_eq!(g.mode, Mode::Play);
    }

    #[test]
    fn two_slots_never_share_a_frog_and_the_first_is_never_off() {
        let mut g = game();
        g.begin();
        for i in 0..SLOTS {
            for _ in 0..3 {
                g.cycle_slot(i);
            }
            for _ in 0..5 {
                g.cycle_frog(i, true);
            }
        }
        let on: Vec<usize> = g
            .slots
            .iter()
            .filter(|(k, _)| *k != Slot::Off)
            .map(|(_, f)| *f)
            .collect();
        let mut sorted = on.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), on.len(), "{:?}", g.slots);
        for _ in 0..7 {
            g.cycle_slot(0);
            assert_ne!(g.slots[0].0, Slot::Off);
        }
    }

    #[test]
    fn a_held_key_is_one_press_and_the_meter_spins_after_space() {
        let mut g = game();
        g.begin();
        g.start_round();
        g.tee_off();
        frames(&mut g, 5, 0);
        let r = g.round.as_ref().unwrap();
        assert_eq!(r.phase, Phase::Aim);
        frames(&mut g, 1, bit("swing"));
        assert_eq!(g.round.as_ref().unwrap().phase, Phase::Meter);
        // held, not pressed again: the meter keeps spinning
        frames(&mut g, 20, bit("swing"));
        assert_eq!(g.round.as_ref().unwrap().phase, Phase::Meter);
        frames(&mut g, 1, 0);
        frames(&mut g, 1, bit("swing"));
        assert!(matches!(
            g.round.as_ref().unwrap().phase,
            Phase::Swing | Phase::Flight
        ));
    }

    #[test]
    fn the_hud_names_whose_turn_it_is_and_draws_the_meter() {
        let mut g = game();
        g.begin();
        g.start_round();
        g.tee_off();
        frames(&mut g, 2, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["player"]["name"], "Lime");
        assert_eq!(h["hole"]["par"], 5);
        assert_eq!(h["meter"]["show"], true);
        assert!(h["meter"]["bar"].as_str().unwrap().starts_with("M "));
        assert_eq!(h["players"].as_array().unwrap().len(), 3);
        assert!(h["says"].as_array().unwrap().iter().any(|s| s["key"] == "turn"));
    }

    #[test]
    fn the_dial_puts_twelve_at_the_top_and_three_at_the_right() {
        assert_eq!(point_on_dial(0, 80), ["100.0".to_string(), "20.0".to_string()]);
        let [x, y] = point_on_dial(90, 80);
        assert_eq!((x.as_str(), y.as_str()), ("180.0", "100.0"));
        assert!(
            arc_path(105, 255, 80).contains(" 0 0 1 "),
            "a 150° arc is the small one, clockwise"
        );
    }

    #[test]
    fn a_whole_round_of_cpus_reaches_the_final_card_and_keeps_no_cpu_record() {
        let mut g = game();
        g.begin();
        g.slots = [(Slot::Cpu, 0), (Slot::Cpu, 1), (Slot::Off, 2), (Slot::Off, 3)];
        g.start_round();
        let mut guard = 0;
        while g.mode != Mode::Final && guard < 200_000 {
            match g.mode {
                Mode::Intro => g.tee_off(),
                Mode::Card => g.next_hole(),
                _ => {}
            }
            frames(&mut g, 30, 0);
            guard += 30;
        }
        assert_eq!(g.mode, Mode::Final);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["rows"][0]["scores"].as_array().unwrap().len(), 5);
        assert!(!s["winners"].as_array().unwrap().is_empty());
        assert!(
            g.best.is_empty() && g.best_round.is_none(),
            "CPU scores are not records"
        );
        let back = Game::new(files().frogs, files().course, files().palette, &g.saved()).unwrap();
        assert_eq!(back.rounds, 1);
    }

    /// Browsers on a pretend network: each message arrives `lag` frames
    /// after it was sent, in order.
    struct Net {
        games: Vec<Game>,
        ids: Vec<u32>,
        wire: Vec<(usize, usize, u32, String)>,
        lag: usize,
        frame: usize,
    }

    impl Net {
        fn new(joiners: usize, lag: usize) -> Net {
            let mut games: Vec<Game> = (0..=joiners).map(|_| game()).collect();
            games[0].begin();
            for i in 1..SLOTS {
                games[0].slots[i].0 = Slot::Human;
                games[0].slots[i].1 = i;
            }
            let room = games[0].host_online("build", 42);
            for g in games.iter_mut().skip(1) {
                assert!(g.join_online("build", &room));
            }
            // the transport: the host is peer 0 to every joiner; joiner k is peer k to the host
            for k in 1..=joiners {
                games[0].net_peer(k as u32);
                games[k].net_peer(0);
            }
            Net {
                games,
                ids: (0..=joiners as u32).collect(),
                wire: Vec::new(),
                lag,
                frame: 0,
            }
        }

        fn frame(&mut self, bits: &[u32]) {
            self.frame += 1;
            for (k, g) in self.games.iter_mut().enumerate() {
                let now = g.last_us.unwrap_or(0) + 16_667;
                g.advance(now, bits.get(k).copied().unwrap_or(0), 0, 0);
                let out: Vec<(u32, String)> = serde_json::from_str(&g.net_outbox()).unwrap();
                for (to, text) in out {
                    let to_index = if k == 0 { to as usize } else { 0 };
                    self.wire
                        .push((self.frame + self.lag, to_index, self.ids[k], text));
                }
            }
            let now = self.frame;
            let (due, later): (Vec<_>, Vec<_>) = self.wire.drain(..).partition(|(at, ..)| *at <= now);
            self.wire = later;
            for (_, to, from, text) in due {
                // the host knows joiner k as peer k; a joiner knows the host as peer 0
                let from_id = if to == 0 { from } else { 0 };
                self.games[to].net_message(from_id, &text);
            }
        }
    }

    #[test]
    fn four_browsers_play_a_whole_round_online_and_every_copy_agrees() {
        let mut web = Net::new(3, 6);
        for _ in 0..30 {
            web.frame(&[]);
        }
        for k in 1..=3 {
            let s: Value = serde_json::from_str(&web.games[k].screen_json()).unwrap();
            assert_eq!(s["slots"][k]["kind"], "you", "joiner {k} sees its own seat");
        }
        // the fourth seat to a CPU, so CPUs and people mix
        web.games[0].slots[3].0 = Slot::Cpu;
        web.games[0].slots[2].1 = 6;
        web.games[0].start_round();
        let mut compared = 0;
        let mut f = 0;
        while web.games[0].mode != Mode::Final && f < 120_000 {
            f += 1;
            match web.games[0].mode {
                Mode::Intro if f % 50 == 0 => web.games[0].tee_off(),
                Mode::Card if f % 50 == 0 => web.games[0].next_hole(),
                _ => {}
            }
            // everyone taps Space now and then: whoever's turn it is swings
            let tap = if f % 37 == 0 { bit("swing") } else { 0 };
            web.frame(&[tap, tap, tap, tap]);
            // a joiner's copy, at the tick it has reached, agrees with the host's record
            for k in 1..=3 {
                let (t, sum) = {
                    let o = web.games[k].online.as_ref().unwrap();
                    (o.tick, web.games[k].round.as_ref().map(|r| r.checksum()))
                };
                if t > 0 && (t - 1) % online::SUM_EVERY == 0 {
                    if let (Some(sum), Some(mine)) =
                        (sum, web.games[0].online.as_ref().unwrap().sums.get(&(t - 1)))
                    {
                        assert_eq!(sum, *mine, "joiner {k} drifted at tick {}", t - 1);
                        compared += 1;
                    }
                }
            }
        }
        assert_eq!(web.games[0].mode, Mode::Final, "the round finished");
        assert_eq!(web.games[0].online.as_ref().unwrap().desync, None);
        assert!(compared > 20, "checksums compared: {compared}");
        for _ in 0..40 {
            web.frame(&[]);
        }
        let host: Value = serde_json::from_str(&web.games[0].screen_json()).unwrap();
        for k in 1..=3 {
            assert_eq!(
                web.games[k].mode,
                Mode::Final,
                "joiner {k} reached the final card"
            );
            let theirs: Value = serde_json::from_str(&web.games[k].screen_json()).unwrap();
            assert_eq!(
                theirs["rows"], host["rows"],
                "joiner {k}'s scorecard matches the host's"
            );
        }
    }

    #[test]
    fn a_joiner_who_leaves_mid_round_is_played_by_the_cpu_and_the_host_leaving_ends_it() {
        let mut web = Net::new(2, 3);
        for _ in 0..20 {
            web.frame(&[]);
        }
        web.games[0].slots[3].0 = Slot::Off;
        web.games[0].start_round();
        for _ in 0..20 {
            web.frame(&[]);
        }
        web.games[0].tee_off();
        for _ in 0..30 {
            web.frame(&[]);
        }
        web.games[0].net_left(2);
        for _ in 0..30 {
            web.frame(&[]);
        }
        let r = web.games[0].round.as_ref().unwrap();
        assert!(r.players[2].cpu, "the seat is the CPU's now");
        assert!(
            web.games[1].round.as_ref().unwrap().players[2].cpu,
            "on the other joiner's copy too"
        );
        web.games[0].quit();
        for _ in 0..10 {
            web.frame(&[]);
        }
        assert_eq!(web.games[1].mode, Mode::Title);
        assert_eq!(web.games[1].notice, Some("online_host_left"));
    }

    #[test]
    fn a_full_room_and_a_different_build_are_turned_away() {
        let mut g = game();
        g.begin();
        g.slots[1].0 = Slot::Off;
        g.slots[2].0 = Slot::Off;
        g.slots[3].0 = Slot::Off;
        g.host_online("b1", 1);
        g.net_message(
            9,
            &online::encode(&Msg::Hello {
                proto: online::PROTO,
                build: "b2".into(),
            }),
        );
        assert!(g.net_outbox().contains("online_build"));
        g.net_message(
            9,
            &online::encode(&Msg::Hello {
                proto: online::PROTO,
                build: "b1".into(),
            }),
        );
        assert!(g.net_outbox().contains("online_full"), "no open seat");
        g.cycle_slot(1);
        assert_eq!(g.slots[1].0, Slot::Open, "after off comes open while hosting");
        g.net_message(
            9,
            &online::encode(&Msg::Hello {
                proto: online::PROTO,
                build: "b1".into(),
            }),
        );
        assert_eq!(g.slots[1].0, Slot::Remote(9));
        assert!(g.net_outbox().contains("Lobby"));
    }

    #[test]
    fn the_render_contract_lines_up() {
        let g = game();
        let n: Value = serde_json::from_str(&numbers()).unwrap();
        let dv = n["draw_values"].as_u64().unwrap() as usize;
        let draws = g.draws();
        assert_eq!(draws.len() % dv, 0);
        assert!(!g.terrain_vertices().is_empty());
        assert_eq!(g.terrain_vertices().len() % mesh::VERTEX_VALUES, 0);
        assert_eq!(g.view().len(), 23);
        assert_eq!(g.scene_uniforms().len(), 3 + 3 + 3 + 2 + 3 + 3 + 3 + 4 + 1);
        // every draw's instances lie inside its buffer
        for d in draws.chunks(dv) {
            let len = if d[0] == render::STATIC_BUFFER {
                g.static_values.len()
            } else {
                g.dyn_values.len()
            };
            let end = d[2] as usize / render::BYTES_PER_VALUE + d[3] as usize * render::INSTANCE_VALUES;
            assert!(end <= len, "{d:?}");
        }
    }
}
