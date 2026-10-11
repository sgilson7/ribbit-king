//! A probe, not a gate: how the CPU scores on each hole over a few seeds,
//! and how long building a hole and one CPU decision take, printed so the
//! courses and the CPU can be tuned against par.
//! `cargo test -p rk_core --release --test cpu_probe -- --nocapture`

use rk_core::content::{parse_course, Palette};
use rk_core::course::Course;
use rk_core::round::{Input, Phase, Round};
use std::time::Instant;

#[test]
fn cpu_scores_against_par() {
    let pal = Palette::parse(include_str!("../../../data/palette.json")).unwrap();
    let holes = parse_course(include_str!("../../../data/course.json"), &pal).unwrap();
    for h in &holes {
        let t0 = Instant::now();
        let c = Course::build(h, &pal);
        let built = t0.elapsed();
        let mesh = Instant::now();
        let m = c.terrain.mesh(&c.def, &c.theme);
        let meshed = mesh.elapsed();
        let mut all = Vec::new();
        let mut worst_tick = std::time::Duration::ZERO;
        for seed in 0..4u64 {
            let mut r = Round::new(vec![(0, true)], seed * 31 + 5);
            r.start_hole(&c);
            let mut notes = Vec::new();
            let mut t = 0;
            while r.phase != Phase::Done && t < 200_000 {
                let s = Instant::now();
                r.tick(&c, Input::default(), &mut notes);
                worst_tick = worst_tick.max(s.elapsed());
                t += 1;
            }
            all.push(r.players[0].scores[0]);
        }
        println!(
            "{}: par {} cpu {:?}; build {:?}, mesh {:?} ({} floats), slowest tick {:?}, columns {} boxes {}",
            h.id,
            h.par,
            all,
            built,
            meshed,
            m.len(),
            worst_tick,
            c.columns.len(),
            c.boxes.len()
        );
    }
}
