//! The survey: a view from high over the hole that a person pans, zooms and
//! turns to plan a shot. It belongs to the camera alone. It moves nothing in
//! the round, so online it never reaches the other browsers.

use crate::balance::*;
use crate::course::Course;
use crate::fx::{self, ONE};
use crate::geom::{facing, right_of, v3, V3};
use crate::round::heading_to;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Survey {
    /// the point on the ground the view looks at
    pub at: V3,
    /// the heading the view faces; "forward" pans this way
    pub yaw: i32,
    /// how high the camera sits over `at`
    pub height: i32,
}

impl Survey {
    /// Opened from a frog: looking at the ground between it and the lotus,
    /// from high enough to see both, facing the lotus.
    pub fn open(c: &Course, from: V3) -> Survey {
        let mid = from.lerp(c.lotus, ONE / 2);
        let apart = v3(c.lotus.x - from.x, 0, c.lotus.z - from.z).len();
        let mut s = Survey {
            at: mid,
            yaw: heading_to(from, c.lotus),
            height: share(apart, SURVEY_FIT_PCT, 100).clamp(SURVEY_FIT_MIN, SURVEY_HIGH),
        };
        s.keep_in(c);
        s
    }

    /// One tick of panning: `right` and `forward` are −1, 0 or 1.
    pub fn pan(&mut self, c: &Course, right: i32, forward: i32) {
        let step = share(self.height, SURVEY_PAN_PERMILLE, 1000);
        let d = right_of(self.yaw)
            .scale(right * ONE)
            .add(facing(self.yaw, 0).scale(forward * ONE));
        self.at = self.at.add(d.scale(step));
        self.keep_in(c);
    }

    /// One tick of the zoom keys: `out` is −1 (in), 0 or 1 (out).
    pub fn zoom_keys(&mut self, out: i32) {
        self.height += out * share(self.height, SURVEY_ZOOM_PERMILLE, 1000);
        self.height = self.height.clamp(SURVEY_LOW, SURVEY_HIGH);
    }

    /// The wheel, in CSS pixels ×100: down (positive) draws back, up closes in,
    /// and a down then the same up come back to the same height.
    pub fn wheel(&mut self, px100: i32) {
        let unit = SURVEY_WHEEL_PX * 100;
        let px = px100.clamp(-unit, unit) as i64;
        let h = self.height as i64;
        let u = unit as i64;
        let next = if px >= 0 {
            h * (u + px) / u
        } else {
            h * u / (u - px)
        };
        self.height = (next as i32).clamp(SURVEY_LOW, SURVEY_HIGH);
    }

    /// Turning the view by dragging.
    pub fn turn(&mut self, by: i32) {
        self.yaw = fx::wrap(self.yaw + by);
    }

    /// Where the camera goes and the point it looks at.
    pub fn eye_look(&self) -> (V3, V3) {
        let back = share(self.height, SURVEY_BACK_PCT, 100);
        let eye = self
            .at
            .sub(facing(self.yaw, 0).scale(back))
            .add(v3(0, self.height, 0));
        (eye, self.at)
    }

    /// The point stays over the hole, on its ground.
    fn keep_in(&mut self, c: &Course) {
        let [x0, z0, x1, z1] = c.def.bounds;
        self.at.x = self.at.x.clamp(x0, x1);
        self.at.z = self.at.z.clamp(z0, z1);
        self.at.y = c.surface(self.at.x, self.at.z);
    }
}

/// `x × n / d` without overflowing on the way.
fn share(x: i32, n: i32, d: i32) -> i32 {
    (x as i64 * n as i64 / d as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_course, Palette};
    use crate::fx::int;

    fn holes() -> Vec<Course> {
        let pal = Palette::parse(include_str!("../../../data/palette.json")).unwrap();
        parse_course(include_str!("../../../data/course.json"), &pal)
            .unwrap()
            .iter()
            .map(|h| Course::build(h, &pal))
            .collect()
    }

    #[test]
    fn opened_at_the_tee_the_survey_sees_the_tee_and_the_lotus() {
        // the survey's whole point: from where it opens, both ends of the
        // hole are on screen, with the frog nearer the bottom
        for c in holes() {
            let s = Survey::open(&c, c.tee);
            let (eye, look) = s.eye_look();
            let cam = crate::render::Camera {
                eye,
                fwd: look.sub(eye).norm(),
                fov: FOV,
            };
            let aspect = ONE * 16 / 9;
            let tee = cam.screen(aspect, c.tee).expect("the tee is in front");
            let lotus = cam.screen(aspect, c.lotus).expect("the lotus is in front");
            for (what, (x, y)) in [("tee", tee), ("lotus", lotus)] {
                let (x, y) = (x / ONE, y / ONE);
                assert!(
                    (0..=100).contains(&x) && (0..=100).contains(&y),
                    "{}: the {what} is off screen at {x}%, {y}%",
                    c.def.name
                );
            }
            assert!(
                tee.1 > lotus.1,
                "{}: the tee is lower on screen than the lotus",
                c.def.name
            );
        }
    }

    #[test]
    fn the_survey_pans_where_it_faces_and_stays_over_the_hole() {
        let c = &holes()[0];
        let mut s = Survey::open(c, c.tee);
        let start = s.at;
        let ahead = facing(s.yaw, 0);
        for _ in 0..30 {
            s.pan(c, 0, 1);
        }
        let moved = s.at.sub(start);
        assert!(moved.dot(ahead) > int(10), "forward moves the view ahead");
        for _ in 0..20_000 {
            s.pan(c, 1, 1);
        }
        let [x0, z0, x1, z1] = c.def.bounds;
        assert!(s.at.x >= x0 && s.at.x <= x1 && s.at.z >= z0 && s.at.z <= z1);
        assert_eq!(s.at.y, c.surface(s.at.x, s.at.z), "the point sits on the ground");
    }

    #[test]
    fn the_wheel_and_the_keys_zoom_within_limits_and_a_wheel_there_and_back_returns() {
        let c = &holes()[0];
        let mut s = Survey::open(c, c.tee);
        // below the top, which would clip the way out
        s.height = int(100);
        let h = s.height;
        s.wheel(12_000);
        assert!(s.height > h, "wheel down draws back");
        s.wheel(-12_000);
        assert!(
            (s.height - h).abs() <= 2,
            "and up comes back: {} vs {h}",
            s.height
        );
        for _ in 0..2000 {
            s.zoom_keys(1);
        }
        assert_eq!(s.height, SURVEY_HIGH);
        for _ in 0..2000 {
            s.zoom_keys(-1);
        }
        assert_eq!(s.height, SURVEY_LOW);
    }
}
