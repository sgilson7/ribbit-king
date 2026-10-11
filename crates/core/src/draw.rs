//! Everything on a hole that moves or can change, drawn fresh every frame:
//! the frogs, the catapult and its mallet, the aiming guide, flies,
//! power-ups, mushrooms, lily pads, webs, the lotus, cars, the tongue,
//! shadows and particles.

use crate::balance::*;
use crate::content::{FrogDef, ItemKind, Palette, Rgb};
use crate::course::{Course, CAR_SIZE};
use crate::frog::{self, Pose};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{shade, Batch, PASS_SOLID};
use crate::round::{Phase, Round};
use crate::scenery::{at, put, rod, tilted};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Particle {
    pub pos: V3,
    pub vel: V3,
    pub life: i32,
    pub max: i32,
    pub rgb: Rgb,
    pub size: i32,
    pub glow: i32,
    pub fall: bool,
}

impl Particle {
    pub fn step(&mut self) {
        self.life -= 1;
        self.pos = self.pos.add(self.vel);
        if self.fall {
            self.vel.y -= GRAVITY * 2 / 3;
        } else {
            self.vel = self.vel.scale(ONE * 94 / 100);
        }
    }
}

pub struct View<'a> {
    pub course: &'a Course,
    pub round: Option<&'a Round>,
    pub pal: &'a Palette,
    pub frogs: &'a [FrogDef],
    pub tick: i32,
    pub particles: &'a [Particle],
    /// the aiming guide's points, when it is shown
    pub guide: &'a [V3],
    /// which player wears the crown (the leader), if anyone
    pub leader: Option<usize>,
}

pub fn scene(b: &mut Batch, v: &View) {
    let c = v.course;
    let pal = v.pal;
    let t = v.tick;
    lotus(b, pal, c.lotus, t);
    // the active frog's flies and power-ups this stroke
    let active = v.round.map(|r| &r.players[r.active]);
    for (k, f) in c.flies.iter().enumerate() {
        if active
            .map(|p| p.caught.get(k).copied().unwrap_or(false))
            .unwrap_or(false)
        {
            continue;
        }
        fly(b, pal, *f, t + k as i32 * 13);
    }
    let taken = v.round.map(|r| r.taken).unwrap_or(0);
    for (k, (kind, p)) in c.items.iter().enumerate() {
        if taken & (1 << k) == 0 {
            item(b, pal, *kind, *p, t + k as i32 * 40);
        }
    }
    let popped = active.map(|p| p.body.popped).unwrap_or(0);
    for (k, m) in c.mushrooms.iter().enumerate() {
        if popped & (1 << k) == 0 {
            mushroom(b, pal, *m, t + k as i32 * 21);
        }
    }
    for (k, bo) in c.bouncers.iter().enumerate() {
        let since = v.round.and_then(|r| r.bounced.get(k).copied()).unwrap_or(999);
        if bo.web {
            web(b, pal, c, bo.at, bo.r, since);
        } else {
            pad(b, pal, bo.at, bo.r, since, k as i32);
        }
    }
    for car in &c.cars {
        let (bx, vx) = c.car_box(car, v.round.map(|r| r.tick).unwrap_or(t));
        draw_car(b, pal, bx.min, bx.max, car.colour, vx);
    }
    if let Some(r) = v.round {
        round(b, v, r);
    }
    for p in v.particles {
        let k = ONE * p.life / p.max.max(1);
        let s = fx::mul(p.size, ONE / 2 + k / 2);
        b.push(Mesh::Blob, &at(p.pos, 0, v3(s, s, s)), p.rgb, p.glow, PASS_SOLID);
    }
    for g in v.guide.iter().skip(2) {
        b.push(
            Mesh::Sphere,
            &at(*g, 0, v3(cm(22), cm(22), cm(22))),
            pal.c("guide"),
            ONE * 7 / 10,
            PASS_SOLID,
        );
    }
    if let Some(end) = v.guide.last() {
        let y = c
            .ground(end.x, end.z)
            .max(c.def.hazard.map(|h| h.1).unwrap_or(i32::MIN));
        let pulse = cm(150) + fx::mul(fx::sin(t * deg(6)), cm(25));
        b.push(
            Mesh::Ring,
            &at(v3(end.x, y + cm(15), end.z), 0, v3(pulse * 2, cm(20), pulse * 2)),
            pal.c("guide_end"),
            ONE / 2,
            PASS_SOLID,
        );
    }
}

fn round(b: &mut Batch, v: &View, r: &Round) {
    let c = v.course;
    let pal = v.pal;
    let t = v.tick;
    let at_tee = matches!(r.phase, Phase::Aim | Phase::Meter | Phase::Swing);
    if r.phase != Phase::Done {
        catapult(b, pal, r, t);
    }
    for (i, p) in r.players.iter().enumerate() {
        let frog_def = &v.frogs[p.frog];
        let mut pose = Pose {
            pos: r.standing_spot(i),
            yaw: p.yaw,
            pitch: 0,
            stretch: 0,
            squash: 0,
            wings: None,
            crown: v.leader == Some(i),
            gape: r
                .tongue
                .map(|tg| tg.from == i && tg.t < TONGUE_TICKS / 2)
                .unwrap_or(false),
            scale: 100,
        };
        if p.holed {
            // the last frog in sits up in the lotus and bounces
            if i == r.active && r.phase == Phase::Holed {
                let hop = fx::mul(fx::sin(t * deg(9)).abs(), cm(60));
                pose.pos = c.lotus.add(v3(0, cm(95) + hop, 0));
                pose.yaw = t * deg(3);
                frog::draw(b, frog_def, pal, &pose);
            }
            continue;
        }
        if i == r.active && at_tee {
            pose.pos = p.lie.add(v3(0, cm(30), 0));
            pose.yaw = r.yaw;
            if r.phase == Phase::Swing {
                // crouching for the throw
                pose.squash = ONE * r.t / SWING_HIT.max(1);
            }
        } else if p.body.moving {
            let vel = p.body.vel;
            let (yaw, pitch) = crate::geom::yaw_pitch_of(vel);
            if v3(vel.x, 0, vel.z).len() > ONE / 200 {
                pose.yaw = yaw;
            }
            pose.pitch = fx::wrap(pitch).clamp(-deg(50), deg(40));
            let speed = vel.len();
            pose.stretch = if p.body.grounded { 0 } else { (speed * 3).min(ONE) };
            if p.body.wings > 0 {
                pose.wings = Some(t * deg(30));
            }
            // a shadow on the ground under a frog in the air
            let g = c
                .ground(p.body.pos.x, p.body.pos.z)
                .max(c.def.hazard.map(|h| h.1).unwrap_or(i32::MIN));
            let high = (p.body.pos.y - g).clamp(0, int(30));
            let w = cm(110) - fx::mul(high, ONE * 2 / 100);
            if high > cm(60) {
                b.push(
                    Mesh::Cyl,
                    &at(v3(p.body.pos.x, g + cm(6), p.body.pos.z), 0, v3(w, cm(4), w)),
                    c.theme.ink,
                    0,
                    PASS_SOLID,
                );
            }
        }
        if p.landed < 10 {
            pose.squash = pose.squash.max(ONE * (10 - p.landed) / 20);
        }
        frog::draw(b, frog_def, pal, &pose);
        // the frog whose turn it is has a marker over its head
        if i == r.active && r.phase != Phase::Done {
            let bob = fx::mul(fx::sin(t * deg(5)), cm(15));
            let above = pose.pos.add(v3(0, cm(170) + bob, 0));
            b.inked(
                Mesh::Octa,
                &at(above, t * deg(4), v3(cm(35), cm(45), cm(35))),
                frog_def.skin,
                ONE / 3,
            );
        }
    }
    if let Some(tg) = r.tongue {
        let from_p = &r.players[tg.from];
        let pose = Pose {
            pos: if r.phase == Phase::Aim && tg.from == r.active {
                from_p.lie.add(v3(0, cm(30), 0))
            } else {
                from_p.body.pos
            },
            yaw: if tg.from == r.active && r.phase == Phase::Aim {
                r.yaw
            } else {
                from_p.yaw
            },
            pitch: 0,
            stretch: 0,
            squash: 0,
            wings: None,
            crown: false,
            gape: true,
            scale: 100,
        };
        let mouth = frog::mouth(&pose);
        let target = r.players[tg.to].body.pos;
        // out in the first half, back in the second
        let half = TONGUE_TICKS / 2;
        let reach = if tg.t < half {
            ONE * tg.t / half.max(1)
        } else {
            ONE * (TONGUE_TICKS - tg.t) / half.max(1)
        };
        let tip = mouth.lerp(target, reach);
        b.inked(Mesh::Cyl, &rod(mouth, tip, cm(14)), pal.c("tongue"), 0);
        b.inked(
            Mesh::Sphere,
            &at(tip, 0, v3(cm(30), cm(26), cm(30))),
            pal.c("tongue"),
            0,
        );
    }
}

/// The catapult: a plank on a fulcrum, the frog on its front end, a pad on
/// its back end, and a mallet on a post that swings down onto the pad.
/// Beside it, the carnival striker's tower with its bell.
fn catapult(b: &mut Batch, pal: &Palette, r: &Round, t: i32) {
    let (base, yaw) = r.catapult;
    let fwd = facing(yaw, 0);
    let right = right_of(yaw);
    let local = |x: i32, y: i32, z: i32| {
        base.add(right.scale(cm(x)))
            .add(v3(0, cm(y), 0))
            .sub(fwd.scale(cm(z)))
    };
    // the lever's tilt: back end up while waiting, slammed down after the hit
    let after = r.since_hit;
    let tilt = if after < secs(250) {
        -deg(22)
    } else if after < secs(900) {
        fx::lerp(-deg(22), deg(10), ONE * (after - secs(250)) / secs(650))
    } else {
        deg(10)
    };
    put(
        b,
        Mesh::Cube,
        &tilted(local(0, 10, 90), yaw, 0, v3(cm(110), cm(20), cm(280))),
        pal.c("wood_dark"),
        true,
    );
    put(
        b,
        Mesh::Wedge,
        &tilted(local(0, 38, 90), yaw + fx::HALF, 0, v3(cm(60), cm(40), cm(50))),
        pal.c("wood"),
        true,
    );
    let pivot = local(0, 58, 90);
    let plank = Affine::translate(pivot)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::rot_x(tilt))
        .then(&Affine::translate(v3(0, 0, cm(40))))
        .then(&Affine::scale(v3(cm(70), cm(12), cm(300))));
    b.inked(Mesh::Cube, &plank, pal.c("wood_light"), 0);
    let pad_at = Affine::translate(pivot)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::rot_x(tilt))
        .then(&Affine::translate(v3(0, cm(10), cm(165))))
        .then(&Affine::scale(v3(cm(60), cm(10), cm(60))));
    b.inked(Mesh::Cyl, &pad_at, pal.c("striker"), 0);
    // the mallet hangs from a post to the right of the pad and swings
    // down sideways onto it, so the view from behind the frog stays clear
    let post_top = local(140, 200, 255);
    b.inked(
        Mesh::Cyl,
        &rod(local(140, 0, 255), post_top, cm(22)),
        pal.c("wood_dark"),
        0,
    );
    let raised = deg(115);
    let swing = match r.phase {
        Phase::Swing => {
            let k = ONE * r.t / SWING_HIT.max(1);
            if k < ONE * 55 / 100 {
                fx::lerp(raised, deg(150), fx::div(k, ONE * 55 / 100))
            } else {
                fx::lerp(deg(150), 0, fx::div(k - ONE * 55 / 100, ONE * 45 / 100))
            }
        }
        _ if after < secs(400) => 0,
        _ if after < secs(1100) => fx::lerp(0, raised, ONE * (after - secs(400)) / secs(700)),
        _ => raised + fx::mul(fx::sin(t * deg(2)), deg(4)),
    };
    // at rest on the pad the arm points down and left from the post top
    let arm = cm(180);
    let ang = deg(222) - swing;
    let dir = right.scale(fx::cos(ang)).add(v3(0, fx::sin(ang), 0));
    let head = post_top.add(dir.scale(arm));
    b.inked(Mesh::Cyl, &rod(post_top, head, cm(12)), pal.c("wood"), 0);
    let across = right.scale(-fx::sin(ang)).add(v3(0, fx::cos(ang), 0));
    b.inked(
        Mesh::Cyl,
        &rod(
            head.sub(across.scale(cm(48))),
            head.add(across.scale(cm(48))),
            cm(58),
        ),
        pal.c("mallet"),
        0,
    );
    for side in [-1, 1] {
        let c0 = head.add(across.scale(side * cm(36)));
        b.push(
            Mesh::Cyl,
            &rod(c0.sub(across.scale(cm(5))), c0.add(across.scale(cm(5))), cm(62)),
            pal.c("mallet_band"),
            0,
            PASS_SOLID,
        );
    }
    // the striker tower: stripes up to a bell, the puck thrown up by the power
    let foot = local(-230, 0, 60);
    for k in 0..6 {
        let colour = if k % 2 == 0 {
            pal.c("striker")
        } else {
            pal.c("white")
        };
        b.inked(
            Mesh::Cyl,
            &at(foot.add(v3(0, cm(45 + k * 70), 0)), 0, v3(cm(26), cm(70), cm(26))),
            colour,
            0,
        );
    }
    let rang = matches!(r.shot, Some(s) if s.hit == crate::round::Hit::Perfect) && after < secs(1500);
    let ring = if rang {
        fx::mul(fx::sin(t * deg(40)), deg(15))
    } else {
        0
    };
    let bell = Affine::translate(foot.add(v3(0, cm(470), 0)))
        .then(&Affine::rot_z(ring))
        .then(&Affine::scale(v3(cm(60), cm(55), cm(60))));
    b.inked(Mesh::Cone, &bell, pal.c("crown"), if rang { ONE / 2 } else { 0 });
    let puck_h = match r.shot {
        Some(s) if after < secs(1500) => {
            let top = (cm(400) as i64 * s.power.min(10_000) as i64 / 10_000) as i32;
            let k = if after < secs(300) {
                ONE * after / secs(300)
            } else {
                ONE - ONE * (after - secs(300)) / secs(1200)
            };
            fx::mul(top, k)
        }
        _ => 0,
    };
    b.inked(
        Mesh::Cube,
        &at(
            foot.add(v3(0, cm(25) + puck_h, 0)).add(right.scale(cm(22))),
            yaw,
            v3(cm(18), cm(14), cm(30)),
        ),
        pal.c("crown"),
        0,
    );
}

fn lotus(b: &mut Batch, pal: &Palette, p: V3, t: i32) {
    let bob = fx::mul(fx::sin(t * deg(2)), cm(4));
    let o = p.add(v3(0, bob, 0));
    put(
        b,
        Mesh::Cyl,
        &at(o.add(v3(0, cm(4), 0)), 0, v3(cm(520), cm(10), cm(520))),
        pal.c("lotus_leaf"),
        true,
    );
    for k in 0..10 {
        let a = deg(36) * k + t * deg(1) / 4;
        let petal = tilted(
            o.add(facing(a, 0).scale(cm(120))).add(v3(0, cm(35), 0)),
            a,
            deg(35),
            v3(cm(80), cm(30), cm(170)),
        );
        put(b, Mesh::Sphere, &petal, pal.c("lotus"), true);
    }
    for k in 0..7 {
        let a = deg(51) * k + deg(20);
        let petal = tilted(
            o.add(facing(a, 0).scale(cm(70))).add(v3(0, cm(70), 0)),
            a,
            deg(62),
            v3(cm(60), cm(25), cm(130)),
        );
        put(b, Mesh::Sphere, &petal, pal.c("lotus_light"), true);
    }
    put(
        b,
        Mesh::Cyl,
        &at(o.add(v3(0, cm(55), 0)), 0, v3(cm(90), cm(40), cm(90))),
        pal.c("lotus_centre"),
        true,
    );
    // a beacon that shows from the tee
    let pulse = ONE / 3 + fx::mul(fx::sin(t * deg(3)), ONE / 10);
    b.glow(
        Mesh::Cyl,
        &at(o.add(v3(0, int(22), 0)), 0, v3(cm(130), int(42), cm(130))),
        pal.c("beacon"),
        pulse,
    );
}

fn fly(b: &mut Batch, pal: &Palette, p: V3, t: i32) {
    let o = p.add(v3(0, fx::mul(fx::sin(t * deg(5)), cm(25)), 0));
    let yaw = t * deg(2);
    put(
        b,
        Mesh::Sphere,
        &at(o, yaw, v3(cm(48), cm(44), cm(48))),
        pal.c("fly_body"),
        true,
    );
    let f = facing(yaw, 0);
    let r = right_of(yaw);
    for side in [-1, 1] {
        let eye = o
            .add(f.scale(cm(18)))
            .add(r.scale(side * cm(10)))
            .add(v3(0, cm(6), 0));
        b.push(
            Mesh::Sphere,
            &at(eye, 0, v3(cm(13), cm(13), cm(13))),
            pal.c("fly_eye"),
            ONE * 6 / 10,
            PASS_SOLID,
        );
        let flap = fx::mul(fx::sin(t * deg(70)), deg(35));
        let wing = Affine::translate(o.add(r.scale(side * cm(26))).add(v3(0, cm(18), cm(0))))
            .then(&Affine::rot_y(yaw))
            .then(&Affine::rot_z(side * (deg(25) + flap)))
            .then(&Affine::scale(v3(cm(40), cm(6), cm(24))));
        b.push(Mesh::Sphere, &wing, pal.c("fly_wing"), ONE / 3, PASS_SOLID);
    }
}

fn item(b: &mut Batch, pal: &Palette, kind: ItemKind, p: V3, t: i32) {
    let o = p.add(v3(0, fx::mul(fx::sin(t * deg(3)), cm(20)), 0));
    let yaw = t * deg(2);
    b.glow(
        Mesh::Sphere,
        &at(o, 0, v3(cm(210), cm(210), cm(210))),
        pal.c("bubble"),
        ONE / 4,
    );
    match kind {
        ItemKind::Jump => {
            for k in 0..4 {
                put(
                    b,
                    Mesh::Ring,
                    &at(o.add(v3(0, cm(-45 + k * 22), 0)), yaw, v3(cm(70), cm(10), cm(70))),
                    if k % 2 == 0 {
                        pal.c("spring")
                    } else {
                        pal.c("spring_dark")
                    },
                    true,
                );
            }
            put(
                b,
                Mesh::Cone,
                &at(o.add(v3(0, cm(55), 0)), yaw, v3(cm(70), cm(50), cm(70))),
                pal.c("arrow"),
                true,
            );
        }
        ItemKind::Wings => {
            let flap = fx::mul(fx::sin(t * deg(8)), deg(20));
            for side in [-1, 1] {
                let wing = Affine::translate(o)
                    .then(&Affine::rot_y(yaw))
                    .then(&Affine::rot_z(side * (deg(20) + flap)))
                    .then(&Affine::translate(v3(side * cm(45), 0, 0)))
                    .then(&Affine::scale(v3(cm(90), cm(14), cm(50))));
                b.inked(Mesh::Sphere, &wing, pal.c("wings"), 0);
            }
        }
    }
}

fn mushroom(b: &mut Batch, pal: &Palette, p: V3, t: i32) {
    let throb = ONE + fx::mul(fx::sin(t * deg(6)), ONE / 25);
    put(
        b,
        Mesh::Cyl,
        &at(p.add(v3(0, cm(35), 0)), 0, v3(cm(55), cm(70), cm(55))),
        pal.c("mushroom_stem"),
        true,
    );
    let cap = at(
        p.add(v3(0, cm(80), 0)),
        0,
        v3(
            fx::mul(cm(170), throb),
            fx::mul(cm(95), throb),
            fx::mul(cm(170), throb),
        ),
    );
    put(b, Mesh::Sphere, &cap, pal.c("mushroom_cap"), true);
    for k in 0..6 {
        let a = deg(60) * k;
        let s = p.add(facing(a, 0).scale(cm(55))).add(v3(0, cm(105), 0));
        put(
            b,
            Mesh::Sphere,
            &at(s, 0, v3(cm(30), cm(14), cm(30))),
            pal.c("mushroom_spot"),
            false,
        );
    }
    put(
        b,
        Mesh::Sphere,
        &at(p.add(v3(0, cm(128), 0)), 0, v3(cm(30), cm(14), cm(30))),
        pal.c("mushroom_spot"),
        false,
    );
    // the fuse, sparking
    let spark = ONE / 2 + fx::mul(fx::sin(t * deg(31)), ONE / 2);
    b.push(
        Mesh::Cyl,
        &at(p.add(v3(0, cm(140), 0)), 0, v3(cm(6), cm(30), cm(6))),
        pal.c("wood_dark"),
        0,
        PASS_SOLID,
    );
    b.push(
        Mesh::Octa,
        &at(p.add(v3(0, cm(160), 0)), t * deg(11), v3(cm(18), cm(18), cm(18))),
        pal.c("spark"),
        spark,
        PASS_SOLID,
    );
}

fn pad(b: &mut Batch, pal: &Palette, p: V3, r: i32, since: i32, k: i32) {
    let dip = if since < 20 {
        fx::mul(fx::sin(since * deg(9)), cm(25))
    } else {
        0
    };
    let o = p.sub(v3(0, cm(8) + dip, 0));
    put(
        b,
        Mesh::Cyl,
        &at(o, k * deg(47), v3(r * 2, cm(10), r * 2)),
        pal.c("pad"),
        true,
    );
    // the notch
    put(
        b,
        Mesh::Wedge,
        &at(
            o.add(facing(k * deg(47), 0).scale(r * 3 / 4))
                .add(v3(0, cm(4), 0)),
            k * deg(47),
            v3(cm(30), cm(6), r / 2),
        ),
        pal.c("pad_dark"),
        false,
    );
    if k % 2 == 0 {
        put(
            b,
            Mesh::Sphere,
            &at(
                o.add(right_of(k * deg(47)).scale(r / 2)).add(v3(0, cm(15), 0)),
                0,
                v3(cm(35), cm(25), cm(35)),
            ),
            pal.c("pad_flower"),
            true,
        );
    }
}

fn web(b: &mut Batch, pal: &Palette, c: &Course, p: V3, r: i32, since: i32) {
    let sag = if since < 24 {
        fx::mul(fx::sin(since * deg(8)), cm(45))
    } else {
        0
    };
    let o = p.sub(v3(0, sag, 0));
    // four posts
    for k in 0..4 {
        let a = deg(45) + deg(90) * k;
        let foot = p.add(facing(a, 0).scale(r + cm(30)));
        let g = c.ground(foot.x, foot.z);
        b.inked(
            Mesh::Cyl,
            &rod(v3(foot.x, g, foot.z), v3(foot.x, p.y + cm(40), foot.z), cm(20)),
            pal.c("web_post"),
            0,
        );
        b.push(
            Mesh::Cyl,
            &rod(
                v3(foot.x, p.y + cm(10), foot.z),
                o.add(facing(a, 0).scale(r)),
                cm(4),
            ),
            pal.c("web"),
            ONE / 3,
            PASS_SOLID,
        );
    }
    for k in 0..8 {
        let a = deg(45) * k;
        b.push(
            Mesh::Cyl,
            &rod(o, o.add(facing(a, 0).scale(r)), cm(4)),
            pal.c("web"),
            ONE / 3,
            PASS_SOLID,
        );
    }
    for ring in 1..=3 {
        let rr = r * ring / 3;
        b.push(
            Mesh::Ring,
            &at(o, 0, v3(rr * 2, cm(4), rr * 2)),
            pal.c("web"),
            ONE / 3,
            PASS_SOLID,
        );
    }
}

fn draw_car(b: &mut Batch, pal: &Palette, min: V3, max: V3, colour: Rgb, vx: i32) {
    let mid = v3((min.x + max.x) / 2, min.y, (min.z + max.z) / 2);
    let yaw = if vx >= 0 { -fx::QUARTER } else { fx::QUARTER };
    let body = at(mid.add(v3(0, cm(65), 0)), yaw, v3(CAR_SIZE.z, cm(75), CAR_SIZE.x));
    b.inked(Mesh::Cube, &body, colour, 0);
    let cab = at(
        mid.add(v3(0, cm(125), 0)),
        yaw,
        v3(CAR_SIZE.z - cm(30), cm(55), CAR_SIZE.x / 2),
    );
    b.inked(Mesh::Cube, &cab, shade(colour, 90), 0);
    let glass = at(
        mid.add(v3(0, cm(128), 0)),
        yaw,
        v3(CAR_SIZE.z - cm(24), cm(38), CAR_SIZE.x / 2 + cm(6)),
    );
    b.push(Mesh::Cube, &glass, pal.c("glass"), 0, PASS_SOLID);
    let fwd = facing(yaw, 0);
    let side = right_of(yaw);
    for (f, s) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let w = mid
            .add(fwd.scale(f * cm(140)))
            .add(side.scale(s * cm(95)))
            .add(v3(0, cm(35), 0));
        let wheel = Affine::translate(w)
            .then(&Affine::rot_y(yaw))
            .then(&Affine::rot_z(fx::QUARTER))
            .then(&Affine::scale(v3(cm(70), cm(25), cm(70))));
        b.inked(Mesh::Cyl, &wheel, pal.c("tyre"), 0);
    }
    for s in [-1, 1] {
        let lamp = mid
            .add(fwd.scale(CAR_SIZE.x / 2))
            .add(side.scale(s * cm(65)))
            .add(v3(0, cm(70), 0));
        b.push(
            Mesh::Sphere,
            &at(lamp, 0, v3(cm(25), cm(20), cm(10))),
            pal.c("glow_lamp"),
            ONE,
            PASS_SOLID,
        );
    }
}
