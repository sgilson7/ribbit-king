# Ribbit King

Frolf in the browser: golf, played with frogs. Your frog sits on a catapult; whack the catapult with a mallet and the frog flies. Steer it through the air, collect flies and power-ups, bounce off lily pads and spider webs, and land it in the lotus flower. The fewest strokes over the round wins, and the leader wears the crown.

**Play:** https://sgilson7.github.io/ribbit-king/

It is a fan game after Bandai's *Ribbit King* (GameCube, 2003), built from a design a friend wrote. It is not affiliated with Bandai and uses none of its material: every frog, course, picture and sound here is new.

- **The swing.** Aim with A and D, set the mallet's power with W and S, and change the loft with Q. A dotted arc shows where a perfect hit at that power first lands. Press Space and a needle starts round a circular meter. Press Space again to whack. The gold notch in the middle of the red power bar is a perfect shot: full power, dead straight, and a little extra. Elsewhere on the bar is weaker, and an early hit hooks left while a late one slices right. Off the bar is a whiff.
- **In the air.** You fly the frog: W A S D steer it, relative to the camera. Drag with the mouse to look around.
- **Power-ups.** Extra jumps (Space in the air, carry up to three). Wings, which give a glide after Mario 64's wing cap: A and D bank, W dives, S climbs. Explosive mushrooms, which blast the frog up and away when it touches one.
- **Bouncers.** Lily pads and spider webs throw a frog back up. Webs throw harder.
- **Flies.** Lines of flies hang over each hole; the first line on every hole follows the arc of a real shot. They look like the flies in *The Binding of Isaac*. Ten flies make the tongue ready: press E with another frog in reach and it gets knocked flying. A frog knocked into the water takes a penalty stroke.
- **Hazards.** Water, lava and out of bounds each cost a stroke, and the frog goes back to where it was hit from.
- **Online, up to four browsers.** Choose Play, then Host online: the setup screen shows a room code and a link. Friends choose Play online and type the code. Each picks their own frog. You swing on your own turn and watch everyone else's live. If someone leaves, the CPU takes their frog. When a room code will not connect, the host can invite by pasted code instead. No server of ours is involved: the browsers connect directly.
- **Turns.** Up to four frogs, people or CPU, tee off in order. After that, the frog farthest from the lotus plays, as in golf. A hole is given up at five over par.
- **Four holes,** each in its own theme: *Lily Swamp* (par 3), *Oasis Dunes* (par 4), *Main Street* (par 4, with traffic), and *Lava Flow* (par 5).
- **Eight frogs,** each with its own colours, pattern and big eyes: a red-eyed tree frog, a blue poison dart frog, a yellow-banded dart frog, a tomato frog, a cane toad, a golden toad, a horned frog and a glass frog.

## Controls

| Input | Does |
| --- | --- |
| A D or ← → | Aim; steer in the air |
| W S or ↑ ↓ | Power; steer in the air (with wings: dive, climb) |
| Q | Change loft (low, middle, high) |
| Space or Enter | Start the meter; whack; jump in the air |
| X | Back out of the meter |
| E | Tongue (10 flies) |
| Mouse drag | Look around |
| Esc · N | Pause · Sound on or off |

On a touch screen, buttons appear along the bottom. Online, Esc shows the menu without stopping the round.

## Online play

Online play is Floodline's and Vagrancy's way of connecting browsers, with nothing of ours running anywhere. The transport is `web/net/rtc.js`: Vagrancy's port of Floodline's `quad_rtc.js`, extended from two seats to four. It is WebRTC between browsers in a star, so every joiner connects only to the host. Two ways in reach the same connection:

- **A room code.** The vendored [Trystero](https://github.com/dmotz/trystero) bundle introduces the browsers over public Nostr relays. The relays are pinned in `web/net/config.js`, and the room name carries the build hash, so two versions of the game never meet.
- **A pasted code.** The host's invitation and the joiner's reply carry the whole handshake, and no relay is involved.

On top of that, `crates/core/src/online.rs` runs a host's tick stream:

- The host runs the round on its own clock. Every tick it sends each joiner one step: the input of whoever's turn it is, plus any command (tee off, next hole, a seat handed to the CPU).
- Each joiner runs the same round from the same start and the same steps, so every copy agrees tick for tick.
- Golf is one frog at a time, so nobody waits on anybody but the player whose turn it is.
- A whack is judged by where the needle stood on the swinging player's own screen, so the round trip to the host does not move it.
- Every second each joiner sends a checksum of its copy, and the host compares it with its own.

`tests/browser/online.py` connects four browsers by pasted codes and plays a round, and the loopback test in `game.rs` plays a whole round between four in-process copies with delayed messages. The room-code path reaches public relays, so it is checked by hand (see `HANDOFF.md`). Two players who are both behind strict NATs cannot connect without a TURN relay, which this project does not run.

## How it is built

Ribbit King is a [Builder](https://github.com/sgilson7/builder-setting) project, and its parents are Sam's other games.

- **From Long Time Comin' and Iron Vector:** the 3D engine. It uses Q16.16 fixed-point maths with no floats in the core. Each mesh is drawn in one instanced draw, and the core computes every matrix. Shapes are lit in toon bands and outlined in ink (the inverted hull). The core runs at a fixed 60 ticks a second.
- **From Floodline and Vagrancy:** online play. The transport, the star, the role byte, the pasted-code handshake and the vendored Trystero bundle are theirs. The host's tick stream is new here.
- **From Long Time Comin' and Food Truck Circus:** the build and check scripts, and the TikZ art pipeline after `Sam_Comm_final.png`.
- **New here:**
  - The ground is a heightfield built from each hole's hills, plateaus, ridges and seeded roughness. It is drawn as one mesh whose vertices carry the ground's own colours: striped fairways, roads, shores and rock on slopes. The frog rolls on exactly the triangles it is drawn on.
  - One physics step runs the shot the player flies, the aiming guide and the CPU's imagined shots, so all three agree.
  - The CPU imagines a few hundred shots. It keeps the best few, then picks the one that still comes off if its whack is a little weak or crooked.

The layers:

- **`crates/core`** holds every rule: the swing and the meter, the physics, the power-ups, the tongue, turns, scoring, the CPU, the online protocol, the camera and the HUD. It has no input or output, no floats and no clock.
- **`crates/shim`** moves values across the WebAssembly boundary and decides nothing.
- **`web/`** is a static page with a WebGL 2 canvas. It draws what the core returns. `web/net/` holds the online transport and its pinned Trystero bundle, fenced off by a boundary test.
- **`data/`** holds every frog, hole, colour, word, sound recipe and shader.
- **`tools/art.py`** draws every picture as TikZ. The sources are kept in `tools/art/`.

```bash
./scripts/setup.sh   # once: Playwright and three browser engines
./scripts/check.sh   # fmt, clippy, the core tests, the wasm build and audit, the browser check, four browsers online
python3 scripts/serve.py 8000
```

A push to `main` runs the same check in Chromium and deploys `dist/` to GitHub Pages if it passes.

`BRIEF.md` holds the request, `PLAN.md` the decisions, `SECOND-ORDER.md` what was found while building, and `HANDOFF.md` where the work stands.

## Credits

Made by Sam Gilson with Claude, from a game design by a friend. It is after Bandai's *Ribbit King*, and the flies are after *The Binding of Isaac*. It uses nothing of either.
