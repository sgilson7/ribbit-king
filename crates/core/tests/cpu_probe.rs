//! A probe, not a gate: how the CPU scores on each hole over a few seeds,
//! printed so the course and the CPU can be tuned against par.
//! `cargo test -p rk_core --test cpu_probe -- --nocapture`

use rk_core::content::{parse_course, Palette};
use rk_core::course::Course;
use rk_core::round::{Input, Phase, Round};

#[test]
fn cpu_scores_against_par() {
    let pal = Palette::parse(include_str!("../../../data/palette.json")).unwrap();
    let holes = parse_course(include_str!("../../../data/course.json"), &pal).unwrap();
    for h in &holes {
        let c = Course::build(h, &pal);
        let mut all = Vec::new();
        for seed in 0..6u64 {
            let mut r = Round::new(vec![(0, true)], seed * 31 + 5);
            r.start_hole(&c);
            let mut notes = Vec::new();
            let mut t = 0;
            while r.phase != Phase::Done && t < 200_000 {
                r.tick(&c, Input::default(), &mut notes);
                t += 1;
            }
            all.push(r.players[0].scores[0]);
            if seed == 0 {
                let says: Vec<_> = notes
                    .iter()
                    .filter_map(|n| match n {
                        rk_core::round::Note::Say(k, _) if *k != "turn" => Some(*k),
                        _ => None,
                    })
                    .collect();
                println!("  {} seed 0 says {:?}", h.id, says);
            }
        }
        println!("{}: par {} cpu {:?}", h.id, h.par, all);
    }
}
