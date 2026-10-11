//! rk_shim: moves values between the page and `rk_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments or result. The core speaks
//! Q16.16 integers; WebGL wants `f32`, so values on the way out are divided by
//! the core's own `ONE`, and the page's float clock and mouse movement are
//! turned into the integers the core expects. `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here.

use rk_core::{fx, game};
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: game::Game,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(frogs: &str, course: &str, palette: &str, saved: &str) -> Result<Game, JsValue> {
        game::Game::new(frogs, course, palette, saved)
            .map(|inner| Game { inner })
            .map_err(|e| JsValue::from_str(&e))
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: u32, css_h: u32, dpr: f64) -> Vec<i32> {
        self.inner
            .resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
            .to_vec()
    }

    /// The page's clock in milliseconds, held action bits, mouse movement in CSS pixels.
    pub fn advance(&mut self, now_ms: f64, bits: u32, dx: f64, dy: f64) {
        self.inner.advance(
            (now_ms * 1000.0) as i64,
            bits,
            (dx * 100.0) as i32,
            (dy * 100.0) as i32,
        )
    }

    pub fn view(&self) -> Vec<f32> {
        floats(self.inner.view())
    }

    pub fn instances(&self) -> Vec<f32> {
        floats(self.inner.instances())
    }

    pub fn static_instances(&self) -> Vec<f32> {
        floats(self.inner.static_instances())
    }

    pub fn terrain(&self) -> Vec<f32> {
        floats(self.inner.terrain_vertices())
    }

    pub fn draws(&self) -> Vec<u32> {
        self.inner.draws()
    }

    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.scene_uniforms())
    }

    pub fn clear_color(&self) -> Vec<f32> {
        floats(self.inner.clear_color())
    }

    pub fn scene_version(&self) -> u32 {
        self.inner.scene_version()
    }

    pub fn hud(&self) -> String {
        self.inner.hud_json()
    }

    pub fn screen(&self) -> String {
        self.inner.screen_json()
    }

    pub fn codex(&self) -> String {
        self.inner.codex_json()
    }

    pub fn begin(&mut self) {
        self.inner.begin()
    }

    pub fn cycle_slot(&mut self, slot: u32) {
        self.inner.cycle_slot(slot as usize)
    }

    pub fn cycle_frog(&mut self, slot: u32, forward: bool) {
        self.inner.cycle_frog(slot as usize, forward)
    }

    pub fn cycle_course(&mut self, forward: bool) {
        self.inner.cycle_course(forward)
    }

    pub fn start_round(&mut self) {
        self.inner.start_round()
    }

    pub fn tee_off(&mut self) {
        self.inner.tee_off()
    }

    pub fn next_hole(&mut self) {
        self.inner.next_hole()
    }

    pub fn pause(&mut self) {
        self.inner.pause()
    }

    pub fn resume(&mut self) {
        self.inner.resume()
    }

    pub fn quit(&mut self) {
        self.inner.quit()
    }

    pub fn again(&mut self) {
        self.inner.again()
    }

    /// Host online from the setup screen; the page's clock seeds the room code.
    pub fn host_online(&mut self, build: &str, now_ms: f64) -> String {
        self.inner.host_online(build, now_ms as u64)
    }

    /// Join by a room code, or by a pasted invitation with an empty code.
    pub fn join_online(&mut self, build: &str, code: &str) -> bool {
        self.inner.join_online(build, code)
    }

    pub fn leave_online(&mut self) {
        self.inner.leave_online()
    }

    pub fn room(&self) -> String {
        self.inner.room()
    }

    pub fn net_peer(&mut self, peer: u32) {
        self.inner.net_peer(peer)
    }

    pub fn net_left(&mut self, peer: u32) {
        self.inner.net_left(peer)
    }

    pub fn net_error(&mut self, key: &str) {
        self.inner.net_error(key)
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        self.inner.net_message(peer, text)
    }

    /// What to send, as JSON `[[peer, text], ...]`.
    pub fn net_outbox(&mut self) -> String {
        self.inner.net_outbox()
    }

    /// The record, for the page to keep in local storage.
    pub fn saved(&self) -> String {
        self.inner.saved()
    }
}

#[wasm_bindgen]
pub fn numbers() -> String {
    game::numbers()
}

#[wasm_bindgen]
pub fn mesh(id: u32) -> Vec<f32> {
    floats(game::mesh_vertices(id))
}

/// White noise for the page's sound, from the core's seeded generator.
#[wasm_bindgen]
pub fn noise(n: u32, seed: u32) -> Vec<f32> {
    floats(game::noise(n, seed))
}
