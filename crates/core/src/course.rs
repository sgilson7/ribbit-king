//! A hole as it is played: the ground, everything a frog can hit or bounce
//! on, the pick-ups, the flies, the tee and the lotus, all placed in the
//! world from the hole's data. Scenery that only stands about is in
//! scenery.rs; this file holds what the rules touch.

use crate::balance::cm;
use crate::content::{HazardKind, HoleDef, ItemKind, Palette, Rgb, Theme};
use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::rng::Rng;
use crate::terrain::{seed_of, seg_dist, Terrain};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

/// An upright post a frog can hit the side of or land on top of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub x: i32,
    pub z: i32,
    pub r: i32,
    pub y0: i32,
    pub y1: i32,
}

/// A flat round bouncer: a lily pad or a spider web.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bouncer {
    pub at: V3,
    pub r: i32,
    pub web: bool,
    pub kick: (i32, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Car {
    pub lane: usize,
    /// how far along its lane it started, Q16 metres
    pub offset: i32,
    pub colour: Rgb,
}

/// A car's size: length (along its lane), height, width.
pub const CAR_SIZE: V3 = v3(cm(430), cm(150), cm(200));

/// A prop as placed, for scenery.rs to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub kind: String,
    pub at: V3,
    pub size: i32,
    pub yaw: i32,
}

#[derive(Debug, Clone)]
pub struct Course {
    pub def: HoleDef,
    pub theme: Theme,
    pub terrain: Terrain,
    pub tee: V3,
    pub lotus: V3,
    pub boxes: Vec<Aabb>,
    pub columns: Vec<Column>,
    pub bouncers: Vec<Bouncer>,
    pub mushrooms: Vec<V3>,
    pub items: Vec<(ItemKind, V3)>,
    pub flies: Vec<V3>,
    pub cars: Vec<Car>,
    pub placed: Vec<Placed>,
    /// the hole's buildings, with the wall colour each is drawn in
    pub buildings: Vec<(Aabb, Rgb)>,
    /// the outside ground's height
    pub backdrop: i32,
}

impl Course {
    pub fn build(def: &HoleDef, palette: &Palette) -> Course {
        let theme = palette.themes[&def.theme].clone();
        let terrain = Terrain::build(def);
        let ground = |x: i32, z: i32| terrain.height(x, z);
        // the surface at a point: water's top where the ground is under it
        let surface = |x: i32, z: i32| {
            let g = ground(x, z);
            match def.hazard {
                Some((_, l)) if g < l => l,
                _ => g,
            }
        };
        let tee = v3(def.tee.0, ground(def.tee.0, def.tee.1), def.tee.1);
        let lotus = v3(def.lotus.0, surface(def.lotus.0, def.lotus.1), def.lotus.1);

        let mut bouncers = Vec::new();
        for &(x, z, r) in &def.pads {
            bouncers.push(Bouncer {
                at: v3(x, surface(x, z) + cm(12), z),
                r,
                web: false,
                kick: (0, 0),
            });
        }
        for w in &def.webs {
            bouncers.push(Bouncer {
                at: v3(w.x, surface(w.x, w.z) + w.height, w.z),
                r: w.r,
                web: true,
                kick: w.kick,
            });
        }
        let mushrooms = def
            .mushrooms
            .iter()
            .map(|&(x, z)| v3(x, ground(x, z), z))
            .collect();
        let items = def
            .items
            .iter()
            .map(|&(k, x, z)| (k, v3(x, surface(x, z) + cm(150), z)))
            .collect();
        let mut flies = Vec::new();
        for &(a, b, n, h, arc) in &def.flies {
            for k in 0..n {
                let t = if n > 1 { ONE * k / (n - 1) } else { 0 };
                let x = fx::lerp(a.0, b.0, t);
                let z = fx::lerp(a.1, b.1, t);
                // 4t(1 − t): 0 at the ends, 1 in the middle
                let rise = fx::mul(arc, 4 * fx::mul(t, ONE - t));
                flies.push(v3(x, surface(x, z) + h + rise, z));
            }
        }
        let mut cars = Vec::new();
        for (li, lane) in def.lanes.iter().enumerate() {
            let span = (lane.x1 - lane.x0).abs();
            for k in 0..lane.count {
                cars.push(Car {
                    lane: li,
                    offset: span / lane.count * k + (k * 7919 % 5) * ONE,
                    colour: palette.c(&lane.colours[k as usize % lane.colours.len()]),
                });
            }
        }

        let mut c = Course {
            def: def.clone(),
            theme,
            tee,
            lotus,
            boxes: Vec::new(),
            columns: Vec::new(),
            bouncers,
            mushrooms,
            items,
            flies,
            cars,
            placed: Vec::new(),
            buildings: Vec::new(),
            backdrop: def.base - cm(5),
            terrain,
        };
        for (k, b) in def.buildings.iter().enumerate() {
            let [x0, z0, x1, z1, h] = b.map(cm);
            let y = c.terrain.height((x0 + x1) / 2, (z0 + z1) / 2) - ONE;
            let bx = Aabb {
                min: v3(x0, y, z0),
                max: v3(x1, y + ONE + h, z1),
            };
            c.boxes.push(bx);
            let walls = &c.theme.walls;
            let wall = if walls.is_empty() {
                c.theme.rock
            } else {
                walls[k % walls.len()]
            };
            c.buildings.push((bx, wall));
        }
        for p in &def.props {
            c.place(&p.kind, p.x, p.z, p.size, p.yaw);
        }
        for s in def.scatter.clone() {
            c.scatter(&s.kind, s.count, s.seed, s.region);
        }
        // flies strung along the arc a shot flies, like rings along a path
        for &(a, b, power, n) in &def.shot_flies {
            let from = v3(a.0, c.ground(a.0, a.1) + crate::balance::FROG_RADIUS, a.1);
            let yaw = crate::geom::yaw_pitch_of(v3(b.0 - a.0, 0, b.1 - a.1)).0;
            let vel = crate::body::launch_velocity(
                power * 100,
                yaw,
                crate::balance::LOFTS[crate::balance::LOFT_DEFAULT],
            );
            let mut body = crate::body::Body::resting(from);
            body.launch(vel);
            let mut path = Vec::new();
            let mut ev = Vec::new();
            for t in 0..crate::balance::secs(8_000) {
                body.step(&c, crate::body::Control::default(), t, &mut ev);
                if body.grounded || !ev.is_empty() {
                    break;
                }
                path.push(body.pos);
            }
            // skip the first stretch off the catapult, then space them evenly
            let start = path.len() / 8;
            let span = path.len().saturating_sub(start + 1);
            for k in 0..n.max(1) as usize {
                if let Some(p) = path.get(start + span * k / (n.max(2) as usize - 1)) {
                    c.flies.push(*p);
                }
            }
        }
        c
    }

    pub fn ground(&self, x: i32, z: i32) -> i32 {
        self.terrain.height(x, z)
    }

    /// Is a point over the hazard (water or lava), below its top?
    pub fn in_hazard(&self, p: V3) -> Option<HazardKind> {
        let (kind, level) = self.def.hazard?;
        if p.y < level + cm(10) && self.ground(p.x, p.z) < level {
            Some(kind)
        } else {
            None
        }
    }

    pub fn out_of_bounds(&self, p: V3) -> bool {
        let [x0, z0, x1, z1] = self.def.bounds;
        p.x < x0 + ONE || p.x > x1 - ONE || p.z < z0 + ONE || p.z > z1 - ONE || p.y < self.backdrop - cm(3000)
    }

    pub fn in_lotus(&self, p: V3) -> bool {
        let d = v3(p.x - self.lotus.x, 0, p.z - self.lotus.z).len();
        d < crate::balance::LOTUS_CUP_RADIUS
            && p.y > self.lotus.y - cm(60)
            && p.y < self.lotus.y + crate::balance::LOTUS_CUP_HEIGHT
    }

    /// A car's box at a tick.
    pub fn car_box(&self, car: &Car, tick: i32) -> (Aabb, i32) {
        let lane = &self.def.lanes[car.lane];
        let span = (lane.x1 - lane.x0).abs().max(ONE);
        let travelled = (car.offset as i64 + lane.speed as i64 * tick as i64).rem_euclid(span as i64) as i32;
        let dir = if lane.x1 >= lane.x0 { 1 } else { -1 };
        let x = lane.x0 + dir * travelled;
        let y = self.ground(x, lane.z);
        let h = v3(CAR_SIZE.x / 2, 0, CAR_SIZE.z / 2);
        (
            Aabb {
                min: v3(x - h.x, y, lane.z - h.z),
                max: v3(x + h.x, y + CAR_SIZE.y, lane.z + h.z),
            },
            dir * lane.speed,
        )
    }

    /// Places one prop: the scenery to draw, and what of it a frog can hit.
    fn place(&mut self, kind: &str, x: i32, z: i32, size: i32, yaw: i32) {
        let y = self.ground(x, z);
        let s = |cmv: i32| cm(cmv) * size / 100;
        let mut column = |r: i32, h: i32| {
            self.columns.push(Column {
                x,
                z,
                r,
                y0: y - ONE,
                y1: y + h,
            });
        };
        match kind {
            "tree" => column(s(45), s(380)),
            "cypress" => column(s(70), s(520)),
            "cactus" => column(s(45), s(480)),
            "palm" => column(s(35), s(300)),
            "lamp" => column(s(22), s(650)),
            "rock" => column(s(110), s(110)),
            "boulder" => column(s(240), s(230)),
            "basalt" => column(s(130), s(260)),
            "vent" => column(s(150), s(120)),
            "hydrant" => column(s(30), s(80)),
            "planter" => column(s(110), s(70)),
            "log" => {
                let (l, r) = (s(250), s(45));
                let (hx, hz) = if (yaw / 45) % 4 == 1 || (yaw / 45) % 4 == 2 {
                    (r, l)
                } else {
                    (l, r)
                };
                self.boxes.push(Aabb {
                    min: v3(x - hx, y - ONE, z - hz),
                    max: v3(x + hx, y + 2 * r, z + hz),
                });
            }
            "bench" => {
                self.boxes.push(Aabb {
                    min: v3(x - s(90), y - ONE, z - s(30)),
                    max: v3(x + s(90), y + s(55), z + s(30)),
                });
            }
            _ => {}
        }
        self.placed.push(Placed {
            kind: kind.to_string(),
            at: v3(x, y, z),
            size,
            yaw,
        });
    }

    /// Scatters props: inside a region of the hole away from play, or in the
    /// band round the hole where nothing is in play.
    fn scatter(&mut self, kind: &str, count: i32, seed: i32, region: Option<[i32; 4]>) {
        let mut rng = Rng::new(seed as u64 ^ seed_of(&self.def.id) as u64);
        let [bx0, bz0, bx1, bz1] = self.def.bounds;
        let band = cm(9000);
        let mut placed = 0;
        let mut tries = 0;
        while placed < count && tries < count * 40 {
            tries += 1;
            let (x, z) = match region {
                Some([x0, z0, x1, z1]) => (
                    rng.range(x0 / ONE, x1 / ONE) * ONE,
                    rng.range(z0 / ONE, z1 / ONE) * ONE,
                ),
                None => {
                    let x = rng.range((bx0 - band) / ONE, (bx1 + band) / ONE) * ONE;
                    let z = rng.range((bz0 - band) / ONE, (bz1 + band) / ONE) * ONE;
                    if x > bx0 - cm(400) && x < bx1 + cm(400) && z > bz0 - cm(400) && z < bz1 + cm(400) {
                        continue;
                    }
                    (x, z)
                }
            };
            let size = rng.range(75, 130);
            let yaw = rng.range(0, 359);
            if region.is_none() {
                self.placed.push(Placed {
                    kind: kind.to_string(),
                    at: v3(x, self.backdrop, z),
                    size,
                    yaw,
                });
                placed += 1;
                continue;
            }
            if !self.clear_for_scenery(x, z) {
                continue;
            }
            self.place(kind, x, z, size, yaw);
            placed += 1;
        }
    }

    /// Somewhere a scattered prop will not block play: dry, off the
    /// fairways and paths, away from the tee, the lotus, pads and webs.
    fn clear_for_scenery(&self, x: i32, z: i32) -> bool {
        if let Some((_, l)) = self.def.hazard {
            if self.ground(x, z) < l + cm(40) {
                return false;
            }
        }
        let p = v3(x, 0, z);
        let far = |q: V3, d: i32| v3(q.x - p.x, 0, q.z - p.z).len() > d;
        if !far(self.tee, cm(1800)) || !far(self.lotus, cm(1600)) {
            return false;
        }
        if self
            .def
            .fairways
            .iter()
            .any(|&(ax, az, bx, bz, w)| seg_dist(x, z, (ax, az), (bx, bz)) < w / 2 + cm(300))
        {
            return false;
        }
        if self
            .def
            .paints
            .iter()
            .any(|pt| seg_dist(x, z, pt.a, pt.b) < pt.w / 2 + cm(150))
        {
            return false;
        }
        if self.bouncers.iter().any(|b| !far(b.at, b.r + cm(400))) {
            return false;
        }
        if self.items.iter().any(|(_, q)| !far(*q, cm(500)))
            || self.mushrooms.iter().any(|q| !far(*q, cm(500)))
        {
            return false;
        }
        if self.boxes.iter().any(|b| {
            x > b.min.x - cm(300) && x < b.max.x + cm(300) && z > b.min.z - cm(300) && z < b.max.z + cm(300)
        }) {
            return false;
        }
        true
    }

    /// Flat distance from a point to the lotus.
    pub fn to_lotus(&self, p: V3) -> i32 {
        v3(self.lotus.x - p.x, 0, self.lotus.z - p.z).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_course, tests::files};

    pub fn courses() -> Vec<Course> {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        parse_course(f.course, &pal)
            .unwrap()
            .iter()
            .map(|h| Course::build(h, &pal))
            .collect()
    }

    #[test]
    fn every_tee_is_dry_and_in_bounds_and_every_lotus_is_reachable() {
        for c in courses() {
            assert!(
                c.in_hazard(c.tee.add(v3(0, crate::balance::FROG_RADIUS, 0)))
                    .is_none(),
                "{}",
                c.def.id
            );
            assert!(!c.out_of_bounds(c.tee), "{}", c.def.id);
            assert!(c.in_lotus(c.lotus.add(v3(0, ONE / 2, 0))), "{}", c.def.id);
            assert!(
                c.to_lotus(c.tee) > cm(8000),
                "{}: a hole is longer than a putt",
                c.def.id
            );
        }
    }

    #[test]
    fn scattered_props_keep_off_the_fairway_and_the_tee() {
        for c in courses() {
            for p in &c.placed {
                assert!(
                    v3(p.at.x - c.tee.x, 0, p.at.z - c.tee.z).len() > cm(1000)
                        || c.def.props.iter().any(|q| q.x == p.at.x && q.z == p.at.z),
                    "{}: a {} on the tee",
                    c.def.id,
                    p.kind
                );
            }
        }
    }

    #[test]
    fn a_car_drives_along_its_lane_and_wraps() {
        let cs = courses();
        let city = cs
            .iter()
            .find(|c| !c.cars.is_empty())
            .expect("a hole with traffic");
        let car = &city.cars[0];
        let (a, _) = city.car_box(car, 0);
        let (b, v) = city.car_box(car, 30);
        assert_ne!(a.min.x, b.min.x, "it moves");
        assert!(v != 0);
        let lane = &city.def.lanes[car.lane];
        let (lo, hi) = (lane.x0.min(lane.x1), lane.x0.max(lane.x1));
        for t in (0..20_000).step_by(997) {
            let (bx, _) = city.car_box(car, t);
            let mid = (bx.min.x + bx.max.x) / 2;
            assert!(
                mid >= lo - ONE && mid <= hi + ONE,
                "car at {mid} outside {lo}..{hi}"
            );
        }
    }
}
