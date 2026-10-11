//! The hole's still scenery, drawn once a hole into the static buffer: the
//! ground mesh, the water or lava, the land beyond the hole, the buildings
//! and every prop. Shapes are placed with `put`, which every drawing file
//! shares.

use crate::balance::cm;
use crate::content::{Palette, Region, Rgb};
use crate::course::Course;
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{shade, Batch, PASS_FACADE, PASS_SOLID};
use crate::rng::Rng;
use crate::terrain::seed_of;

/// A shape at a point, turned to a heading, of a size (each axis, the full
/// width; a mesh is one unit across).
pub fn at(p: V3, yaw: i32, size: V3) -> Affine {
    Affine::translate(p)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::scale(size))
}

/// The same, tilted forward (about its own x) after turning.
pub fn tilted(p: V3, yaw: i32, pitch: i32, size: V3) -> Affine {
    Affine::translate(p)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::rot_x(pitch))
        .then(&Affine::scale(size))
}

/// A cylinder from one point to another, this thick.
pub fn rod(a: V3, b: V3, thick: i32) -> Affine {
    let d = b.sub(a);
    let len = d.len();
    let mid = v3((a.x + b.x) / 2, (a.y + b.y) / 2, (a.z + b.z) / 2);
    if len == 0 {
        return Affine::translate(mid).then(&Affine::scale(v3(thick, thick, thick)));
    }
    // the cylinder's axis is y; looking() points −z along d, so turn y onto −z
    let basis = Affine::looking(d.norm());
    let along = Affine {
        c0: v3(ONE, 0, 0),
        c1: v3(0, 0, -ONE),
        c2: v3(0, ONE, 0),
        t: V3::ZERO,
    };
    Affine::translate(mid)
        .then(&basis)
        .then(&along)
        .then(&Affine::scale(v3(thick, len, thick)))
}

/// A shape, inked or not.
pub fn put(b: &mut Batch, mesh: Mesh, a: &Affine, rgb: Rgb, ink: bool) {
    if ink {
        b.inked(mesh, a, rgb, 0);
    } else {
        b.push(mesh, a, rgb, 0, PASS_SOLID);
    }
}

fn s(size: i32, cm_: i32) -> i32 {
    (crate::balance::cm(cm_) as i64 * size as i64 / 100) as i32
}

/// Everything still on a hole.
pub fn build(c: &Course, pal: &Palette) -> Batch {
    let t = &c.theme;
    let mut b = Batch::new(t.ink);
    b.push(Mesh::Terrain, &Affine::IDENTITY, [ONE; 3], 0, PASS_SOLID);
    let [x0, z0, x1, z1] = c.def.bounds;
    let band = int(700);
    // the land beyond the hole, four strips round it
    let y = c.backdrop;
    for (ax, az, bx, bz) in [
        (x0 - band, z0 - band, x1 + band, z0),
        (x0 - band, z1, x1 + band, z1 + band),
        (x0 - band, z0, x0, z1),
        (x1, z0, x1 + band, z1),
    ] {
        let mid = v3((ax + bx) / 2, y, (az + bz) / 2);
        b.push(
            Mesh::Ground,
            &at(mid, 0, v3(bx - ax, ONE, bz - az)),
            t.far,
            0,
            PASS_SOLID,
        );
    }
    // rivers, ponds and lava: a flat surface wherever each lies; the ground
    // covers it where it is higher
    for w in &c.def.waters {
        let lava = w.kind == crate::content::HazardKind::Lava;
        let (colour, glow) = if lava {
            (pal.c("lava"), ONE * t.hazard_glow.max(60) / 100)
        } else {
            (t.water, 0)
        };
        let bank = crate::terrain::BANK * 2;
        match w.region {
            Region::All => {
                let mid = v3((x0 + x1) / 2, w.level, (z0 + z1) / 2);
                b.push(
                    Mesh::Ground,
                    &at(mid, 0, v3(x1 - x0, ONE, z1 - z0)),
                    colour,
                    glow,
                    PASS_SOLID,
                );
            }
            Region::Line { a, b: bb, w: wide } => {
                let d = v3(bb.0 - a.0, 0, bb.1 - a.1);
                let yaw = crate::geom::yaw_pitch_of(d).0;
                let mid = v3((a.0 + bb.0) / 2, w.level, (a.1 + bb.1) / 2);
                b.push(
                    Mesh::Ground,
                    &at(mid, yaw, v3(wide + ONE * 3, ONE, d.len() + wide / 2)),
                    colour,
                    glow,
                    PASS_SOLID,
                );
            }
            Region::Disc { x, z, r } => {
                b.push(
                    Mesh::Cyl,
                    &at(
                        v3(x, w.level - cm(5), z),
                        0,
                        v3(2 * r + bank, cm(10), 2 * r + bank),
                    ),
                    colour,
                    glow,
                    PASS_SOLID,
                );
            }
        }
    }
    for f in &c.def.falls {
        // lava falls as lava: the pool at its foot says which
        let lava = crate::terrain::water_at(&c.def, f.x, f.z, f.bottom - ONE)
            .map(|w| w.kind == crate::content::HazardKind::Lava)
            .unwrap_or(false);
        waterfall(&mut b, pal, if lava { pal.c("lava") } else { t.water }, f);
    }
    for r in &c.roofs {
        rock_bridge(&mut b, pal, t, r);
    }
    // the tee: a mat
    put(
        &mut b,
        Mesh::Cube,
        &at(c.tee.add(v3(0, ONE / 20, 0)), 0, v3(int(4), ONE / 8, int(4))),
        shade(t.fairway, 85),
        true,
    );
    for (bx, wall) in &c.buildings {
        building(&mut b, pal, bx.min, bx.max, *wall, c);
    }
    for p in &c.placed {
        prop(&mut b, pal, c, &p.kind, p.at, p.size, fx::deg(p.yaw));
    }
    b
}

/// A sheet of falling water over a cliff, with foam at its foot and stripes
/// of light down it.
fn waterfall(b: &mut Batch, pal: &Palette, water: Rgb, f: &crate::content::Fall) {
    let h = (f.top - f.bottom).max(ONE);
    let yaw = deg(f.yaw);
    let mid = v3(f.x, (f.top + f.bottom) / 2, f.z);
    b.push(
        Mesh::Cube,
        &at(mid, yaw, v3(f.w, h, cm(60))),
        water,
        ONE / 5,
        PASS_SOLID,
    );
    let across = crate::geom::right_of(yaw);
    let front = facing(yaw, 0).scale(cm(35));
    let n = (f.w / cm(80)).max(2);
    for k in 0..n {
        let off = across.scale(f.w * (2 * k + 1 - n) / (2 * n));
        let stripe_h = h * (40 + (k * 37) % 50) / 100;
        let top = v3(mid.x, f.top - (k * 53 % 30) * h / 100, mid.z)
            .add(off)
            .add(front);
        b.push(
            Mesh::Cube,
            &at(top.sub(v3(0, stripe_h / 2, 0)), yaw, v3(cm(18), stripe_h, cm(8))),
            pal.c("water_jet"),
            ONE / 2,
            PASS_SOLID,
        );
    }
    for k in 0..5 {
        let off = across.scale(f.w * (k - 2) / 5);
        let s = cm(160) + k % 2 * cm(70);
        b.push(
            Mesh::Blob,
            &at(
                v3(f.x, f.bottom + cm(30), f.z).add(off).add(front.scale(ONE * 3)),
                0,
                v3(s, s * 2 / 3, s),
            ),
            pal.c("splash"),
            ONE / 3,
            PASS_SOLID,
        );
    }
}

/// A tunnel's roof: a slab of rock over the cut, with arch stones at each mouth.
fn rock_bridge(b: &mut Batch, pal: &Palette, t: &crate::content::Theme, r: &crate::course::Roof) {
    let size = r.max.sub(r.min);
    let mid = v3(
        (r.min.x + r.max.x) / 2,
        (r.min.y + r.max.y) / 2,
        (r.min.z + r.max.z) / 2,
    );
    put(b, Mesh::Cube, &at(mid, 0, size), t.rock, true);
    // the underside, in shadow
    b.push(
        Mesh::Cube,
        &at(
            v3(mid.x, r.min.y + cm(5), mid.z),
            0,
            v3(size.x - cm(40), cm(12), size.z - cm(20)),
        ),
        shade(t.rock, 55),
        0,
        PASS_SOLID,
    );
    for z in [r.min.z, r.max.z] {
        let n = 7;
        for k in 0..n {
            let x = r.min.x + size.x * k / (n - 1);
            let stone = v3(x, r.min.y + cm(40), z);
            put(
                b,
                Mesh::Blob,
                &at(stone, deg(k * 40), v3(cm(180), cm(140), cm(160))),
                shade(t.rock, 85 + (k % 3) * 8),
                true,
            );
        }
        // pillars down each side of the mouth, to the floor
        for x in [r.min.x + cm(70), r.max.x - cm(70)] {
            let hgt = r.min.y - r.floor;
            put(
                b,
                Mesh::Cube,
                &at(v3(x, r.floor + hgt / 2, z), 0, v3(cm(140), hgt, cm(160))),
                shade(t.rock, 75),
                true,
            );
        }
    }
    let _ = pal;
}

fn building(b: &mut Batch, pal: &Palette, min: V3, max: V3, wall: Rgb, c: &Course) {
    let size = max.sub(min);
    let mid = v3((min.x + max.x) / 2, (min.y + max.y) / 2, (min.z + max.z) / 2);
    put(b, Mesh::Cube, &at(mid, 0, size), wall, true);
    // a cornice
    let top = v3(mid.x, max.y + ONE / 8, mid.z);
    put(
        b,
        Mesh::Cube,
        &at(top, 0, v3(size.x + ONE / 3, ONE / 4, size.z + ONE / 3)),
        shade(wall, 80),
        true,
    );
    // windows on every face, from the first storey up
    let seed = (seed_of(&c.def.id) as i32 ^ min.x ^ min.z) & 0xFF;
    let ground_floor = int(3);
    let up = v3(0, ONE, 0);
    let h = size.y - ground_floor - ONE;
    if h > ONE {
        let cy = min.y + ground_floor + h / 2;
        for (n, w, centre) in [
            (v3(ONE, 0, 0), size.z, v3(max.x, cy, mid.z)),
            (v3(-ONE, 0, 0), size.z, v3(min.x, cy, mid.z)),
            (v3(0, 0, ONE), size.x, v3(mid.x, cy, max.z)),
            (v3(0, 0, -ONE), size.x, v3(mid.x, cy, min.z)),
        ] {
            let along = up.cross(n).scale(w - ONE);
            let face = Affine {
                c0: up.scale(h),
                c1: n.scale(ONE / 10),
                c2: along,
                t: centre.add(n.scale(ONE / 30)),
            };
            b.push(
                Mesh::Facade,
                &face,
                pal.c("window"),
                seed * ONE / 255,
                PASS_FACADE,
            );
        }
    }
    // shopfront awnings on the faces toward the street (x = 0)
    let street_x = if mid.x > 0 { min.x } else { max.x };
    let out = if mid.x > 0 { -ONE } else { ONE };
    let awning = if (seed & 1) == 0 {
        pal.c("awning_a")
    } else {
        pal.c("awning_b")
    };
    if size.z > int(4) && size.x < int(30) {
        let p = v3(street_x + out * 3 / 4, min.y + ONE + int(3), mid.z);
        let yaw = if out > 0 { -fx::QUARTER } else { fx::QUARTER };
        put(
            b,
            Mesh::Wedge,
            &at(p, yaw, v3(size.z - int(2), ONE / 2, int(3) / 2)),
            awning,
            true,
        );
    }
    // rooftop clutter
    let mut rng = Rng::new(seed as u64 + 7);
    for _ in 0..rng.range(1, 3) {
        let px = rng.range((min.x + ONE) / ONE, (max.x - ONE) / ONE) * ONE;
        let pz = rng.range((min.z + ONE) / ONE, (max.z - ONE) / ONE) * ONE;
        let roof = max.y + ONE / 4;
        if rng.chance(55) {
            put(
                b,
                Mesh::Cube,
                &at(v3(px, roof + ONE / 2, pz), 0, v3(int(2), ONE, ONE * 3 / 2)),
                pal.c("roof_box"),
                true,
            );
        } else {
            for k in [-1, 1] {
                put(
                    b,
                    Mesh::Cyl,
                    &at(
                        v3(px + k * ONE / 2, roof + ONE / 2, pz),
                        0,
                        v3(ONE / 8, ONE, ONE / 8),
                    ),
                    pal.c("wood_dark"),
                    false,
                );
            }
            put(
                b,
                Mesh::Cyl,
                &at(v3(px, roof + int(2), pz), 0, v3(int(2), int(2), int(2))),
                pal.c("wood"),
                true,
            );
            put(
                b,
                Mesh::Cone,
                &at(
                    v3(px, roof + int(3) + ONE / 4, pz),
                    0,
                    v3(int(2), ONE / 2, int(2)),
                ),
                shade(wall, 75),
                true,
            );
        }
    }
    // a sign high on the street side
    if size.y > int(18) {
        let colour = [pal.c("sign_a"), pal.c("sign_b"), pal.c("sign_c")][(seed % 3) as usize];
        let p = v3(street_x + out / 2, max.y - int(4), mid.z + (seed % 5 - 2) * ONE);
        put(
            b,
            Mesh::Cube,
            &at(p, 0, v3(ONE / 3, int(4), int(2))),
            colour,
            true,
        );
    }
}

fn prop(b: &mut Batch, pal: &Palette, c: &Course, kind: &str, p: V3, size: i32, yaw: i32) {
    let t = &c.theme;
    let g = |k: usize| t.greens[k % t.greens.len()];
    let sz = |x: i32| s(size, x);
    match kind {
        "tree" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(160), 0)), yaw, v3(sz(50), sz(320), sz(50))),
                pal.c("trunk"),
                true,
            );
            for (k, (dx, dy, r)) in [(-60, 330, 220), (70, 360, 240), (0, 470, 200)]
                .into_iter()
                .enumerate()
            {
                put(
                    b,
                    Mesh::Blob,
                    &at(
                        p.add(v3(sz(dx), sz(dy), sz(dx / 2))),
                        yaw,
                        v3(sz(r), sz(r) * 4 / 5, sz(r)),
                    ),
                    g(k),
                    true,
                );
            }
        }
        "cypress" => {
            put(
                b,
                Mesh::Cone,
                &at(p.add(v3(0, sz(70), 0)), yaw, v3(sz(170), sz(140), sz(170))),
                pal.c("bark_dark"),
                true,
            );
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(280), 0)), yaw, v3(sz(70), sz(520), sz(70))),
                pal.c("trunk"),
                true,
            );
            for (k, (dy, r, hh)) in [(470, 300, 160), (580, 240, 150), (680, 170, 130)]
                .into_iter()
                .enumerate()
            {
                put(
                    b,
                    Mesh::Blob,
                    &at(
                        p.add(v3(0, sz(dy), 0)),
                        yaw + deg(30) * k as i32,
                        v3(sz(r), sz(hh), sz(r)),
                    ),
                    g(k + 2),
                    true,
                );
            }
            for k in 0..5 {
                let a = yaw + deg(72) * k;
                let off = crate::geom::facing(a, 0).scale(sz(120));
                put(
                    b,
                    Mesh::Cube,
                    &at(p.add(off).add(v3(0, sz(380), 0)), a, v3(sz(14), sz(150), sz(14))),
                    pal.c("moss"),
                    false,
                );
            }
        }
        "reeds" => {
            let mut rng = Rng::new((p.x ^ p.z) as u64);
            for _ in 0..7 {
                let dx = rng.range(-60, 60);
                let dz = rng.range(-60, 60);
                let h = rng.range(140, 230);
                let base = p.add(v3(sz(dx), 0, sz(dz)));
                put(
                    b,
                    Mesh::Cyl,
                    &at(base.add(v3(0, sz(h) / 2, 0)), 0, v3(sz(7), sz(h), sz(7))),
                    pal.c("reed"),
                    false,
                );
                if rng.chance(60) {
                    put(
                        b,
                        Mesh::Cyl,
                        &at(base.add(v3(0, sz(h) - sz(20), 0)), 0, v3(sz(16), sz(45), sz(16))),
                        pal.c("cattail"),
                        true,
                    );
                }
            }
        }
        "log" => {
            let a = Affine::translate(p.add(v3(0, sz(45), 0)))
                .then(&Affine::rot_y(yaw))
                .then(&Affine::rot_z(fx::QUARTER))
                .then(&Affine::scale(v3(sz(90), sz(500), sz(90))));
            put(b, Mesh::Cyl, &a, pal.c("wood"), true);
        }
        "rock" => put(
            b,
            Mesh::Blob,
            &at(p.add(v3(0, sz(40), 0)), yaw, v3(sz(230), sz(150), sz(200))),
            pal.c("rock"),
            true,
        ),
        "boulder" => put(
            b,
            Mesh::Blob,
            &at(p.add(v3(0, sz(80), 0)), yaw, v3(sz(500), sz(330), sz(450))),
            pal.c("rock_dark"),
            true,
        ),
        "cactus" => {
            let col = pal.c("cactus");
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(230), 0)), yaw, v3(sz(90), sz(460), sz(90))),
                col,
                true,
            );
            put(
                b,
                Mesh::Sphere,
                &at(p.add(v3(0, sz(460), 0)), yaw, v3(sz(90), sz(90), sz(90))),
                col,
                true,
            );
            for (side, hgt, reach) in [(1, 190, 260), (-1, 250, 320)] {
                let r = crate::geom::right_of(yaw).scale(side);
                let elbow = p.add(r.scale(sz(75))).add(v3(0, sz(hgt), 0));
                put(
                    b,
                    Mesh::Cube,
                    &at(elbow, yaw, v3(sz(110), sz(60), sz(60))),
                    col,
                    true,
                );
                let up = p
                    .add(r.scale(sz(110)))
                    .add(v3(0, sz(hgt) + sz(reach - hgt + 120) / 2, 0));
                put(
                    b,
                    Mesh::Cyl,
                    &at(up, yaw, v3(sz(60), sz(reach - hgt + 140), sz(60))),
                    pal.c("cactus_dark"),
                    true,
                );
            }
        }
        "palm" => {
            let lean = crate::geom::facing(yaw, 0);
            let mut top = p;
            for k in 0..5 {
                let next = top.add(v3(0, sz(110), 0)).add(lean.scale(sz(12 + k * 10)));
                b.inked(
                    Mesh::Cyl,
                    &rod(top, next, sz(55 - k * 4)),
                    if k % 2 == 0 {
                        pal.c("wood")
                    } else {
                        pal.c("wood_light")
                    },
                    0,
                );
                top = next;
            }
            for k in 0..7 {
                let a = yaw + deg(51) * k;
                let leaf = tilted(
                    top.add(crate::geom::facing(a, 0).scale(sz(150))),
                    a,
                    -deg(25),
                    v3(sz(70), sz(18), sz(330)),
                );
                put(b, Mesh::Blob, &leaf, pal.c("palm_leaf"), true);
            }
            for k in 0..3 {
                let off = crate::geom::facing(yaw + deg(120) * k, 0).scale(sz(30));
                put(
                    b,
                    Mesh::Sphere,
                    &at(top.add(off).sub(v3(0, sz(25), 0)), 0, v3(sz(30), sz(30), sz(30))),
                    pal.c("wood_dark"),
                    true,
                );
            }
        }
        "lamp" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(330), 0)), 0, v3(sz(22), sz(660), sz(22))),
                pal.c("lamp_post"),
                true,
            );
            let toward = if p.x > 0 { -ONE } else { ONE };
            let head = p.add(v3(toward * 3 / 4, sz(640), 0));
            put(
                b,
                Mesh::Cube,
                &at(
                    p.add(v3(toward * 3 / 8, sz(650), 0)),
                    0,
                    v3(ONE * 3 / 4, sz(10), sz(10)),
                ),
                pal.c("lamp_post"),
                false,
            );
            b.inked(
                Mesh::Sphere,
                &at(head, 0, v3(sz(50), sz(40), sz(50))),
                pal.c("glow_lamp"),
                ONE * 6 / 10,
            );
        }
        "basalt" => {
            let mut rng = Rng::new((p.x ^ (p.z << 3)) as u64);
            for _ in 0..rng.range(3, 5) {
                let dx = rng.range(-110, 110);
                let dz = rng.range(-110, 110);
                let h = rng.range(140, 330);
                let q = p.add(v3(sz(dx), sz(h) / 2, sz(dz)));
                put(
                    b,
                    Mesh::Cyl,
                    &at(q, yaw, v3(sz(120), sz(h), sz(120))),
                    pal.c("basalt"),
                    true,
                );
                put(
                    b,
                    Mesh::Cyl,
                    &at(q.add(v3(0, sz(h) / 2, 0)), yaw, v3(sz(100), sz(8), sz(100))),
                    pal.c("basalt_top"),
                    false,
                );
            }
        }
        "bench" => {
            put(
                b,
                Mesh::Cube,
                &at(p.add(v3(0, sz(45), 0)), 0, v3(sz(180), sz(12), sz(55))),
                pal.c("bench"),
                true,
            );
            put(
                b,
                Mesh::Cube,
                &at(p.add(v3(0, sz(80), sz(25))), 0, v3(sz(180), sz(45), sz(8))),
                pal.c("bench"),
                true,
            );
            for k in [-1, 1] {
                put(
                    b,
                    Mesh::Cube,
                    &at(p.add(v3(sz(75) * k, sz(22), 0)), 0, v3(sz(10), sz(45), sz(50))),
                    pal.c("lamp_post"),
                    false,
                );
            }
        }
        "hydrant" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(35), 0)), 0, v3(sz(40), sz(70), sz(40))),
                pal.c("hydrant"),
                true,
            );
            put(
                b,
                Mesh::Sphere,
                &at(p.add(v3(0, sz(72), 0)), 0, v3(sz(40), sz(36), sz(40))),
                pal.c("hydrant"),
                true,
            );
        }
        "planter" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(35), 0)), 0, v3(sz(220), sz(70), sz(220))),
                pal.c("planter"),
                true,
            );
            put(
                b,
                Mesh::Blob,
                &at(p.add(v3(0, sz(110), 0)), yaw, v3(sz(200), sz(140), sz(200))),
                g(0),
                true,
            );
        }
        "vent" => {
            put(
                b,
                Mesh::Cone,
                &at(p.add(v3(0, sz(60), 0)), yaw, v3(sz(300), sz(160), sz(300))),
                pal.c("basalt"),
                true,
            );
            b.push(
                Mesh::Cyl,
                &at(p.add(v3(0, sz(122), 0)), 0, v3(sz(90), sz(10), sz(90))),
                pal.c("lava_hot"),
                ONE,
                PASS_SOLID,
            );
            for k in 0..3 {
                let q = p.add(v3(sz(20) * k, sz(300 + k * 170), -sz(30) * k));
                put(
                    b,
                    Mesh::Blob,
                    &at(q, yaw, v3(sz(160 + k * 60), sz(120 + k * 40), sz(160 + k * 60))),
                    shade(pal.c("smoke"), 110 + k * 8),
                    false,
                );
            }
        }
        "dune_grass" => {
            for k in 0..6 {
                let a = yaw + deg(60) * k;
                let q = p.add(crate::geom::facing(a, 0).scale(sz(18)));
                put(
                    b,
                    Mesh::Wedge,
                    &tilted(q.add(v3(0, sz(35), 0)), a, deg(15), v3(sz(14), sz(75), sz(10))),
                    g(k as usize),
                    false,
                );
            }
        }
        "skull" => {
            put(
                b,
                Mesh::Blob,
                &at(p.add(v3(0, sz(25), 0)), yaw, v3(sz(70), sz(50), sz(90))),
                pal.c("bone"),
                true,
            );
            for k in [-1, 1] {
                let r = crate::geom::right_of(yaw).scale(k * sz(70));
                put(
                    b,
                    Mesh::Cone,
                    &tilted(p.add(r).add(v3(0, sz(55), 0)), yaw, 0, v3(sz(18), sz(70), sz(18))),
                    pal.c("bone"),
                    true,
                );
            }
        }
        "volcano" => {
            let w = sz(42000);
            let h = sz(16000);
            put(
                b,
                Mesh::Cone,
                &at(p.add(v3(0, h / 2 - int(10), 0)), yaw, v3(w, h, w)),
                pal.c("volcano"),
                true,
            );
            b.push(
                Mesh::Cyl,
                &at(
                    p.add(v3(0, h * 82 / 100, 0)),
                    0,
                    v3(w * 18 / 100, sz(400), w * 18 / 100),
                ),
                pal.c("lava"),
                ONE * 8 / 10,
                PASS_SOLID,
            );
            for k in 0..4 {
                let a = yaw + deg(80) * k + deg(20);
                let dir = crate::geom::facing(a, 0);
                let from = p.add(v3(0, h * 80 / 100, 0)).add(dir.scale(w * 9 / 100));
                let to = p.add(v3(0, h * 25 / 100, 0)).add(dir.scale(w * 30 / 100));
                b.push(
                    Mesh::Cyl,
                    &rod(from, to, sz(900)),
                    pal.c("lava_hot"),
                    ONE * 7 / 10,
                    PASS_SOLID,
                );
            }
            for k in 0..4 {
                let q = p.add(v3(sz(900) * k, h + sz(1500) + sz(1800) * k, -sz(500) * k));
                put(
                    b,
                    Mesh::Blob,
                    &at(
                        q,
                        yaw,
                        v3(sz(5000 + 1500 * k), sz(2600 + 600 * k), sz(5000 + 1500 * k)),
                    ),
                    shade(pal.c("smoke"), 100 + 10 * k),
                    false,
                );
            }
        }
        "tower" => {
            let mut rng = Rng::new((p.x as u64) << 7 ^ p.z as u64);
            let wall = if t.walls.is_empty() {
                t.rock
            } else {
                t.walls[rng.range(0, t.walls.len() as i32 - 1) as usize]
            };
            let w = sz(1400) + rng.range(0, 8) * ONE;
            let d = sz(1400) + rng.range(0, 8) * ONE;
            let h = rng.range(20, 70) * ONE;
            building(
                b,
                pal,
                v3(p.x - w / 2, p.y, p.z - d / 2),
                v3(p.x + w / 2, p.y + h, p.z + d / 2),
                wall,
                c,
            );
        }
        "fountain" => {
            put(
                b,
                Mesh::Ring,
                &at(p.add(v3(0, sz(20), 0)), 0, v3(sz(1180), sz(90), sz(1180))),
                pal.c("fountain"),
                true,
            );
            put(
                b,
                Mesh::Ring,
                &at(p.add(v3(0, sz(10), 0)), 0, v3(sz(1260), sz(60), sz(1260))),
                shade(pal.c("fountain"), 85),
                true,
            );
        }
        "toadstool" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(35), 0)), 0, v3(sz(30), sz(70), sz(30))),
                pal.c("mushroom_stem"),
                true,
            );
            put(
                b,
                Mesh::Sphere,
                &at(p.add(v3(0, sz(75), 0)), yaw, v3(sz(110), sz(60), sz(110))),
                pal.c("toadstool_cap"),
                true,
            );
            for k in 0..4 {
                let q = p
                    .add(facing(yaw + deg(90) * k, 0).scale(sz(30)))
                    .add(v3(0, sz(95), 0));
                put(
                    b,
                    Mesh::Sphere,
                    &at(q, 0, v3(sz(20), sz(10), sz(20))),
                    pal.c("mushroom_spot"),
                    false,
                );
            }
        }
        "glowcap" => {
            let mut rng = Rng::new((p.x ^ (p.z << 2)) as u64);
            for _ in 0..5 {
                let q = p.add(v3(sz(rng.range(-80, 80)), 0, sz(rng.range(-80, 80))));
                let h = sz(rng.range(30, 80));
                let colour = if rng.chance(50) {
                    pal.c("glowcap")
                } else {
                    pal.c("glowcap_b")
                };
                put(
                    b,
                    Mesh::Cyl,
                    &at(q.add(v3(0, h / 2, 0)), 0, v3(sz(10), h, sz(10))),
                    pal.c("mushroom_stem"),
                    false,
                );
                b.inked(
                    Mesh::Sphere,
                    &at(q.add(v3(0, h, 0)), 0, v3(sz(40), sz(22), sz(40))),
                    colour,
                    ONE * 6 / 10,
                );
            }
        }
        "puffball" => {
            for (dx, s2) in [(0, 120), (90, 70), (-70, 55)] {
                let q = p.add(crate::geom::right_of(yaw).scale(sz(dx)));
                put(
                    b,
                    Mesh::Sphere,
                    &at(
                        q.add(v3(0, sz(s2) * 4 / 10, 0)),
                        0,
                        v3(sz(s2), sz(s2) * 85 / 100, sz(s2)),
                    ),
                    pal.c("puffball"),
                    true,
                );
            }
        }
        "shelf_tree" => {
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(450), 0)), yaw, v3(sz(110), sz(900), sz(110))),
                pal.c("bark_dark"),
                true,
            );
            for k in 0..5 {
                let a = yaw + deg(130) * k;
                let q = p.add(facing(a, 0).scale(sz(70))).add(v3(0, sz(150 + k * 140), 0));
                put(
                    b,
                    Mesh::Cyl,
                    &at(q, a, v3(sz(150), sz(25), sz(150))),
                    pal.c("shelf"),
                    true,
                );
            }
            put(
                b,
                Mesh::Blob,
                &at(p.add(v3(0, sz(950), 0)), yaw, v3(sz(520), sz(320), sz(520))),
                g(1),
                true,
            );
        }
        "fungus_tree" => {
            // a giant mushroom that is only scenery: violet, out of play
            put(
                b,
                Mesh::Cyl,
                &at(p.add(v3(0, sz(600), 0)), yaw, v3(sz(160), sz(1200), sz(160))),
                pal.c("bumper_stem"),
                true,
            );
            put(
                b,
                Mesh::Sphere,
                &at(p.add(v3(0, sz(1250), 0)), yaw, v3(sz(1100), sz(600), sz(1100))),
                pal.c("glowcap_b"),
                true,
            );
        }
        _ => {}
    }
}
