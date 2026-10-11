//! A round of frolf: the players, whose turn it is, the swing and its meter,
//! the shot in the air, the tongue, penalties, holing out and the scores.
//! Everything here is a rule; game.rs wraps it in screens, the camera and
//! the HUD, and draw.rs draws it.

use crate::balance::*;
use crate::body::{imagine, launch_velocity, Body, Control, Event, Outcome};
use crate::content::{HazardKind, ItemKind};
use crate::course::Course;
use crate::fx::{self, deg, ONE, TURN};
use crate::geom::{facing, v3, V3};
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Aim,
    Meter,
    Swing,
    Flight,
    Settle,
    Penalty,
    Holed,
    /// every frog is in: the scorecard
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub frog: usize,
    pub cpu: bool,
    /// strokes on each hole played so far
    pub scores: Vec<i32>,
    pub strokes: i32,
    /// where the frog sits: the centre of its ball
    pub lie: V3,
    pub body: Body,
    pub holed: bool,
    pub flies: i32,
    pub jumps: i32,
    /// which of the hole's flies this frog has caught
    pub caught: Vec<bool>,
    /// which of the hole's point bubbles this frog has popped
    pub popped: Vec<bool>,
    /// points from bubbles: on each hole played, and on this one so far
    pub point_scores: Vec<i32>,
    pub points: i32,
    /// knocked by a tongue: where it was before, to go back to on a splash
    pub knocked_from: Option<V3>,
    /// facing, while sitting
    pub yaw: i32,
    /// ticks since it last landed, for the squash
    pub landed: i32,
}

/// How the whack went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Perfect,
    Good,
    Whiff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    pub hit: Hit,
    /// power as percent ×100
    pub power: i32,
    pub yaw: i32,
    pub loft: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tongue {
    pub from: usize,
    pub to: usize,
    pub t: i32,
}

/// What happened this tick, for sounds, effects and words on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    Sound(&'static str),
    /// a word on screen: its key in data/copy.json, and whose it is
    Say(&'static str, usize),
    /// a burst of particles: kind, where
    Burst(&'static str, V3),
    /// a word on screen with a number in it (points)
    SayN(&'static str, usize, i32),
}

#[derive(Debug, Clone)]
pub struct Round {
    pub players: Vec<Player>,
    pub active: usize,
    pub phase: Phase,
    pub t: i32,
    pub yaw: i32,
    pub power: i32,
    pub loft: usize,
    /// the meter's needle, in angle units clockwise from twelve
    pub needle: i32,
    pub shot: Option<Shot>,
    /// where the catapult stands this turn, and which way
    pub catapult: (V3, i32),
    /// ticks since the mallet struck
    pub since_hit: i32,
    /// power-ups taken this stroke (bits)
    pub taken: u32,
    /// ticks since each pad or web last threw a frog
    pub bounced: Vec<i32>,
    /// ticks since each giant mushroom last threw a frog, and each antlion bit
    pub bumped: Vec<i32>,
    pub bitten: Vec<i32>,
    pub tongue: Option<Tongue>,
    pub tick: i32,
    rng: Rng,
    cpu_plan: Option<(i32, i32, usize, i32)>,
    /// the CPU's search for a shot, a few imagined shots a tick, and the tick it finished
    search: Option<Search>,
    cpu_ready: i32,
    repeat: i32,
    jumped_this_flight: bool,
}

/// The CPU's search in progress: shots to imagine, how far it has got,
/// what each scored, and the safety of the best ones.
#[derive(Debug, Clone)]
struct Search {
    cands: Vec<(i32, i32, usize, i32)>,
    next: usize,
    found: Vec<(i32, i32, i32, usize)>,
    stage: u8,
    safe: Vec<i32>,
}

/// Inputs for one tick of play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    /// edges: pressed this tick
    pub swing: bool,
    pub tongue: bool,
    pub loft: bool,
    pub cancel: bool,
    pub cam_yaw: i32,
    /// online: where the needle stood on the swinging player's own screen
    /// when they pressed, so their timing is judged as they saw it, not
    /// late by the round trip to the host
    pub needle: Option<i32>,
}

impl Round {
    pub fn new(players: Vec<(usize, bool)>, seed: u64) -> Round {
        Round {
            players: players
                .into_iter()
                .map(|(frog, cpu)| Player {
                    frog,
                    cpu,
                    scores: Vec::new(),
                    strokes: 0,
                    lie: V3::ZERO,
                    body: Body::resting(V3::ZERO),
                    holed: false,
                    flies: 0,
                    jumps: 0,
                    caught: Vec::new(),
                    popped: Vec::new(),
                    point_scores: Vec::new(),
                    points: 0,
                    knocked_from: None,
                    yaw: 0,
                    landed: 99,
                })
                .collect(),
            active: 0,
            phase: Phase::Aim,
            t: 0,
            yaw: 0,
            power: 100,
            loft: LOFT_DEFAULT,
            needle: 0,
            shot: None,
            catapult: (V3::ZERO, 0),
            since_hit: 999,
            taken: 0,
            bounced: Vec::new(),
            bumped: Vec::new(),
            bitten: Vec::new(),
            tongue: None,
            tick: 0,
            rng: Rng::new(seed),
            cpu_plan: None,
            search: None,
            cpu_ready: 0,
            repeat: 0,
            jumped_this_flight: false,
        }
    }

    /// Everyone to the tee of a new hole.
    pub fn start_hole(&mut self, c: &Course) {
        let lie = c.tee.add(v3(0, FROG_RADIUS, 0));
        let to_lotus = heading_to(lie, c.lotus);
        for (i, p) in self.players.iter_mut().enumerate() {
            // side by side behind the tee, the one to play on it
            let _ = i;
            p.strokes = 0;
            p.holed = false;
            p.lie = lie;
            p.body = Body::resting(lie);
            p.caught = vec![false; c.flies.len()];
            p.popped = vec![false; c.bubbles.len()];
            p.points = 0;
            p.knocked_from = None;
            p.yaw = to_lotus;
            p.jumps = 0;
        }
        self.bounced = vec![999; c.bouncers.len()];
        self.bumped = vec![999; c.bumpers.len()];
        self.bitten = vec![999; c.antlions.len()];
        self.tick = 0;
        self.tongue = None;
        self.phase = Phase::Aim;
        self.next_turn(c, &mut Vec::new(), true);
    }

    /// Where a waiting frog is drawn: those on the same spot as the active
    /// one stand aside, so they are not drawn inside each other.
    pub fn standing_spot(&self, i: usize) -> V3 {
        let p = &self.players[i];
        if i == self.active || p.body.moving {
            return p.body.pos;
        }
        let mut crowd: i32 = 0;
        for (j, q) in self.players.iter().enumerate() {
            if j < i && !q.holed && q.body.pos.dist(p.body.pos) < ONE {
                crowd += 1;
            }
        }
        let active_here = self.players[self.active].body.pos.dist(p.body.pos) < ONE && self.active != i;
        if crowd == 0 && !active_here {
            return p.body.pos;
        }
        let k = crowd + 1;
        let side = crate::geom::right_of(p.yaw).scale(if k % 2 == 0 { -ONE } else { ONE });
        p.body
            .pos
            .add(side.scale(cm(220) * k.div_euclid(2).max(1)))
            .add(facing(p.yaw, 0).scale(cm(150)))
    }

    fn next_turn(&mut self, c: &Course, notes: &mut Vec<Note>, first: bool) {
        // anyone at the pick-up limit is done
        let par = c.def.par;
        for (i, p) in self.players.iter_mut().enumerate() {
            if !p.holed && p.strokes >= par + PICKUP_OVER_PAR {
                p.holed = true;
                p.strokes = par + PICKUP_OVER_PAR;
                notes.push(Note::Say("picked_up", i));
            }
        }
        let open: Vec<usize> = (0..self.players.len())
            .filter(|&i| !self.players[i].holed)
            .collect();
        if open.is_empty() {
            for p in self.players.iter_mut() {
                p.scores.push(p.strokes);
                p.point_scores.push(p.points);
            }
            self.phase = Phase::Done;
            return;
        }
        // whoever has not teed off goes first, in order; then whoever is
        // farthest from the lotus, as in golf
        let next = open
            .iter()
            .copied()
            .find(|&i| self.players[i].strokes == 0)
            .unwrap_or_else(|| {
                *open
                    .iter()
                    .max_by_key(|&&i| (c.to_lotus(self.players[i].lie), -(i as i32)))
                    .unwrap()
            });
        self.active = next;
        let lie = self.players[next].lie;
        self.yaw = heading_to(lie, c.lotus);
        self.loft = LOFT_DEFAULT;
        self.power = self.suggest_power(c, lie, self.yaw, LOFTS[self.loft]);
        self.catapult = (v3(lie.x, c.ground(lie.x, lie.z), lie.z), self.yaw);
        self.players[next].yaw = self.yaw;
        self.phase = Phase::Aim;
        self.t = 0;
        self.shot = None;
        self.since_hit = 999;
        self.cpu_plan = None;
        self.search = None;
        self.needle = 0;
        if !first || next != 0 {
            notes.push(Note::Sound("turn"));
        }
        notes.push(Note::Say("turn", next));
    }

    /// The power that best reaches the lotus along a line, as a starting
    /// point for a person's aim and the CPU's plan.
    pub fn suggest_power(&self, c: &Course, lie: V3, yaw: i32, loft: i32) -> i32 {
        let mut best = (i32::MAX, POWER_MAX);
        let mut p = POWER_MIN;
        while p <= POWER_MAX {
            let (score, _) = judge(c, lie, p, yaw, loft, self.tick);
            if score < best.0 {
                best = (score, p);
            }
            p += POWER_STEP * 2;
        }
        best.1
    }

    /// The CPU's choice of line, loft and power: the shot whose imagined
    /// end is nearest the lotus, a splash counting as far away.
    /// The CPU's choice of line, loft and power, worked out a few imagined
    /// shots a tick so a turn never stalls the page (CPU_SIMS_PER_TICK).
    ///
    /// The lines tried: straight at the lotus and a little either side, and
    /// toward the far end of every stretch of fairway ahead that is nearer
    /// the lotus, so the CPU follows the routes round cliffs and over
    /// crossings rather than only beelining. Powers go in tens, then the
    /// best few are tried five either side. Of the best eight, it takes the
    /// one that still comes off if the whack is a little weak or crooked.
    /// Returns the plan once the search is done.
    fn cpu_search(&mut self, c: &Course) -> Option<(i32, i32, usize)> {
        let lie = self.players[self.active].lie;
        if self.search.is_none() {
            let straight = heading_to(lie, c.lotus);
            let mut yaws: Vec<(i32, i32)> = [0, -6, 6, -14, 14]
                .iter()
                .map(|d| (straight + deg(*d), d.abs() * ONE / 8))
                .collect();
            // and every way round, a little against, for a frog boxed in by walls
            for k in 0..8 {
                let yaw = straight + deg(45 * k + 22);
                if yaws.iter().all(|(y, _)| fx::wrap(*y - yaw).abs() > deg(4)) {
                    yaws.push((yaw, ONE * 5));
                }
            }
            let here = c.to_lotus(lie);
            for &(ax, az, bx, bz, _) in &c.def.fairways {
                for (x, z) in [(ax, az), (bx, bz)] {
                    let p = v3(x, 0, z);
                    let d = v3(x - lie.x, 0, z - lie.z).len();
                    if d > cm(2500) && d < cm(15000) && c.to_lotus(p) < here - cm(1500) {
                        let yaw = heading_to(lie, p);
                        if yaws.iter().all(|(y, _)| fx::wrap(*y - yaw).abs() > deg(4)) {
                            yaws.push((yaw, ONE * 2));
                        }
                    }
                }
            }
            let mut cands = Vec::new();
            for &(yaw, bend) in &yaws {
                for loft in 0..LOFTS.len() {
                    let mut p = POWER_MIN;
                    while p <= POWER_MAX {
                        cands.push((yaw, p, loft, bend));
                        p += POWER_STEP * 2;
                    }
                }
            }
            self.search = Some(Search {
                cands,
                next: 0,
                found: Vec::new(),
                stage: 0,
                safe: Vec::new(),
            });
        }
        let tick = self.tick;
        let s = self.search.as_mut().unwrap();
        let mut budget = CPU_SIMS_PER_TICK;
        while budget > 0 {
            match s.stage {
                0 | 1 => {
                    if let Some(&(yaw, p, loft, bend)) = s.cands.get(s.next) {
                        let (score, _) = judge(c, lie, p, yaw, LOFTS[loft], tick);
                        s.found.push((score + bend, yaw, p, loft));
                        s.next += 1;
                        budget -= 1;
                    } else if s.stage == 0 {
                        // refine the best four by five either side
                        s.found.sort();
                        let best: Vec<_> = s.found.iter().take(4).copied().collect();
                        for (_, yaw, p, loft) in best {
                            for dp in [-POWER_STEP, POWER_STEP] {
                                let q = p + dp;
                                if (POWER_MIN..=POWER_MAX).contains(&q) {
                                    s.cands.push((yaw, q, loft, 0));
                                }
                            }
                        }
                        s.stage = 1;
                    } else {
                        s.found.sort();
                        s.found.truncate(8);
                        s.stage = 2;
                    }
                }
                _ => {
                    let k = s.safe.len();
                    if k >= s.found.len() {
                        let best = s
                            .found
                            .iter()
                            .zip(s.safe.iter())
                            .min_by_key(|(_, safe)| **safe)
                            .map(|(f, _)| *f)
                            .unwrap_or((0, heading_to(lie, c.lotus), POWER_MAX, LOFT_DEFAULT));
                        self.search = None;
                        return Some((best.1, best.2, best.3));
                    }
                    let (score, yaw, p, loft) = s.found[k];
                    let mut worst = score;
                    for (dp, dyaw) in [(-12, 0), (0, -5), (0, 5)] {
                        let (s2, _) = judge(c, lie, p * (100 + dp) / 100, yaw + deg(dyaw), LOFTS[loft], tick);
                        worst = worst.max(s2);
                    }
                    s.safe.push(score + worst / 2);
                    budget -= 3;
                }
            }
        }
        None
    }

    pub fn tick(&mut self, c: &Course, input: Input, notes: &mut Vec<Note>) {
        self.tick += 1;
        self.t += 1;
        self.since_hit += 1;
        for b in self
            .bounced
            .iter_mut()
            .chain(self.bumped.iter_mut())
            .chain(self.bitten.iter_mut())
        {
            *b = (*b + 1).min(999);
        }
        for p in self.players.iter_mut() {
            p.landed += 1;
        }
        if let Some(tg) = &mut self.tongue {
            tg.t += 1;
            if tg.t > TONGUE_TICKS {
                self.tongue = None;
            }
        }
        let cpu = self.players[self.active].cpu;
        let input = if cpu { self.cpu_input(c) } else { input };
        match self.phase {
            Phase::Aim => self.aim(c, input, notes),
            Phase::Meter => self.meter(input, notes),
            Phase::Swing => {
                if self.t >= SWING_HIT {
                    self.launch(notes);
                }
            }
            Phase::Flight => {
                if input.swing {
                    let p = &mut self.players[self.active];
                    if p.jumps > 0 && p.body.moving {
                        p.jumps -= 1;
                        p.body.jump();
                        notes.push(Note::Sound("jump"));
                        notes.push(Note::Burst("puff", p.body.pos));
                    }
                }
                if input.tongue {
                    self.lash(c, notes);
                }
            }
            Phase::Settle => {
                if self.t >= SETTLE_TICKS && self.all_still() {
                    self.next_turn(c, notes, false);
                }
            }
            Phase::Penalty => {
                if self.t >= PENALTY_TICKS && self.all_still() {
                    let p = &mut self.players[self.active];
                    p.body = Body::resting(p.lie);
                    p.strokes += 1;
                    self.next_turn(c, notes, false);
                }
            }
            Phase::Holed => {
                if self.t >= HOLED_TICKS && self.all_still() {
                    self.next_turn(c, notes, false);
                }
            }
            Phase::Done => {}
        }
        self.physics(c, input, notes);
    }

    /// A seat whose player left an online round is played by the CPU from
    /// here on.
    pub fn hand_to_cpu(&mut self, i: usize) {
        if let Some(p) = self.players.get_mut(i) {
            p.cpu = true;
        }
        if i == self.active && matches!(self.phase, Phase::Meter) {
            self.phase = Phase::Aim;
            self.t = 0;
        }
        self.cpu_plan = None;
    }

    /// A fingerprint of everything the rules hold, for two copies of a round
    /// to compare: if they ever differ, the copies have drifted apart.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: i64| {
            h ^= v as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.tick as i64);
        mix(self.active as i64);
        mix(self.phase as i64);
        mix(self.t as i64);
        mix(self.yaw as i64);
        mix(self.power as i64);
        mix(self.loft as i64);
        mix(self.needle as i64);
        mix(self.taken as i64);
        for p in &self.players {
            for v in [
                p.body.pos.x,
                p.body.pos.y,
                p.body.pos.z,
                p.body.vel.x,
                p.body.vel.y,
                p.body.vel.z,
            ] {
                mix(v as i64);
            }
            for v in [
                p.strokes,
                p.flies,
                p.jumps,
                p.holed as i32,
                p.cpu as i32,
                p.body.wings,
                p.points,
                p.body.hops,
                p.body.spin,
            ] {
                mix(v as i64);
            }
        }
        h
    }

    fn all_still(&self) -> bool {
        self.players.iter().all(|p| p.holed || !p.body.moving)
    }

    fn aim(&mut self, c: &Course, input: Input, notes: &mut Vec<Note>) {
        if input.left {
            self.yaw = fx::wrap(self.yaw + AIM_TURN);
        }
        if input.right {
            self.yaw = fx::wrap(self.yaw - AIM_TURN);
        }
        if input.forward || input.back {
            if self.repeat == 0 {
                let step = if input.forward { POWER_STEP } else { -POWER_STEP };
                self.power = (self.power + step).clamp(POWER_MIN, POWER_MAX);
                notes.push(Note::Sound("click"));
            }
            self.repeat = (self.repeat + 1) % POWER_REPEAT;
        } else {
            self.repeat = 0;
        }
        if input.loft {
            self.loft = (self.loft + 1) % LOFTS.len();
            notes.push(Note::Sound("click"));
        }
        if input.tongue {
            self.lash(c, notes);
        }
        let lie = self.players[self.active].lie;
        self.catapult = (v3(lie.x, c.ground(lie.x, lie.z), lie.z), self.yaw);
        self.players[self.active].yaw = self.yaw;
        if input.swing && self.all_still() {
            self.phase = Phase::Meter;
            self.t = 0;
            self.needle = 0;
            notes.push(Note::Sound("meter"));
        }
    }

    fn meter(&mut self, input: Input, notes: &mut Vec<Note>) {
        self.needle = (self.needle + TURN / METER_LAP) & (TURN - 1);
        if self.t % (METER_LAP / 4) == 0 {
            notes.push(Note::Sound("tick"));
        }
        if input.cancel {
            self.phase = Phase::Aim;
            self.t = 0;
            return;
        }
        if input.swing {
            let at = input.needle.map(|n| n & (TURN - 1)).unwrap_or(self.needle);
            let shot = self.whack(at);
            self.shot = Some(shot);
            self.phase = Phase::Swing;
            self.t = 0;
            notes.push(Note::Sound("windup"));
        }
    }

    /// The meter's verdict on a needle angle.
    pub fn whack(&self, needle: i32) -> Shot {
        let at = needle * 360 / TURN;
        let mid = (BAR_FROM + BAR_TO) / 2;
        let half = (BAR_TO - BAR_FROM) / 2;
        let off = at - mid;
        let (hit, pct_x100, slice) = if off.abs() <= PERFECT_HALF {
            (Hit::Perfect, 100 * (100 + PERFECT_BONUS_PCT), 0)
        } else if off.abs() <= half {
            let q = 100 - (off.abs() - PERFECT_HALF) * 100 / (half - PERFECT_HALF);
            let pct = BAR_EDGE_PCT * 100 + (100 - BAR_EDGE_PCT) * q;
            (Hit::Good, pct, SLICE_MAX * off / half)
        } else {
            (
                Hit::Whiff,
                WHIFF_PCT * 100,
                if off < 0 { -WHIFF_SLICE } else { WHIFF_SLICE },
            )
        };
        Shot {
            hit,
            power: self.power * pct_x100 / 100,
            // early (needle short of the middle) hooks left, late slices right
            yaw: fx::wrap(self.yaw - slice),
            loft: LOFTS[self.loft],
        }
    }

    fn launch(&mut self, notes: &mut Vec<Note>) {
        let shot = self.shot.expect("a swing has a shot");
        let p = &mut self.players[self.active];
        p.strokes += 1;
        p.knocked_from = None;
        p.body = Body::resting(p.lie);
        p.body.launch(launch_velocity(shot.power, shot.yaw, shot.loft));
        self.taken = 0;
        self.since_hit = 0;
        self.jumped_this_flight = false;
        self.phase = Phase::Flight;
        self.t = 0;
        notes.push(Note::Sound("whack"));
        match shot.hit {
            Hit::Perfect => {
                notes.push(Note::Sound("perfect"));
                notes.push(Note::Say("perfect", self.active));
                notes.push(Note::Burst("sparkle", p.lie));
                // a firework from the top of the striker's bell
                notes.push(Note::Burst("firework", crate::draw::bell_top(self.catapult)));
                notes.push(Note::Sound("firework"));
            }
            Hit::Good => notes.push(Note::Sound("boing")),
            Hit::Whiff => {
                notes.push(Note::Sound("whiff"));
                notes.push(Note::Say("whiff", self.active));
            }
        }
    }

    /// The tongue: knocks the nearest other frog within reach away.
    fn lash(&mut self, _c: &Course, notes: &mut Vec<Note>) {
        let me = self.active;
        if self.players[me].flies < FLIES_FOR_TONGUE || self.tongue.is_some() {
            if self.players[me].flies < FLIES_FOR_TONGUE {
                notes.push(Note::Say("need_flies", me));
            }
            return;
        }
        let from = self.players[me].body.pos;
        let target = (0..self.players.len())
            .filter(|&i| i != me && !self.players[i].holed)
            .map(|i| (self.standing_spot(i).dist(from), i))
            .filter(|(d, _)| *d < TONGUE_REACH)
            .min();
        let Some((_, to)) = target else {
            notes.push(Note::Say("no_reach", me));
            notes.push(Note::Sound("whiff"));
            return;
        };
        let spot = self.standing_spot(to);
        let away = v3(spot.x - from.x, 0, spot.z - from.z).norm();
        let away = if away == V3::ZERO {
            facing(self.yaw, 0)
        } else {
            away
        };
        let q = &mut self.players[to];
        if !q.body.moving {
            q.knocked_from = Some(q.lie);
            q.body = Body::resting(spot);
        }
        q.body.launch(away.scale(TONGUE_OUT).add(v3(0, TONGUE_UP, 0)));
        self.players[me].flies -= FLIES_FOR_TONGUE;
        self.tongue = Some(Tongue { from: me, to, t: 0 });
        notes.push(Note::Sound("tongue"));
        notes.push(Note::Say("tongue_hit", to));
    }

    fn physics(&mut self, c: &Course, input: Input, notes: &mut Vec<Note>) {
        let tick = self.tick;
        for i in 0..self.players.len() {
            if self.players[i].holed {
                continue;
            }
            let active = i == self.active;
            let flying_shot = active && self.phase == Phase::Flight;
            let ctl = if flying_shot {
                Control {
                    right: (input.right as i32 - input.left as i32) * ONE,
                    forward: (input.forward as i32 - input.back as i32) * ONE,
                    cam_yaw: input.cam_yaw,
                }
            } else {
                Control::default()
            };
            let was_moving = self.players[i].body.moving;
            let mut events = Vec::new();
            let out = self.players[i].body.step(c, ctl, tick, &mut events);
            for e in events {
                self.event(i, e, notes);
            }
            let p = &mut self.players[i];
            if !was_moving && !p.body.moving {
                continue;
            }
            if flying_shot {
                self.pickups(c, notes);
            }
            let pos = self.players[i].body.pos;
            match out {
                Outcome::Flying => {}
                Outcome::Stopped => {
                    let p = &mut self.players[i];
                    p.lie = p.body.pos;
                    p.knocked_from = None;
                    p.yaw = heading_to(p.lie, c.lotus);
                    if flying_shot {
                        self.phase = Phase::Settle;
                        self.t = 0;
                        notes.push(Note::Say("landed", i));
                    }
                }
                Outcome::Lotus => {
                    let p = &mut self.players[i];
                    p.holed = true;
                    p.body.moving = false;
                    p.lie = c.lotus;
                    notes.push(Note::Sound("lotus"));
                    notes.push(Note::Burst("confetti", c.lotus.add(v3(0, ONE, 0))));
                    notes.push(Note::Say(score_word(p.strokes, c.def.par), i));
                    if active {
                        self.phase = Phase::Holed;
                        self.t = 0;
                    }
                }
                Outcome::Swim(level) => {
                    // no stroke: the frog swims where it fell, and a lily pad
                    // comes up under it for the next shot
                    let p = &mut self.players[i];
                    p.lie = v3(p.body.pos.x, level + FROG_RADIUS / 3, p.body.pos.z);
                    p.body = Body::resting(p.lie);
                    p.knocked_from = None;
                    p.yaw = heading_to(p.lie, c.lotus);
                    notes.push(Note::Sound("splash"));
                    notes.push(Note::Burst("splash", pos));
                    notes.push(Note::Say("swim", i));
                    if flying_shot {
                        self.phase = Phase::Settle;
                        self.t = 0;
                    }
                }
                Outcome::Grabbed(k) => {
                    // the antlion holds the frog: the turn ends there, in the pit
                    self.bitten[k] = 0;
                    let p = &mut self.players[i];
                    p.lie = p.body.pos;
                    p.knocked_from = None;
                    p.yaw = heading_to(p.lie, c.lotus);
                    notes.push(Note::Sound("chomp"));
                    notes.push(Note::Burst("dust", pos));
                    notes.push(Note::Say("antlion", i));
                    if flying_shot {
                        self.phase = Phase::Settle;
                        self.t = 0;
                    }
                }
                Outcome::Hazard(_) | Outcome::Out => {
                    let (sound, say, burst) = match out {
                        Outcome::Hazard(HazardKind::Lava) => ("sizzle", "lava", "lava"),
                        _ => ("whiff", "out", "puff"),
                    };
                    notes.push(Note::Sound(sound));
                    notes.push(Note::Burst(burst, pos));
                    let p = &mut self.players[i];
                    if flying_shot {
                        p.body.moving = false;
                        p.body.vel = V3::ZERO;
                        notes.push(Note::Say(say, i));
                        self.phase = Phase::Penalty;
                        self.t = 0;
                    } else {
                        // knocked in: back where it was, a stroke the worse
                        let back = p.knocked_from.unwrap_or(p.lie);
                        p.lie = back;
                        p.body = Body::resting(back);
                        p.strokes += 1;
                        p.knocked_from = None;
                        notes.push(Note::Say("knocked_in", i));
                    }
                }
            }
        }
    }

    fn event(&mut self, i: usize, e: Event, notes: &mut Vec<Note>) {
        let pos = self.players[i].body.pos;
        match e {
            Event::Pad(k) => {
                self.bounced[k] = 0;
                notes.push(Note::Sound("boing"));
                notes.push(Note::Burst("drops", pos));
            }
            Event::Web(k) => {
                self.bounced[k] = 0;
                notes.push(Note::Sound("web"));
            }
            Event::Mushroom(_) => {
                notes.push(Note::Sound("boom"));
                notes.push(Note::Burst("blast", pos));
            }
            Event::Car => notes.push(Note::Sound("honk")),
            Event::Bumper(k) => {
                self.bumped[k] = 0;
                notes.push(Note::Sound("bumper"));
                notes.push(Note::Burst("sparkle", pos));
            }
            Event::Hop => notes.push(Note::Sound("hop")),
            Event::Whirl => {
                notes.push(Note::Sound("whirl"));
                notes.push(Note::Say("whirlpool", i));
            }
            Event::Wall => notes.push(Note::Sound("bonk")),
            Event::Bounce | Event::Landed => {
                self.players[i].landed = 0;
                notes.push(Note::Sound("land"));
                notes.push(Note::Burst("dust", pos.sub(v3(0, FROG_RADIUS, 0))));
            }
        }
    }

    fn pickups(&mut self, c: &Course, notes: &mut Vec<Note>) {
        let i = self.active;
        let pos = self.players[i].body.pos;
        for (k, f) in c.flies.iter().enumerate() {
            if !self.players[i].caught[k] && pos.dist(*f) < FLY_REACH {
                self.players[i].caught[k] = true;
                self.players[i].flies += 1;
                // and one small hop when it next lands
                self.players[i].body.hops += 1;
                notes.push(Note::Sound("fly"));
                notes.push(Note::Burst("gulp", *f));
                if self.players[i].flies == FLIES_FOR_TONGUE {
                    notes.push(Note::Say("tongue_ready", i));
                    notes.push(Note::Sound("ready"));
                }
            }
        }
        for (k, b) in c.bubbles.iter().enumerate() {
            if !self.players[i].popped[k] && pos.dist(b.at) < BUBBLE_REACH {
                self.players[i].popped[k] = true;
                self.players[i].points += b.points;
                notes.push(Note::Sound("pop"));
                notes.push(Note::Burst("pop", b.at));
                notes.push(Note::SayN("points", i, b.points));
            }
        }
        for (k, (kind, at)) in c.items.iter().enumerate() {
            if self.taken & (1 << k) != 0 || pos.dist(*at) > PICKUP_REACH {
                continue;
            }
            self.taken |= 1 << k;
            let p = &mut self.players[i];
            match kind {
                ItemKind::Jump => {
                    p.jumps = (p.jumps + 1).min(JUMPS_MAX);
                    notes.push(Note::Say("got_jump", i));
                    notes.push(Note::Sound("power"));
                }
                ItemKind::Wings => {
                    p.body.give_wings();
                    notes.push(Note::Say("got_wings", i));
                    notes.push(Note::Sound("wings"));
                }
            }
            notes.push(Note::Burst("sparkle", *at));
        }
    }

    /// What the CPU presses this tick.
    fn cpu_input(&mut self, c: &Course) -> Input {
        let mut input = Input::default();
        let me = self.active;
        match self.phase {
            Phase::Aim => {
                if self.cpu_plan.is_none() {
                    let Some((yaw, power, loft)) = self.cpu_search(c) else {
                        return input;
                    };
                    // a miss of up to CPU_MISS degrees either way, nearer the middle more often
                    let miss =
                        (self.rng.range(-CPU_MISS, CPU_MISS) + self.rng.range(-CPU_MISS, CPU_MISS)) / 2;
                    self.cpu_plan = Some((yaw, power, loft, miss));
                    self.cpu_ready = self.t;
                    self.power = power;
                    self.loft = loft;
                }
                if let Some((yaw, _, _, _)) = self.cpu_plan {
                    let d = fx::wrap(yaw - self.yaw);
                    if d > AIM_TURN {
                        input.left = true;
                    } else if d < -AIM_TURN {
                        input.right = true;
                    } else {
                        self.yaw = yaw;
                    }
                    // a frog within reach and flies to spare: the tongue
                    if self.players[me].flies >= FLIES_FOR_TONGUE
                        && self.t == self.cpu_ready + 1
                        && self.someone_in_reach()
                    {
                        input.tongue = true;
                    }
                    if self.t >= (self.cpu_ready + CPU_SETTLE).max(CPU_THINK) && d.abs() <= AIM_TURN {
                        input.swing = true;
                    }
                }
            }
            Phase::Meter => {
                if let Some((_, _, _, miss)) = self.cpu_plan {
                    let mid = (BAR_FROM + BAR_TO) / 2 + miss;
                    let at = self.needle * 360 / TURN;
                    if at >= mid {
                        input.swing = true;
                    }
                }
            }
            Phase::Flight => {
                let p = &self.players[me];
                let to = v3(c.lotus.x - p.body.pos.x, 0, c.lotus.z - p.body.pos.z);
                let heading = crate::geom::yaw_pitch_of(p.body.vel).0;
                input.cam_yaw = heading;
                // steer for the lotus once near it, and turn a wing glide toward it
                let want = crate::geom::yaw_pitch_of(to).0;
                let turn = fx::wrap(want - heading);
                if to.len() < cm(4500) || p.body.wings > 0 {
                    input.left = turn > deg(4);
                    input.right = turn < -deg(4);
                }
                if p.body.wings > 0 && to.len() < cm(2500) {
                    input.forward = true;
                }
                if p.jumps > 0
                    && !self.jumped_this_flight
                    && p.body.vel.y < 0
                    && p.body.vel.y > -ONE / 20
                    && to.len() > cm(2000)
                {
                    input.swing = true;
                    self.jumped_this_flight = true;
                }
                if p.flies >= FLIES_FOR_TONGUE && self.tongue.is_none() && self.someone_in_reach() {
                    input.tongue = true;
                }
            }
            _ => {}
        }
        input
    }

    fn someone_in_reach(&self) -> bool {
        let from = self.players[self.active].body.pos;
        (0..self.players.len()).any(|i| {
            i != self.active && !self.players[i].holed && self.standing_spot(i).dist(from) < TONGUE_REACH
        })
    }

    /// The arc the aim would fly, until it first comes down (the aiming guide).
    pub fn guide(&self, c: &Course) -> Vec<V3> {
        let p = &self.players[self.active];
        let v = launch_velocity(self.power * 100, self.yaw, LOFTS[self.loft]);
        let mut b = Body::resting(p.lie);
        b.launch(v);
        let mut path = Vec::new();
        let mut events = Vec::new();
        for t in 0..secs(6_000) {
            let out = b.step(c, Control::default(), self.tick + t, &mut events);
            if t % 3 == 0 {
                path.push(b.pos);
            }
            if out != Outcome::Flying || b.grounded || !events.is_empty() {
                path.push(b.pos);
                break;
            }
        }
        path
    }
}

/// How good an imagined shot is: its end's distance to the lotus, with a
/// splash or out of bounds counted as a long way off. Lower is better.
fn judge(c: &Course, lie: V3, power: i32, yaw: i32, loft: i32, tick: i32) -> (i32, V3) {
    let (_, end, out) = imagine(
        c,
        lie,
        launch_velocity(power * 100, yaw, loft),
        tick,
        secs(14_000),
        1000,
    );
    let score = match out {
        Outcome::Lotus => 0,
        Outcome::Hazard(_) | Outcome::Out => c.to_lotus(lie) + cm(30_000),
        // swimming costs nothing but a shot from the water; an antlion ends the turn in a pit
        Outcome::Swim(_) => c.to_lotus(end) + cm(800),
        Outcome::Grabbed(_) => c.to_lotus(end) + cm(1500),
        _ => c.to_lotus(end),
    };
    (score, end)
}

pub fn heading_to(from: V3, to: V3) -> i32 {
    crate::geom::yaw_pitch_of(v3(to.x - from.x, 0, to.z - from.z)).0
}

/// The word for a hole's score against its par.
pub fn score_word(strokes: i32, par: i32) -> &'static str {
    if strokes == 1 {
        return "hole_in_one";
    }
    match strokes - par {
        i32::MIN..=-3 => "albatross",
        -2 => "eagle",
        -1 => "birdie",
        0 => "par",
        1 => "bogey",
        2 => "double_bogey",
        _ => "over_par",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_course, tests::files, Palette};
    use crate::fx::int;

    fn course(i: usize) -> Course {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        Course::build(&parse_course(f.course, &pal).unwrap()[i], &pal)
    }

    fn needle_at(deg_: i32) -> i32 {
        deg_ * TURN / 360 + 1
    }

    #[test]
    fn the_middle_of_the_bar_is_perfect_its_ends_weaker_and_outside_a_whiff() {
        let mut r = Round::new(vec![(0, false)], 1);
        r.power = 80;
        let mid = (BAR_FROM + BAR_TO) / 2;
        let perfect = r.whack(needle_at(mid));
        assert_eq!(perfect.hit, Hit::Perfect);
        assert_eq!(perfect.power, 80 * (100 + PERFECT_BONUS_PCT));
        assert_eq!(perfect.yaw, r.yaw, "a perfect shot flies straight");
        let edge = r.whack(needle_at(BAR_TO - 1));
        assert_eq!(edge.hit, Hit::Good);
        assert!(edge.power < perfect.power && edge.power >= 80 * BAR_EDGE_PCT);
        let early = r.whack(needle_at(BAR_FROM + 5));
        assert!(fx::wrap(early.yaw - r.yaw) > 0, "early hooks left (positive yaw)");
        assert!(fx::wrap(edge.yaw - r.yaw) < 0, "late slices right");
        let whiff = r.whack(needle_at(20));
        assert_eq!(whiff.hit, Hit::Whiff);
        assert_eq!(whiff.power, 80 * WHIFF_PCT);
    }

    #[test]
    fn players_tee_off_in_order_then_the_farthest_plays() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false), (1, false), (2, false)], 1);
        r.start_hole(&c);
        assert_eq!(r.active, 0);
        let mut notes = Vec::new();
        // fake three tee shots of different lengths
        for (i, dist) in [(0, 30), (1, 60), (2, 45)] {
            r.players[i].strokes = 1;
            r.players[i].lie = c.tee.add(v3(0, FROG_RADIUS, -cm(dist * 100)));
        }
        r.next_turn(&c, &mut notes, false);
        assert_eq!(r.active, 0, "the shortest drive is farthest from the lotus");
    }

    #[test]
    fn a_frog_in_the_water_swims_on_with_no_stroke_and_plays_from_a_lily_pad() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false)], 1);
        r.start_hole(&c);
        let mut notes = Vec::new();
        // a shot already in the air, coming down into the lotus pond
        let p = &mut r.players[0];
        p.strokes = 1;
        p.body = Body::resting(c.lotus.add(v3(int(7), int(5), int(5))));
        p.body.launch(v3(0, -ONE / 10, 0));
        r.phase = Phase::Flight;
        for _ in 0..secs(6_000) {
            r.tick(&c, Input::default(), &mut notes);
            if r.phase == Phase::Aim {
                break;
            }
        }
        assert!(notes.contains(&Note::Say("swim", 0)), "{notes:?}");
        assert_eq!(r.players[0].strokes, 1, "no stroke for the water");
        assert_eq!(
            r.players[0].lie.y,
            c.lotus.y + FROG_RADIUS / 3,
            "it plays from the water's surface"
        );
        assert_eq!(r.phase, Phase::Aim);
    }

    #[test]
    fn a_bubble_scores_points_once_apart_from_the_strokes() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false)], 1);
        r.start_hole(&c);
        let mut notes = Vec::new();
        let b = c.bubbles[0];
        for _ in 0..2 {
            let p = &mut r.players[0];
            p.body = Body::resting(b.at.add(v3(0, int(2), 0)));
            p.body.launch(v3(0, -ONE / 10, 0));
            r.phase = Phase::Flight;
            for _ in 0..30 {
                r.tick(&c, Input::default(), &mut notes);
            }
        }
        assert_eq!(r.players[0].points, b.points, "popped once");
        assert_eq!(r.players[0].strokes, 0);
        assert!(notes.contains(&Note::SayN("points", 0, b.points)));
    }

    #[test]
    fn a_perfect_whack_sets_off_a_firework_from_the_bell() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false)], 1);
        r.start_hole(&c);
        let mut notes = Vec::new();
        r.shot = Some(Shot {
            hit: Hit::Perfect,
            power: 5_000,
            yaw: r.yaw,
            loft: LOFTS[1],
        });
        r.phase = Phase::Swing;
        r.t = SWING_HIT;
        r.tick(&c, Input::default(), &mut notes);
        let bell = crate::draw::bell_top(r.catapult);
        assert!(notes.contains(&Note::Burst("firework", bell)), "{notes:?}");
        // and a good hit does not
        let mut r2 = Round::new(vec![(0, false)], 1);
        r2.start_hole(&c);
        let mut notes2 = Vec::new();
        r2.shot = Some(Shot {
            hit: Hit::Good,
            power: 5_000,
            yaw: r2.yaw,
            loft: LOFTS[1],
        });
        r2.phase = Phase::Swing;
        r2.t = SWING_HIT;
        r2.tick(&c, Input::default(), &mut notes2);
        assert!(!notes2.iter().any(|n| matches!(n, Note::Burst("firework", _))));
    }

    #[test]
    fn each_fly_caught_adds_a_hop_to_the_next_landing() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false)], 1);
        r.start_hole(&c);
        let mut notes = Vec::new();
        let fly = c.flies[0];
        let p = &mut r.players[0];
        p.body = Body::resting(fly.add(v3(0, ONE, 0)));
        p.body.launch(v3(0, -ONE / 20, 0));
        r.phase = Phase::Flight;
        r.tick(&c, Input::default(), &mut notes);
        assert_eq!(r.players[0].flies, 1);
        assert_eq!(r.players[0].body.hops, 1);
    }

    #[test]
    fn a_cpu_round_finishes_every_hole() {
        for i in 0..5 {
            let c = course(i);
            let mut r = Round::new(vec![(0, true), (1, true)], 7 + i as u64);
            r.start_hole(&c);
            let mut notes = Vec::new();
            let mut ticks = 0;
            while r.phase != Phase::Done && ticks < secs(600_000) {
                r.tick(&c, Input::default(), &mut notes);
                ticks += 1;
            }
            assert_eq!(r.phase, Phase::Done, "hole {} unfinished", c.def.id);
            for p in &r.players {
                let s = p.scores[0];
                assert!(s >= 1 && s <= c.def.par + PICKUP_OVER_PAR, "{}: {s}", c.def.id);
            }
        }
    }

    #[test]
    fn ten_flies_buy_one_tongue_that_knocks_a_frog_away() {
        let c = course(0);
        let mut r = Round::new(vec![(0, false), (1, false)], 1);
        r.start_hole(&c);
        let mut notes = Vec::new();
        r.players[0].flies = FLIES_FOR_TONGUE - 1;
        r.tick(
            &c,
            Input {
                tongue: true,
                ..Input::default()
            },
            &mut notes,
        );
        assert!(r.tongue.is_none(), "nine flies are not enough");
        r.players[0].flies = FLIES_FOR_TONGUE;
        let before = r.standing_spot(1);
        r.tick(
            &c,
            Input {
                tongue: true,
                ..Input::default()
            },
            &mut notes,
        );
        assert!(r.tongue.is_some());
        assert_eq!(r.players[0].flies, 0);
        assert!(r.players[1].body.moving);
        for _ in 0..secs(8_000) {
            r.tick(&c, Input::default(), &mut notes);
        }
        assert!(r.players[1].lie.dist(before) > cm(400), "knocked well away");
    }

    #[test]
    fn a_good_drive_down_the_line_catches_flies_on_every_hole() {
        for i in 0..5 {
            let c = course(i);
            let mut r = Round::new(vec![(0, false)], 1);
            r.start_hole(&c);
            // along the first line of flies, at the power they were strung for
            let (from, to, power, _) = c.def.shot_flies[0];
            let yaw = heading_to(v3(from.0, 0, from.1), v3(to.0, 0, to.1));
            r.shot = Some(Shot {
                hit: Hit::Good,
                power: power * 100,
                yaw,
                loft: LOFTS[LOFT_DEFAULT],
            });
            r.phase = Phase::Swing;
            r.t = SWING_HIT;
            let mut notes = Vec::new();
            for _ in 0..secs(4_000) {
                r.tick(&c, Input::default(), &mut notes);
            }
            assert!(
                r.players[0].flies >= 4,
                "{}: {} flies",
                c.def.id,
                r.players[0].flies
            );
        }
    }

    #[test]
    fn score_words_follow_golf() {
        assert_eq!(score_word(1, 3), "hole_in_one");
        assert_eq!(score_word(2, 4), "eagle");
        assert_eq!(score_word(3, 4), "birdie");
        assert_eq!(score_word(4, 4), "par");
        assert_eq!(score_word(5, 4), "bogey");
        assert_eq!(score_word(9, 4), "over_par");
    }
}
