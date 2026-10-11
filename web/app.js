// The page draws what the core returns and decides nothing. It loads the data
// files, starts the core through the shim, and each display frame passes the
// clock, the held actions and the mouse movement in, then draws what comes
// out. The core says which screen is up; the page shows it.
import init, { Game, numbers, mesh } from "./pkg/rk_shim.js";
import { makeRenderer } from "./gl.js";
import { makeInput } from "./input.js";
import { makeHud } from "./hud.js";
import { makeScreens } from "./screens.js";
import { setup as setupSound, wake, toggleMute, play } from "./sound.js";

const BUILD = "__BUILD__";
const SAVE_KEY = "ribbit-king.record";
const el = (id) => document.getElementById(id);
const file = (path) => fetch(`${path}?v=${BUILD}`).then((r) => r.text());

// Browser storage can be missing or refused; the game works without it.
const storage = {
  load() {
    try {
      return localStorage.getItem(SAVE_KEY) || "";
    } catch {
      return "";
    }
  },
  save(text) {
    try {
      localStorage.setItem(SAVE_KEY, text);
    } catch {
      // nothing to do: the record is simply not kept
    }
  },
};

function fillCopy(copy) {
  for (const node of document.querySelectorAll("[data-copy]")) {
    node.textContent = copy[node.dataset.copy];
  }
  document.title = copy.title;
  el("build").textContent = BUILD;
}

async function start() {
  const names = ["frogs", "course", "palette", "copy", "controls", "sounds", "display"];
  const texts = await Promise.all(
    names.map((n) => file(`data/${n}.json`)).concat([file("data/shaders/scene.vert"), file("data/shaders/scene.frag")]),
  );
  const [frogs, course, palette, copyText, controlsText, soundsText, displayText, vertex, fragment] = texts;
  const copy = JSON.parse(copyText);
  const display = JSON.parse(displayText);
  fillCopy(copy);
  setupSound(JSON.parse(soundsText));
  await init();
  const N = JSON.parse(numbers());

  let game;
  try {
    game = new Game(frogs, course, palette, storage.load());
  } catch (e) {
    el("message").textContent = String(e);
    document.body.dataset.ready = "true";
    return;
  }

  const canvas = el("view");
  const renderer = makeRenderer(canvas, N, { vertex, fragment, software: display.software }, (id) => mesh(id));
  if (!renderer) el("message").textContent = copy.no_webgl;
  const input = makeInput(canvas, JSON.parse(controlsText), N.actions);
  const hud = makeHud(copy, play);
  const save = () => storage.save(game.saved());
  let shownMode = null;
  const screens = makeScreens(game, copy, () => showMode(JSON.parse(game.hud()).mode, true));
  screens.help(copy.controls, copy.help_rows);

  // a button that changes the screen: wake the sound, act, keep the record
  const act = (fn) => () => {
    wake();
    fn();
    if (document.activeElement) document.activeElement.blur();
    save();
    showMode(JSON.parse(game.hud()).mode, true);
  };
  el("play").addEventListener("click", act(() => game.begin()));
  el("setup-back").addEventListener("click", act(() => game.quit()));
  el("course-prev").addEventListener("click", act(() => game.cycle_course(false)));
  el("course-next").addEventListener("click", act(() => game.cycle_course(true)));
  el("start").addEventListener(
    "click",
    act(() => {
      game.start_round();
      hud.reset();
    }),
  );
  el("tee-off").addEventListener("click", act(() => game.tee_off()));
  el("card-next").addEventListener(
    "click",
    act(() => {
      if (JSON.parse(game.hud()).mode === "final") game.again();
      else game.next_hole();
      hud.reset();
    }),
  );
  el("card-quit").addEventListener("click", act(() => game.quit()));
  el("resume").addEventListener("click", act(() => game.resume()));
  el("quit").addEventListener("click", act(() => game.quit()));
  el("mute").addEventListener("click", () => toggleMute());
  for (const b of document.querySelectorAll(".open-help")) {
    b.addEventListener("click", () => {
      game.pause();
      el("help").hidden = false;
      b.blur();
    });
  }
  el("help-close").addEventListener("click", () => {
    el("help").hidden = true;
    el("help-close").blur();
  });

  // on-screen buttons, for a touch screen
  if (window.matchMedia && window.matchMedia("(pointer: coarse)").matches) el("touch").hidden = false;
  for (const b of document.querySelectorAll("#touch [data-hold]")) {
    b.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      wake();
      input.press(b.dataset.hold, true);
    });
    b.addEventListener("pointerup", () => input.press(b.dataset.hold, false));
    b.addEventListener("pointerleave", () => input.press(b.dataset.hold, false));
  }

  window.addEventListener("keydown", (e) => {
    const mode = document.body.dataset.mode;
    if (e.code === "Escape") {
      if (!el("help").hidden) el("help").hidden = true;
      else if (mode === "play") game.pause();
      else if (mode === "paused") game.resume();
    }
    if (e.code === "KeyN") toggleMute();
    wake();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) game.pause();
  });

  function resize() {
    const scale = renderer && renderer.software ? display.software_scale : 1;
    const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, (window.devicePixelRatio || 1) * scale);
    if (renderer) renderer.resize(w, h);
  }
  window.addEventListener("resize", resize);
  resize();

  const overlays = { title: "title", setup: "setup", intro: "intro", card: "card", final: "card", paused: "paused" };
  function showMode(mode, again) {
    if (mode === shownMode && !again) return;
    document.body.dataset.mode = mode;
    for (const id of new Set(Object.values(overlays))) el(id).hidden = true;
    if (overlays[mode]) el(overlays[mode]).hidden = false;
    el("hud").hidden = !["play", "paused"].includes(mode);
    const s = JSON.parse(game.screen());
    if (mode === "setup") screens.setup(s);
    if (mode === "intro") screens.intro(s);
    if (mode === "card") screens.card(s, false);
    if (mode === "final") screens.card(s, true);
    if (mode !== "play") input.release();
    if (mode === "card" || mode === "final") save();
    shownMode = mode;
  }

  let sceneVersion = null;
  function frame(now) {
    const { bits, dx, dy } = input.read();
    game.advance(now, bits, dx, dy);
    const h = JSON.parse(game.hud());
    showMode(h.mode, false);
    if (h.mode === "play") hud.update(h);
    else for (const s of h.sounds || []) play(s);
    if (renderer) {
      if (game.scene_version() !== sceneVersion) {
        sceneVersion = game.scene_version();
        renderer.setScene(game.static_instances(), game.uniforms(), game.clear_color(), game.terrain());
      }
      renderer.frame(game.view(), game.instances(), game.draws());
    }
    requestAnimationFrame(frame);
  }

  showMode(JSON.parse(game.hud()).mode, true);
  document.body.dataset.ready = "true";
  requestAnimationFrame(frame);
}

start();
