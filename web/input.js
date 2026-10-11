// Keyboard and mouse, turned into the action bits the core defines.
// data/controls.json names the action for each key; the core's numbers()
// gives each action its bit. Dragging on the view turns the camera. Nothing
// here decides what an action does.

export function makeInput(canvas, controls, actions) {
  // `tapped` keeps a press that is released before the next frame reads it
  const state = { held: 0, tapped: 0, dx: 0, dy: 0, dragging: false };
  const bitOf = (code) => actions[controls[code]] || 0;

  function release() {
    state.held = 0;
    state.tapped = 0;
    state.dragging = false;
  }

  window.addEventListener("keydown", (e) => {
    const bit = bitOf(e.code);
    if (!bit) return;
    // keys typed into a button or a field stay there
    if (e.target && e.target.tagName === "INPUT") return;
    state.held |= bit;
    state.tapped |= bit;
    e.preventDefault();
  });
  window.addEventListener("keyup", (e) => {
    state.held &= ~bitOf(e.code);
  });
  canvas.addEventListener("pointerdown", (e) => {
    state.dragging = true;
    if (canvas.setPointerCapture) canvas.setPointerCapture(e.pointerId);
  });
  window.addEventListener("pointerup", () => {
    state.dragging = false;
  });
  window.addEventListener("pointermove", (e) => {
    if (!state.dragging) return;
    state.dx += e.movementX;
    state.dy += e.movementY;
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("blur", release);

  return {
    release,
    // Presses from the on-screen buttons (a touch screen has no keys).
    press(action, down) {
      const bit = actions[action] || 0;
      if (down) {
        state.held |= bit;
        state.tapped |= bit;
      } else {
        state.held &= ~bit;
      }
    },
    // The held bits, and the movement since the last read.
    read() {
      const drag = state.dragging ? actions.drag : 0;
      const bits = state.held | state.tapped | drag;
      const out = { bits, dx: state.dx, dy: state.dy };
      state.tapped = 0;
      state.dx = 0;
      state.dy = 0;
      return out;
    },
  };
}
