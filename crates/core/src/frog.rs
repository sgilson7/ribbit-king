//! A frog, drawn from its colours and pattern (data/frogs.json): a squat
//! body, a wide head with two large eyes on top, folded back legs and small
//! front feet. Sitting, it crouches; flying, it stretches out like a
//! superhero. The parts are written in centimetres about the centre of the
//! ball the physics rolls (the feet are at −45).

use crate::content::{FrogDef, Palette, Pattern, Rgb};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{shade, Batch, PASS_SOLID};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub pos: V3,
    pub yaw: i32,
    pub pitch: i32,
    /// 0 sitting, ONE stretched in flight
    pub stretch: i32,
    /// a squash on landing or a hop, 0..ONE
    pub squash: i32,
    /// wings on, flapping at this phase
    pub wings: Option<i32>,
    pub crown: bool,
    /// the mouth open for the tongue
    pub gape: bool,
    /// drawn this much smaller or larger, percent (the setup screen's spin)
    pub scale: i32,
}

fn c(x: i32, y: i32, z: i32) -> V3 {
    v3(
        crate::balance::cm(x),
        crate::balance::cm(y),
        crate::balance::cm(z),
    )
}

fn lerp3(a: V3, b: V3, t: i32) -> V3 {
    a.lerp(b, t)
}

/// A point on the body's shell, by angle round it and angle up it.
fn on_body(around: i32, up: i32) -> V3 {
    let (rx, ry, rz) = (49, 36, 58);
    let ce = fx::cos(deg(up));
    let p = v3(
        fx::mul(fx::mul(fx::sin(deg(around)), ce), crate::balance::cm(rx)),
        fx::mul(fx::sin(deg(up)), crate::balance::cm(ry)),
        fx::mul(fx::mul(fx::cos(deg(around)), ce), crate::balance::cm(rz)),
    );
    p.add(c(0, -4, 6))
}

/// The pattern's marks: where on the body (round, up) and how wide (cm).
fn marks(pattern: Pattern) -> Vec<(i32, i32, i32)> {
    match pattern {
        Pattern::Plain => vec![],
        Pattern::Spots => vec![
            (-40, 50, 22),
            (35, 55, 18),
            (0, 70, 20),
            (-70, 25, 16),
            (75, 30, 17),
            (150, 45, 18),
            (-150, 40, 16),
            (110, 15, 14),
            (-110, 15, 14),
        ],
        Pattern::Warts => {
            let mut v = Vec::new();
            for k in 0..22 {
                v.push(((k * 137) % 360 - 180, 20 + (k * 53) % 60, 9));
            }
            v
        }
        Pattern::Dots => {
            let mut v = Vec::new();
            for k in 0..18 {
                v.push(((k * 97) % 360 - 180, 15 + (k * 41) % 65, 7));
            }
            v
        }
        Pattern::Blotches => vec![
            (-30, 45, 34),
            (40, 55, 30),
            (160, 50, 32),
            (-120, 30, 28),
            (95, 20, 26),
        ],
        Pattern::Bands | Pattern::Flanks => vec![],
    }
}

/// Draws one frog.
pub fn draw(b: &mut Batch, f: &FrogDef, pal: &Palette, pose: &Pose) {
    let k = ONE * pose.scale / 100;
    let squash = v3(
        ONE + pose.squash / 4,
        ONE - pose.squash / 3,
        ONE + pose.squash / 4,
    );
    let base = Affine::translate(pose.pos)
        .then(&Affine::rot_y(pose.yaw))
        .then(&Affine::rot_x(pose.pitch))
        .then(&Affine::translate(c(0, -45, 0)))
        .then(&Affine::scale(v3(
            fx::mul(k, squash.x),
            fx::mul(k, squash.y),
            fx::mul(k, squash.z),
        )))
        .then(&Affine::translate(c(0, 45, 0)));
    let part = |b: &mut Batch, mesh: Mesh, at: V3, size: V3, rgb: Rgb, ink: bool| {
        let a = base.then(&Affine::translate(at)).then(&Affine::scale(size));
        if ink {
            b.inked(mesh, &a, rgb, 0);
        } else {
            b.push(mesh, &a, rgb, 0, PASS_SOLID);
        }
    };
    let turned = |b: &mut Batch, mesh: Mesh, at: V3, rot: Affine, size: V3, rgb: Rgb, ink: bool| {
        let a = base
            .then(&Affine::translate(at))
            .then(&rot)
            .then(&Affine::scale(size));
        if ink {
            b.inked(mesh, &a, rgb, 0);
        } else {
            b.push(mesh, &a, rgb, 0, PASS_SOLID);
        }
    };
    let s = pose.stretch;

    // body, belly, head
    turned(
        b,
        Mesh::Sphere,
        c(0, -4, 6),
        Affine::rot_x(deg(14)),
        c(98, 72, 116),
        f.skin,
        true,
    );
    part(b, Mesh::Sphere, c(0, -16, -14), c(80, 50, 84), f.belly, false);
    part(b, Mesh::Sphere, c(0, 10, -36), c(92, 56, 74), f.skin, true);
    // throat, under the chin
    part(b, Mesh::Sphere, c(0, -4, -46), c(64, 30, 46), f.belly, false);

    // the mouth: a wide smile, or open for the tongue
    if pose.gape {
        part(
            b,
            Mesh::Sphere,
            c(0, 2, -70),
            c(46, 22, 12),
            pal.c("mouth"),
            false,
        );
    } else {
        for side in [-1, 1] {
            turned(
                b,
                Mesh::Cube,
                c(side * 15, 4, -71),
                Affine::rot_y(side * deg(18)).then(&Affine::rot_z(side * deg(12))),
                c(32, 3, 3),
                pal.c("mouth"),
                false,
            );
        }
    }

    // eyes: sockets, irises, pupils and a glint, looking forward and out
    for side in [-1, 1] {
        let socket = c(side * 25, 32, -38);
        part(b, Mesh::Sphere, socket, c(42, 40, 42), f.skin, true);
        let look = v3(side * ONE * 45 / 100, ONE * 25 / 100, -ONE * 85 / 100).norm();
        let iris = socket.add(look.scale(crate::balance::cm(11)));
        part(b, Mesh::Sphere, iris, c(30, 30, 30), f.eye, true);
        let pupil = socket.add(look.scale(crate::balance::cm(20)));
        part(b, Mesh::Sphere, pupil, c(14, 16, 14), pal.c("pupil"), false);
        let glint = pupil.add(c(-side * 3, 5, -3));
        let a = base
            .then(&Affine::translate(glint))
            .then(&Affine::scale(c(5, 5, 5)));
        b.push(Mesh::Sphere, &a, pal.c("glint"), ONE, PASS_SOLID);
    }

    // back legs: folded under when sitting, trailing when flying
    for side in [-1, 1] {
        let thigh = lerp3(c(side * 40, -18, 22), c(side * 30, -14, 56), s);
        part(
            b,
            Mesh::Sphere,
            thigh,
            lerp3(c(36, 36, 62), c(28, 28, 72), s),
            f.skin,
            true,
        );
        let foot = lerp3(c(side * 50, -41, -4), c(side * 36, -18, 100), s);
        let toe_turn = Affine::rot_y(side * deg(20)).then(&Affine::rot_x(fx::mul(s, deg(70))));
        turned(b, Mesh::Sphere, foot, toe_turn, c(42, 10, 56), f.feet, true);
    }
    // front legs: down to the ground, or reaching ahead
    for side in [-1, 1] {
        let shoulder = c(side * 28, -14, -36);
        let hand = lerp3(c(side * 33, -42, -56), c(side * 42, -8, -78), s);
        let arm = crate::scenery::rod(shoulder, hand, crate::balance::cm(15));
        b.inked(Mesh::Cyl, &base.then(&arm), f.skin, 0);
        part(b, Mesh::Sphere, hand, c(28, 8, 28), f.feet, true);
    }

    // the pattern
    match f.pattern {
        Pattern::Bands => {
            for (z, w) in [(-8, 14), (22, 14)] {
                turned(
                    b,
                    Mesh::Sphere,
                    c(0, -4, z),
                    Affine::rot_x(deg(14)),
                    c(102, 76, w),
                    f.mark,
                    false,
                );
            }
            part(b, Mesh::Sphere, c(0, 12, -36), c(96, 58, 12), f.mark, false);
        }
        Pattern::Flanks => {
            part(b, Mesh::Sphere, c(0, -10, 8), c(101, 12, 100), f.mark, false);
            for z in [-10, 12, 34] {
                turned(
                    b,
                    Mesh::Sphere,
                    c(0, -2, z),
                    Affine::rot_x(deg(14)),
                    c(101, 30, 7),
                    shade(f.mark, 80),
                    false,
                );
            }
        }
        p => {
            for (around, up, w) in marks(p) {
                let at = on_body(around, up);
                let flat = if p == Pattern::Blotches { 45 } else { 60 };
                part(b, Mesh::Sphere, at, c(w, w * flat / 100, w), f.mark, false);
            }
        }
    }

    if let Some(phase) = pose.wings {
        let flap = fx::mul(fx::sin(phase), deg(28));
        for side in [-1, 1] {
            let root = c(side * 30, 26, 18);
            let rot = Affine::rot_z(side * (deg(15) + flap)).then(&Affine::rot_y(side * deg(10)));
            let wing = base.then(&Affine::translate(root)).then(&rot);
            for (dx, w, l) in [(38, 70, 40), (78, 50, 30), (108, 34, 22)] {
                let a = wing
                    .then(&Affine::translate(c(side * dx, 0, dx / 4)))
                    .then(&Affine::scale(c(w, 9, l)));
                b.inked(
                    Mesh::Sphere,
                    &a,
                    if dx > 100 {
                        pal.c("wing_tip")
                    } else {
                        pal.c("wings")
                    },
                    0,
                );
            }
        }
    }

    if pose.crown {
        let at = c(0, 44, -18);
        part(b, Mesh::Cyl, at, c(34, 12, 34), pal.c("crown"), true);
        for kk in 0..5 {
            let a = deg(72) * kk;
            let p = at.add(v3(
                fx::mul(fx::sin(a), crate::balance::cm(14)),
                crate::balance::cm(12),
                fx::mul(fx::cos(a), crate::balance::cm(14)),
            ));
            part(b, Mesh::Cone, p, c(10, 16, 10), pal.c("crown"), true);
        }
        part(
            b,
            Mesh::Sphere,
            at.add(c(0, 2, -17)),
            c(8, 8, 8),
            pal.c("crown_gem"),
            false,
        );
    }
}

/// The mouth, in world space, for the tongue to start from.
pub fn mouth(pose: &Pose) -> V3 {
    let a = Affine::translate(pose.pos)
        .then(&Affine::rot_y(pose.yaw))
        .then(&Affine::rot_x(pose.pitch));
    a.apply(c(0, 2, -70))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_frogs, tests::files};

    #[test]
    fn every_frog_draws_with_two_eyes_and_its_own_colours() {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        for frog in parse_frogs(f.frogs).unwrap() {
            let mut b = Batch::new([0; 3]);
            let pose = Pose {
                pos: V3::ZERO,
                yaw: 0,
                pitch: 0,
                stretch: 0,
                squash: 0,
                wings: None,
                crown: false,
                gape: false,
                scale: 100,
            };
            draw(&mut b, &frog, &pal, &pose);
            let (values, _) = b.finish(crate::render::DYNAMIC_BUFFER);
            let colours: Vec<[i32; 3]> = values
                .chunks(crate::render::INSTANCE_VALUES)
                .map(|v| [v[12], v[13], v[14]])
                .collect();
            let irises = colours.iter().filter(|c| **c == frog.eye).count();
            assert!(irises >= 2, "{}: {irises} irises", frog.id);
            assert!(
                colours.contains(&frog.skin) && colours.contains(&frog.feet),
                "{}",
                frog.id
            );
            if frog.pattern != Pattern::Plain {
                assert!(colours.contains(&frog.mark), "{}: no pattern drawn", frog.id);
            }
        }
    }
}
