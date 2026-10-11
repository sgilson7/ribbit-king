//! Feel numbers: every speed, size and timing the game plays by, in one
//! place. Written in the units a person thinks in (metres, seconds, degrees)
//! and converted to Q16 per-tick values here.

use crate::fx::{self, ONE};

pub const TICKS_PER_SECOND: i32 = 60;
/// One tick of the page's clock, in microseconds.
pub const TICK_US: i64 = 1_000_000 / TICKS_PER_SECOND as i64;
/// A frame later than this (a hidden tab) is cut short rather than caught up.
pub const MAX_CATCH_UP_US: i64 = 250_000;

/// Metres per second to Q16 metres per tick.
pub const fn mps(cm_per_s: i32) -> i32 {
    ((cm_per_s as i64 * ONE as i64) / (100 * TICKS_PER_SECOND as i64)) as i32
}

/// Metres per second squared (in cm/s²) to Q16 metres per tick².
pub const fn mps2(cm_per_s2: i32) -> i32 {
    ((cm_per_s2 as i64 * ONE as i64) / (100 * (TICKS_PER_SECOND * TICKS_PER_SECOND) as i64)) as i32
}

/// Seconds (in ms) to ticks.
pub const fn secs(ms: i32) -> i32 {
    ms * TICKS_PER_SECOND / 1000
}

/// Centimetres to Q16 metres.
pub const fn cm(x: i32) -> i32 {
    ((x as i64 * ONE as i64) / 100) as i32
}

// ---- the frog in the air --------------------------------------------------

/// The frog is a ball this wide for every collision.
pub const FROG_RADIUS: i32 = cm(45);
pub const GRAVITY: i32 = mps2(1800);
/// A whack at full power on a perfect hit leaves the catapult this fast.
pub const LAUNCH_MAX: i32 = mps(3900);
/// A perfect hit adds this share of the power on top.
pub const PERFECT_BONUS_PCT: i32 = 8;
/// Air drag: the share of speed kept each tick, out of 10 000.
pub const AIR_KEEP: i32 = 9_992;
/// Steering in the air: sideways and along, never up.
pub const AIR_ACCEL: i32 = mps2(700);
/// Ground bounce: share of the speed into the ground given back, out of 100.
pub const GROUND_BOUNCE_PCT: i32 = 32;
/// Ground friction: share of the speed along the ground lost per touching tick, out of 100.
pub const GROUND_FRICTION_PCT: i32 = 9;
/// Slower than this on the ground for REST_TICKS and the frog has stopped.
pub const REST_SPEED: i32 = mps(110);
pub const REST_TICKS: i32 = 14;
/// A shot that has not stopped after this long is stopped where it is.
pub const FLIGHT_LIMIT: i32 = secs(25_000);
/// A wall gives back this share of the speed into it, out of 100.
pub const WALL_BOUNCE_PCT: i32 = 45;

// ---- things on the course -------------------------------------------------

/// A lily pad throws a frog back up at least this fast.
pub const PAD_MIN: i32 = mps(1500);
pub const PAD_KEEP_PCT: i32 = 85;
/// A spider web throws harder than a lily pad.
pub const WEB_MIN: i32 = mps(2200);
pub const WEB_KEEP_PCT: i32 = 95;
/// An explosive mushroom: how far its blast reaches and how hard it throws.
pub const MUSHROOM_REACH: i32 = cm(150);
pub const MUSHROOM_UP: i32 = mps(2400);
pub const MUSHROOM_OUT: i32 = mps(1500);
/// Power-ups and flies are picked up within this distance of the frog's centre.
pub const PICKUP_REACH: i32 = cm(150);
pub const FLY_REACH: i32 = cm(260);
/// The lotus cup: inside this radius and height, the frog is in.
pub const LOTUS_CUP_RADIUS: i32 = cm(170);
pub const LOTUS_CUP_HEIGHT: i32 = cm(180);
/// A car's bump: up, on top of its own speed.
pub const CAR_BUMP_UP: i32 = mps(900);

// ---- power-ups -------------------------------------------------------------

/// An extra jump in the air: at least this fast upward, and a little forward.
pub const JUMP_UP: i32 = mps(1450);
pub const JUMP_FORWARD: i32 = mps(300);
pub const JUMPS_MAX: i32 = 3;
/// The wings: how long they last, and the glide they fly.
pub const WINGS_TICKS: i32 = secs(6_500);
pub const GLIDE_MIN: i32 = mps(1300);
pub const GLIDE_MAX: i32 = mps(4200);
pub const GLIDE_START: i32 = mps(2000);
pub const GLIDE_TURN: i32 = fx::deg(95) / TICKS_PER_SECOND;
pub const GLIDE_PITCH_RATE: i32 = fx::deg(70) / TICKS_PER_SECOND;
pub const GLIDE_PITCH_LEVEL: i32 = -fx::deg(7);
pub const GLIDE_PITCH_DIVE: i32 = -fx::deg(38);
pub const GLIDE_PITCH_CLIMB: i32 = fx::deg(24);
/// Speed lost to drag every tick of a glide.
pub const GLIDE_DRAG: i32 = mps2(160);
/// Gravity's share in a glide, out of 100, for speed traded with height.
pub const GLIDE_GRAVITY_PCT: i32 = 100;

// ---- flies and the tongue -------------------------------------------------

pub const FLIES_FOR_TONGUE: i32 = 10;
pub const TONGUE_REACH: i32 = cm(1100);
pub const TONGUE_TICKS: i32 = secs(320);
pub const TONGUE_OUT: i32 = mps(1700);
pub const TONGUE_UP: i32 = mps(1100);

// ---- the swing ------------------------------------------------------------

/// Aim: degrees a second while A or D is held.
pub const AIM_TURN: i32 = fx::deg(70) / TICKS_PER_SECOND;
/// The power setting, in percent, and its step.
pub const POWER_MIN: i32 = 10;
pub const POWER_MAX: i32 = 100;
pub const POWER_STEP: i32 = 5;
/// W or S held this long steps the power again.
pub const POWER_REPEAT: i32 = 6;
/// The lofts: low, middle, high.
pub const LOFTS: [i32; 3] = [fx::deg(28), fx::deg(42), fx::deg(58)];
pub const LOFT_DEFAULT: usize = 1;
/// The meter's needle goes round once in this many ticks, clockwise from twelve.
pub const METER_LAP: i32 = secs(1_300);
/// The power bar runs from BAR_FROM to BAR_TO degrees round the dial; its
/// middle is the perfect shot, and PERFECT_HALF either side of it counts.
pub const BAR_FROM: i32 = 105;
pub const BAR_TO: i32 = 255;
pub const PERFECT_HALF: i32 = 7;
/// Inside the bar, power falls from 100% at the middle to this at the ends.
pub const BAR_EDGE_PCT: i32 = 72;
/// Outside it, a whiff: this share of the power.
pub const WHIFF_PCT: i32 = 30;
/// Off the middle, the shot hooks (early) or slices (late) by up to this.
pub const SLICE_MAX: i32 = fx::deg(11);
pub const WHIFF_SLICE: i32 = fx::deg(18);
/// The mallet: rising, then falling, then the frog is away.
pub const SWING_TICKS: i32 = secs(520);
pub const SWING_HIT: i32 = secs(420);

// ---- turns ----------------------------------------------------------------

/// After every frog has stopped, a pause before the next turn.
pub const SETTLE_TICKS: i32 = secs(1_100);
pub const PENALTY_TICKS: i32 = secs(1_700);
pub const HOLED_TICKS: i32 = secs(2_400);
/// A hole is given up at this many strokes over par, and scored at it.
pub const PICKUP_OVER_PAR: i32 = 5;
/// The CPU: how long it looks before aiming, and before it swings.
pub const CPU_THINK: i32 = secs(900);
pub const CPU_SETTLE: i32 = secs(500);
/// The CPU's miss on the meter, in degrees, at most, from the middle.
pub const CPU_MISS: i32 = 22;

// ---- camera ---------------------------------------------------------------

pub const FOV: i32 = fx::deg(62);
pub const CAM_AIM_BACK: i32 = cm(820);
pub const CAM_AIM_UP: i32 = cm(380);
pub const CAM_FLY_BACK: i32 = cm(1050);
pub const CAM_FLY_UP: i32 = cm(420);
/// The share of the way to its goal the camera moves each tick, out of 100.
pub const CAM_FOLLOW_PCT: i32 = 9;
/// Dragging the view: angle units turned per CSS pixel dragged.
pub const DRAG_TURN: i32 = 64;
/// How quickly the camera turns to a new aim, out of 100 a tick.
pub const CAM_AIM_PCT: i32 = 25;
pub const INTRO_TICKS: i32 = secs(5_200);
