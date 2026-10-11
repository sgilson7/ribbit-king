//! rk_core: every rule of Ribbit King.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
#![allow(clippy::needless_range_loop, clippy::collapsible_match)]
// the HUD is one large json! literal
#![recursion_limit = "256"]

pub mod balance;
pub mod body;
pub mod content;
pub mod course;
pub mod draw;
pub mod frog;
pub mod fx;
pub mod game;
pub mod geom;
pub mod mesh;
pub mod render;
pub mod rng;
pub mod round;
pub mod scenery;
pub mod terrain;
