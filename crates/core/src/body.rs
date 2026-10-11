//! A frog in the air and on the ground: a ball that falls, bounces on the
//! ground, glances off walls, is thrown up by lily pads and spider webs,
//! blown away by mushrooms and bumped by cars, glides on wings, and stops.
//! The same step runs the shot a person flies, the CPU's imagined shots and
//! the aiming guide, so all three agree.

use crate::balance::*;
use crate::content::HazardKind;
use crate::course::{Aabb, Course};
use crate::fx::{self, ONE};
use crate::geom::{facing, v3, yaw_pitch_of, V3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Body {
    pub pos: V3,
    pub vel: V3,
    pub moving: bool,
    pub grounded: bool,
    still: i32,
    pub ticks: i32,
    /// ticks of wings left
    pub wings: i32,
    pub glide_yaw: i32,
    pub glide_pitch: i32,
    /// which mushrooms have gone off this shot (bits)
    pub popped: u32,
    /// small bounces still to come when it next lands: one a fly
    pub hops: i32,
    /// in a whirlpool: ticks of spinning left, the whirlpool, the angle round it
    pub spin: i32,
    pub spin_at: (V3, i32),
    pub spin_angle: i32,
    /// ticks before a whirlpool can take it again
    pub spin_rest: i32,
}

/// What a person (or the CPU) asks of a frog in the air this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Control {
    /// steering along the camera's ground axes, each −ONE..ONE: right, forward
    pub right: i32,
    pub forward: i32,
    /// the camera's heading, which the steering is relative to
    pub cam_yaw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Bounce,
    Pad(usize),
    Web(usize),
    Wall,
    Mushroom(usize),
    Car,
    Landed,
    Bumper(usize),
    Hop,
    Whirl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Flying,
    Stopped,
    /// lava: a stroke, and back where it was hit from
    Hazard(HazardKind),
    /// in the water: it swims where it is, on a lily pad, at this level
    Swim(i32),
    /// an antlion has it: it stays in the pit
    Grabbed(usize),
    Out,
    Lotus,
}

impl Body {
    pub fn resting(at: V3) -> Body {
        Body {
            pos: at,
            vel: V3::ZERO,
            moving: false,
            grounded: true,
            still: 0,
            ticks: 0,
            wings: 0,
            glide_yaw: 0,
            glide_pitch: 0,
            popped: 0,
            hops: 0,
            spin: 0,
            spin_at: (V3::ZERO, 0),
            spin_angle: 0,
            spin_rest: 0,
        }
    }

    /// Thrown from where it sits.
    pub fn launch(&mut self, vel: V3) {
        self.vel = vel;
        self.moving = true;
        self.grounded = false;
        self.still = 0;
        self.ticks = 0;
        self.popped = 0;
        self.hops = 0;
        self.spin = 0;
    }

    pub fn give_wings(&mut self) {
        self.wings = WINGS_TICKS;
        let (yaw, pitch) = yaw_pitch_of(self.vel);
        self.glide_yaw = yaw;
        self.glide_pitch = fx::wrap(pitch).clamp(GLIDE_PITCH_DIVE, GLIDE_PITCH_CLIMB);
    }

    /// The extra jump: up at least JUMP_UP, and a little along the way it faces.
    pub fn jump(&mut self) {
        let flat = v3(self.vel.x, 0, self.vel.z).norm();
        self.vel.y = self.vel.y.max(JUMP_UP);
        self.vel = self.vel.add(flat.scale(JUMP_FORWARD));
        self.grounded = false;
        self.still = 0;
        self.moving = true;
        // the wings stop gliding on a jump; the jump is the jump
        self.wings = 0;
    }

    /// One tick. Events are pushed for the sounds and effects.
    pub fn step(&mut self, c: &Course, ctl: Control, tick: i32, events: &mut Vec<Event>) -> Outcome {
        if !self.moving {
            // a frog sitting still can still be hit by a car
            if self.bumped_by_cars(c, tick, events) {
                self.moving = true;
                self.ticks = 0;
                return Outcome::Flying;
            }
            return Outcome::Stopped;
        }
        self.ticks += 1;
        self.spin_rest = (self.spin_rest - 1).max(0);
        if self.spin > 0 {
            return self.whirl(events);
        }
        if self.wings > 0 {
            self.glide(ctl);
        } else {
            self.vel.y -= GRAVITY;
            if !self.grounded {
                self.steer(ctl);
            }
            self.vel = self.vel.scale(ONE * AIR_KEEP / 10_000);
        }
        // substeps keep a fast frog from passing through a web or a wall
        let speed = self.vel.len();
        let n = (1 + speed / (FROG_RADIUS / 2)).clamp(1, 8);
        let was_grounded = self.grounded;
        self.grounded = false;
        for _ in 0..n {
            let before = self.pos;
            self.pos = self.pos.add(v3(self.vel.x / n, self.vel.y / n, self.vel.z / n));
            self.collide(c, before, tick, events);
            if self.pos.y > c.top_water {
                // above every water there is: nothing to look up
            } else if let Some((k, level)) = c.in_hazard(self.pos) {
                if k == HazardKind::Lava {
                    return Outcome::Hazard(k);
                }
                if self.spin_rest == 0 {
                    let inside = |(w, r): &&(V3, i32)| v3(self.pos.x - w.x, 0, self.pos.z - w.z).len() < *r;
                    if let Some(&(centre, r)) = c.whirlpools.iter().find(inside) {
                        self.spin = WHIRL_TICKS;
                        self.spin_at = (centre, r);
                        self.spin_angle = fx::atan2(self.pos.z - centre.z, self.pos.x - centre.x);
                        self.wings = 0;
                        events.push(Event::Whirl);
                        return Outcome::Flying;
                    }
                }
                self.pos.y = level + FROG_RADIUS / 3;
                return Outcome::Swim(level);
            }
            if c.in_lotus(self.pos) {
                return Outcome::Lotus;
            }
        }
        if c.out_of_bounds(self.pos) {
            return Outcome::Out;
        }
        // an antlion takes a frog that rolls or lands near the bottom of its pit
        for (i, (bottom, r)) in c.antlions.iter().enumerate() {
            let flat = v3(self.pos.x - bottom.x, 0, self.pos.z - bottom.z).len();
            // a frog whacked out of a pit is free until it falls back
            let leaving = self.ticks < ANTLION_LEAVE && self.vel.y > 0;
            if !leaving && flat < r * ANTLION_GRAB_PCT / 100 && self.pos.y < bottom.y + FROG_RADIUS * 4 {
                self.pos = bottom.add(v3(0, FROG_RADIUS, 0));
                self.vel = V3::ZERO;
                self.moving = false;
                return Outcome::Grabbed(i);
            }
        }
        if self.grounded && !was_grounded && self.vel.y.abs() < REST_SPEED * 3 {
            events.push(Event::Landed);
        }
        if self.grounded {
            self.wings = 0;
        } else if self.wings > 0 {
            self.wings -= 1;
        }
        if self.grounded && self.vel.len() < REST_SPEED {
            self.still += 1;
        } else {
            self.still = 0;
        }
        if self.still >= REST_TICKS || self.ticks >= FLIGHT_LIMIT {
            self.moving = false;
            self.vel = V3::ZERO;
            self.wings = 0;
            return Outcome::Stopped;
        }
        Outcome::Flying
    }

    /// Round and round a whirlpool, drawn in toward its eye, then thrown out
    /// along the way it was turning: somewhere new.
    fn whirl(&mut self, events: &mut Vec<Event>) -> Outcome {
        let (centre, r) = self.spin_at;
        self.spin -= 1;
        self.spin_angle = (self.spin_angle + WHIRL_TURN) & (fx::TURN - 1);
        let k = ONE * self.spin / WHIRL_TICKS;
        let radius = fx::mul(r, ONE / 4 + fx::mul(k, ONE * 6 / 10));
        let (cx, sz) = (fx::cos(self.spin_angle), fx::sin(self.spin_angle));
        self.pos = v3(
            centre.x + fx::mul(cx, radius),
            centre.y + FROG_RADIUS / 3,
            centre.z + fx::mul(sz, radius),
        );
        if self.spin == 0 {
            // the tangent, the way it was turning
            let tangent = v3(-sz, 0, cx);
            self.vel = tangent.scale(WHIRL_FLING).add(v3(0, WHIRL_UP, 0));
            self.spin_rest = WHIRL_REST;
            self.grounded = false;
            events.push(Event::Bounce);
        } else {
            self.vel = V3::ZERO;
        }
        Outcome::Flying
    }

    fn steer(&mut self, ctl: Control) {
        if ctl.right == 0 && ctl.forward == 0 {
            return;
        }
        let f = facing(ctl.cam_yaw, 0);
        let r = crate::geom::right_of(ctl.cam_yaw);
        let want = f.scale(ctl.forward).add(r.scale(ctl.right)).clamp_len(ONE);
        self.vel = self.vel.add(want.scale(AIR_ACCEL));
    }

    /// Mario's wing cap: A and D bank round, W dives (and gains speed), S
    /// climbs (and loses it); the frog glides on at the speed it keeps.
    fn glide(&mut self, ctl: Control) {
        self.glide_yaw = fx::wrap(self.glide_yaw - fx::mul(ctl.right, GLIDE_TURN));
        let target = if ctl.forward > 0 {
            GLIDE_PITCH_DIVE
        } else if ctl.forward < 0 {
            GLIDE_PITCH_CLIMB
        } else {
            GLIDE_PITCH_LEVEL
        };
        self.glide_pitch = fx::approach(self.glide_pitch, target, GLIDE_PITCH_RATE);
        let mut speed = self
            .vel
            .len()
            .max(GLIDE_START.min(self.vel.len() + GLIDE_MIN / 8));
        let pull = fx::mul(GRAVITY * GLIDE_GRAVITY_PCT / 100, fx::sin(self.glide_pitch));
        speed = (speed - pull - GLIDE_DRAG).clamp(GLIDE_MIN, GLIDE_MAX);
        self.vel = facing(self.glide_yaw, self.glide_pitch).scale(speed);
        // a little sink, so a level glide still comes down
        self.vel.y -= GRAVITY / 6;
    }

    fn bounce_off(&mut self, n: V3, keep_pct: i32, friction_pct: i32) {
        let vn = self.vel.dot(n);
        if vn < 0 {
            let normal = n.scale(vn);
            let along = self.vel.sub(normal);
            self.vel = along
                .scale(ONE * (100 - friction_pct) / 100)
                .sub(normal.scale(ONE * keep_pct / 100));
        }
    }

    fn collide(&mut self, c: &Course, before: V3, tick: i32, events: &mut Vec<Event>) {
        let r = FROG_RADIUS;
        // lily pads and webs: crossing the top going down
        for (i, b) in c.bouncers.iter().enumerate() {
            let top = b.at.y + r;
            if self.vel.y < 0 && before.y >= top - ONE / 8 && self.pos.y < top {
                let d = v3(self.pos.x - b.at.x, 0, self.pos.z - b.at.z).len();
                if d < b.r {
                    // a bigger web throws harder
                    let (min, keep) = if b.web {
                        (
                            WEB_MIN + fx::mul(b.r - WEB_MEDIUM, WEB_PER_METRE).max(0),
                            WEB_KEEP_PCT,
                        )
                    } else {
                        (PAD_MIN, PAD_KEEP_PCT)
                    };
                    self.pos.y = top;
                    self.vel.y = (-self.vel.y * keep / 100).max(min);
                    self.vel.x += b.kick.0;
                    self.vel.z += b.kick.1;
                    self.wings = 0;
                    events.push(if b.web { Event::Web(i) } else { Event::Pad(i) });
                }
            }
        }
        // mushrooms: a touch sets one off
        for (i, m) in c.mushrooms.iter().enumerate() {
            if self.popped & (1 << i) != 0 {
                continue;
            }
            let centre = m.add(v3(0, cm(60), 0));
            if self.pos.dist(centre) < MUSHROOM_REACH {
                let mut away = v3(self.pos.x - m.x, 0, self.pos.z - m.z).norm();
                if away == V3::ZERO {
                    away = v3(self.vel.x, 0, self.vel.z).norm();
                }
                self.vel = away.scale(MUSHROOM_OUT).add(v3(0, MUSHROOM_UP, 0));
                self.popped |= 1 << i;
                self.wings = 0;
                events.push(Event::Mushroom(i));
            }
        }
        // the ground
        let g = c.ground(self.pos.x, self.pos.z);
        if self.pos.y - r < g {
            let n = c.terrain.normal(self.pos.x, self.pos.z);
            let hard = self.vel.dot(n) < -REST_SPEED * 4;
            self.pos.y = g + r;
            self.bounce_off(n, GROUND_BOUNCE_PCT, GROUND_FRICTION_PCT);
            self.grounded = true;
            if hard {
                events.push(Event::Bounce);
            }
            // a fly caught is a little hop on landing
            if self.hops > 0 && self.vel.y < HOP_UP / 2 {
                self.vel.y = HOP_UP;
                self.hops -= 1;
                events.push(Event::Hop);
            }
        }
        // giant blue mushrooms: a pinball bumper's kick
        for (i, b) in c.bumpers.iter().enumerate() {
            if self.push_off_ball(b.cap, b.r, BUMPER_KEEP_PCT, BUMPER_SPEED) {
                events.push(Event::Bumper(i));
            }
        }
        // snails: their shells block a bounce
        for i in 0..c.def.snails.len() {
            let sn = c.def.snails[i];
            let reach = sn.size + FROG_RADIUS;
            if self.pos.x < sn.a.0.min(sn.b.0) - reach
                || self.pos.x > sn.a.0.max(sn.b.0) + reach
                || self.pos.z < sn.a.1.min(sn.b.1) - reach
                || self.pos.z > sn.a.1.max(sn.b.1) + reach
            {
                continue;
            }
            let (at, _) = c.snail_at(i, tick);
            if self.push_off_ball(at, c.def.snails[i].size / 2, WALL_BOUNCE_PCT, 0) {
                events.push(Event::Wall);
            }
        }
        // walls, logs, roofs
        for b in &c.boxes {
            if self.hit_box(b) {
                events.push(Event::Wall);
            }
        }
        for col in &c.columns {
            self.hit_column(col, events);
        }
        if self.bumped_by_cars(c, tick, events) {
            self.grounded = false;
        }
    }

    /// Pushes the frog off a ball (a bumper's cap, a snail's shell), giving
    /// back `keep_pct` of its speed into it, and at least `kick`. True on a hit.
    fn push_off_ball(&mut self, centre: V3, r: i32, keep_pct: i32, kick: i32) -> bool {
        let d = self.pos.sub(centre);
        let reach = r + FROG_RADIUS;
        if d.x.abs() > reach || d.y.abs() > reach || d.z.abs() > reach {
            return false;
        }
        let len = d.len();
        if len >= reach {
            return false;
        }
        let n = if len == 0 { V3::UP } else { d.norm() };
        self.pos = centre.add(n.scale(reach));
        let vn = self.vel.dot(n);
        let out = (-vn * keep_pct / 100).max(kick);
        self.vel = self.vel.sub(n.scale(vn)).add(n.scale(out));
        self.grounded = false;
        self.wings = 0;
        true
    }

    /// Pushes the ball out of a box; true on a hard knock.
    fn hit_box(&mut self, b: &Aabb) -> bool {
        let r = FROG_RADIUS;
        let cp = v3(
            self.pos.x.clamp(b.min.x, b.max.x),
            self.pos.y.clamp(b.min.y, b.max.y),
            self.pos.z.clamp(b.min.z, b.max.z),
        );
        let d = self.pos.sub(cp);
        let len = d.len();
        if len >= r {
            return false;
        }
        let n = if len == 0 { V3::UP } else { d.norm() };
        self.pos = cp.add(n.scale(r));
        let hard = self.vel.dot(n) < -REST_SPEED * 3;
        if n.y > ONE * 7 / 10 {
            self.bounce_off(n, GROUND_BOUNCE_PCT, GROUND_FRICTION_PCT);
            self.grounded = true;
        } else {
            self.bounce_off(n, WALL_BOUNCE_PCT, 5);
        }
        hard
    }

    fn hit_column(&mut self, col: &crate::course::Column, events: &mut Vec<Event>) {
        let r = FROG_RADIUS;
        if self.pos.y < col.y0 || self.pos.y > col.y1 + r {
            return;
        }
        if (self.pos.x - col.x).abs() > col.r + r || (self.pos.z - col.z).abs() > col.r + r {
            return;
        }
        let d = v3(self.pos.x - col.x, 0, self.pos.z - col.z);
        let len = d.len();
        if len >= col.r + r {
            return;
        }
        if self.pos.y > col.y1 && len < col.r {
            // on top
            self.pos.y = col.y1 + r;
            self.bounce_off(V3::UP, GROUND_BOUNCE_PCT, GROUND_FRICTION_PCT);
            self.grounded = true;
            return;
        }
        let n = if len == 0 { v3(ONE, 0, 0) } else { d.norm() };
        self.pos = v3(col.x, self.pos.y, col.z).add(n.scale(col.r + r));
        if self.vel.dot(n) < -REST_SPEED * 3 {
            events.push(Event::Wall);
        }
        self.bounce_off(n, WALL_BOUNCE_PCT, 5);
    }

    fn bumped_by_cars(&mut self, c: &Course, tick: i32, events: &mut Vec<Event>) -> bool {
        let mut hit = false;
        for car in &c.cars {
            let (b, vx) = c.car_box(car, tick);
            let r = FROG_RADIUS;
            if self.pos.x > b.min.x - r
                && self.pos.x < b.max.x + r
                && self.pos.z > b.min.z - r
                && self.pos.z < b.max.z + r
                && self.pos.y < b.max.y + r
                && self.pos.y > b.min.y - r
            {
                // thrown ahead of the car and up
                let side = if self.pos.x > (b.min.x + b.max.x) / 2 {
                    1
                } else {
                    -1
                };
                let ahead = if vx >= 0 { b.max.x + r } else { b.min.x - r };
                if (side > 0) == (vx >= 0) || self.pos.y > b.max.y {
                    self.pos.x = ahead;
                }
                self.pos.y = self.pos.y.max(b.min.y + r);
                self.vel = v3(vx * 2, CAR_BUMP_UP, self.vel.z / 2);
                self.moving = true;
                self.still = 0;
                events.push(Event::Car);
                hit = true;
            }
        }
        hit
    }
}

/// Where a shot goes without steering, until it first comes down: the
/// points of its path every few ticks, and where it ended.
pub fn imagine(c: &Course, from: V3, vel: V3, tick: i32, limit: i32, every: i32) -> (Vec<V3>, V3, Outcome) {
    let mut b = Body::resting(from);
    b.launch(vel);
    let mut path = Vec::new();
    let mut events = Vec::new();
    let mut outcome = Outcome::Flying;
    for t in 0..limit {
        outcome = b.step(c, Control::default(), tick + t, &mut events);
        if t % every == 0 {
            path.push(b.pos);
        }
        if outcome != Outcome::Flying {
            break;
        }
    }
    (path, b.pos, outcome)
}

/// The velocity a whack gives: the speed at a power (percent ×100 for
/// precision), along a heading and a loft.
pub fn launch_velocity(power_x100: i32, yaw: i32, loft: i32) -> V3 {
    let speed = (LAUNCH_MAX as i64 * power_x100 as i64 / 10_000) as i32;
    facing(yaw, loft).scale(speed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_course, tests::files, Palette};
    use crate::fx::{deg, int};

    fn course(i: usize) -> Course {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        Course::build(&parse_course(f.course, &pal).unwrap()[i], &pal)
    }

    fn flat() -> Course {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        let mut h = parse_course(f.course, &pal).unwrap().remove(1);
        h.shapes.clear();
        h.rough = (0, ONE);
        h.waters.clear();
        h.falls.clear();
        h.tunnels.clear();
        h.bumpers.clear();
        h.antlions.clear();
        h.snails.clear();
        h.whirlpools.clear();
        h.bubbles.clear();
        h.pads.clear();
        h.webs.clear();
        h.mushrooms.clear();
        h.props.clear();
        h.scatter.clear();
        h.lanes.clear();
        Course::build(&h, &pal)
    }

    fn fly(c: &Course, b: &mut Body, ticks: i32) -> (Outcome, Vec<Event>) {
        let mut ev = Vec::new();
        let mut out = Outcome::Flying;
        for t in 0..ticks {
            out = b.step(c, Control::default(), t, &mut ev);
            if out != Outcome::Flying {
                break;
            }
        }
        (out, ev)
    }

    #[test]
    fn a_whack_at_45_degrees_lands_near_v_squared_over_g_and_stops() {
        let c = flat();
        let start = c.tee.add(v3(0, FROG_RADIUS, 0));
        let mut b = Body::resting(start);
        b.launch(launch_velocity(5_000, 0, deg(45)));
        // first touch-down: v = 19.5 m/s, g = 18 m/s², range v²/g = 21.1 m
        let mut ev = Vec::new();
        let mut t = 0;
        while !b.grounded && t < 600 {
            b.step(&c, Control::default(), t, &mut ev);
            t += 1;
        }
        let range = start.dist(v3(b.pos.x, start.y, b.pos.z));
        assert!((range - int(21)).abs() < int(2), "range {}", range / ONE);
        let (out, _) = fly(&c, &mut b, FLIGHT_LIMIT);
        assert_eq!(out, Outcome::Stopped);
        assert!(!b.moving && b.vel == V3::ZERO);
    }

    #[test]
    fn steering_moves_a_frog_sideways_in_the_air() {
        let c = flat();
        let start = c.tee.add(v3(0, FROG_RADIUS, 0));
        let land = |right: i32| {
            let mut b = Body::resting(start);
            b.launch(launch_velocity(6_000, 0, deg(45)));
            let mut ev = Vec::new();
            let ctl = Control {
                right,
                forward: 0,
                cam_yaw: 0,
            };
            for t in 0..400 {
                b.step(&c, ctl, t, &mut ev);
                if b.grounded {
                    break;
                }
            }
            b.pos.x
        };
        assert!(land(ONE) > land(0) + int(3), "right steers toward +x");
        assert!(land(-ONE) < land(0) - int(3));
    }

    #[test]
    fn a_lily_pad_throws_a_falling_frog_back_up() {
        let c = course(0);
        let pad = c.bouncers.iter().find(|b| !b.web).expect("the swamp has pads");
        let mut b = Body::resting(pad.at.add(v3(0, int(6), 0)));
        b.launch(v3(0, -ONE / 10, 0));
        let mut ev = Vec::new();
        for t in 0..120 {
            b.step(&c, Control::default(), t, &mut ev);
            if ev.iter().any(|e| matches!(e, Event::Pad(_))) {
                break;
            }
        }
        assert!(ev.iter().any(|e| matches!(e, Event::Pad(_))), "{ev:?}");
        assert!(b.vel.y >= PAD_MIN, "thrown up at {}", b.vel.y);
    }

    #[test]
    fn a_frog_dropped_in_the_water_swims_at_the_surface() {
        let c = course(0);
        // beside the lotus, in its pond
        let at = c.lotus.add(v3(int(6), int(3), int(4)));
        let mut b = Body::resting(at);
        b.launch(v3(0, -ONE / 10, 0));
        let (out, _) = fly(&c, &mut b, 300);
        assert_eq!(
            out,
            Outcome::Swim(c.lotus.y),
            "the lotus floats on the pond's surface"
        );
        assert_eq!(b.pos.y, c.lotus.y + FROG_RADIUS / 3);
    }

    #[test]
    fn lava_still_costs_a_stroke() {
        let c = course(4);
        let w = c
            .def
            .waters
            .iter()
            .find(|w| w.kind == HazardKind::Lava && matches!(w.region, crate::content::Region::Line { .. }))
            .unwrap();
        let crate::content::Region::Line { a, b: bb, .. } = w.region else {
            unreachable!()
        };
        let mid = v3((a.0 + bb.0) / 2, w.level + int(3), (a.1 + bb.1) / 2);
        let mut b = Body::resting(mid);
        b.launch(v3(0, -ONE / 10, 0));
        assert_eq!(fly(&c, &mut b, 300).0, Outcome::Hazard(HazardKind::Lava));
    }

    #[test]
    fn a_giant_blue_mushroom_throws_a_frog_off_like_a_bumper() {
        let c = course(2);
        let m = c.bumpers[0];
        // falling onto the cap from above, a little off centre
        let mut b = Body::resting(m.cap.add(v3(int(1), m.r + int(4), 0)));
        b.launch(v3(0, -ONE / 6, 0));
        let mut ev = Vec::new();
        for t in 0..60 {
            b.step(&c, Control::default(), t, &mut ev);
            if ev.iter().any(|e| matches!(e, Event::Bumper(_))) {
                break;
            }
        }
        assert!(ev.iter().any(|e| matches!(e, Event::Bumper(0))), "{ev:?}");
        assert!(
            b.vel.len() >= BUMPER_SPEED - ONE / 100,
            "thrown at {}",
            b.vel.len()
        );
        assert!(b.vel.y > 0, "up and away");
    }

    #[test]
    fn an_antlion_grabs_a_frog_that_lands_in_its_pit() {
        let c = course(1);
        let (bottom, _) = c.antlions[0];
        let mut b = Body::resting(bottom.add(v3(ONE / 2, int(6), 0)));
        b.launch(v3(0, -ONE / 10, 0));
        assert_eq!(fly(&c, &mut b, 300).0, Outcome::Grabbed(0));
        assert!(!b.moving);
    }

    #[test]
    fn a_whirlpool_spins_a_frog_round_and_throws_it_somewhere_new() {
        let c = course(0);
        let (centre, _) = c.whirlpools[0];
        let mut b = Body::resting(centre.add(v3(int(2), int(4), 0)));
        b.launch(v3(0, -ONE / 8, 0));
        let mut ev = Vec::new();
        let mut spun = 0;
        let mut out = Outcome::Flying;
        for t in 0..900 {
            out = b.step(&c, Control::default(), t, &mut ev);
            spun += (b.spin > 0) as i32;
            if out != Outcome::Flying {
                break;
            }
        }
        assert!(ev.contains(&Event::Whirl));
        assert!(spun >= WHIRL_TICKS - 2, "it spun for {spun} ticks");
        assert!(
            b.pos.dist(centre) > int(6),
            "and was thrown clear: {}",
            b.pos.dist(centre) / ONE
        );
        assert_ne!(out, Outcome::Flying);
    }

    #[test]
    fn a_snail_s_shell_blocks_a_frog() {
        let c = course(0);
        let (shell, _) = c.snail_at(0, 0);
        let size = c.def.snails[0].size;
        let mut b = Body::resting(shell.add(v3(0, size / 2 + int(3), 0)));
        b.launch(v3(0, -ONE / 8, 0));
        let mut ev = Vec::new();
        for t in 0..40 {
            b.step(&c, Control::default(), t, &mut ev);
        }
        assert!(ev.contains(&Event::Wall), "{ev:?}");
        assert!(
            b.pos.dist(c.snail_at(0, 40).0) >= size / 2,
            "not inside the shell"
        );
    }

    #[test]
    fn each_fly_is_one_small_hop_on_landing() {
        let c = flat();
        let hops = |n: i32| {
            let mut b = Body::resting(c.tee.add(v3(0, FROG_RADIUS, 0)));
            b.launch(launch_velocity(4_000, 0, deg(45)));
            b.hops = n;
            let (_, ev) = fly(&c, &mut b, FLIGHT_LIMIT);
            ev.iter().filter(|e| **e == Event::Hop).count()
        };
        assert_eq!(hops(0), 0);
        assert_eq!(hops(3), 3);
        assert_eq!(hops(7), 7);
    }

    #[test]
    fn a_bigger_web_throws_harder() {
        let c = course(2);
        let throw = |r_want: i32| {
            let w = c
                .bouncers
                .iter()
                .filter(|b| b.web)
                .min_by_key(|b| (b.r - r_want).abs())
                .unwrap();
            let mut b = Body::resting(w.at.add(v3(0, int(3), 0)));
            b.launch(v3(0, -ONE / 20, 0));
            let mut ev = Vec::new();
            for t in 0..120 {
                b.step(&c, Control::default(), t, &mut ev);
                if ev.iter().any(|e| matches!(e, Event::Web(_))) {
                    return b.vel.y;
                }
            }
            panic!("no web bounce");
        };
        assert!(throw(cm(900)) > throw(cm(400)) + mps(300));
    }

    #[test]
    fn a_mushroom_throws_a_frog_up_and_away_once() {
        let c = course(1);
        let m = c.mushrooms[0];
        let mut b = Body::resting(m.add(v3(int(2), int(4), 0)));
        b.launch(v3(-ONE / 8, -ONE / 8, 0));
        let mut ev = Vec::new();
        for t in 0..60 {
            b.step(&c, Control::default(), t, &mut ev);
        }
        let pops = ev.iter().filter(|e| matches!(e, Event::Mushroom(_))).count();
        assert_eq!(pops, 1, "{ev:?}");
        assert!(b.pos.y > m.y + int(5), "thrown up: {}", (b.pos.y - m.y) / ONE);
    }

    #[test]
    fn wings_carry_a_frog_further_than_the_same_shot_without() {
        let c = flat();
        let start = c.tee.add(v3(0, FROG_RADIUS, 0));
        let go = |wings: bool| {
            let mut b = Body::resting(start);
            b.launch(launch_velocity(6_000, 0, deg(42)));
            let mut ev = Vec::new();
            for t in 0..1500 {
                if t == 30 && wings {
                    b.give_wings();
                }
                if b.step(&c, Control::default(), t, &mut ev) != Outcome::Flying {
                    break;
                }
            }
            start.z - b.pos.z
        };
        let (plain, winged) = (go(false), go(true));
        assert!(
            winged > plain + int(25),
            "winged {} plain {}",
            winged / ONE,
            plain / ONE
        );
    }

    #[test]
    fn a_wall_turns_a_frog_back() {
        let c = course(3);
        let b0 = c.buildings[0].0;
        // fly at the building's west face from the west
        let at = v3(b0.min.x - int(3), b0.min.y + int(3), (b0.min.z + b0.max.z) / 2);
        let mut b = Body::resting(at);
        b.launch(v3(int(15) / 60, 0, 0));
        let mut ev = Vec::new();
        for t in 0..40 {
            b.step(&c, Control::default(), t, &mut ev);
        }
        assert!(b.pos.x < b0.min.x, "stays outside the wall");
        assert!(b.vel.x <= 0, "and is going back");
    }

    #[test]
    fn imagining_a_shot_matches_flying_it() {
        let c = course(0);
        let start = c.tee.add(v3(0, FROG_RADIUS, 0));
        let v = launch_velocity(7_000, deg(3), deg(42));
        let (_, end, out) = imagine(&c, start, v, 0, FLIGHT_LIMIT, 4);
        let mut b = Body::resting(start);
        b.launch(v);
        let (out2, _) = fly(&c, &mut b, FLIGHT_LIMIT);
        assert_eq!(out, out2);
        assert_eq!(end, b.pos);
    }
}
