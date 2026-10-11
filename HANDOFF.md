# Handoff

11 October 2026, round 5: online play fixed after "it has broken since the content update / survey view". Round 4 (the survey) and round 3 follow.

## Round 5: online fixed

Two causes, both fixed (SECOND-ORDER S31, PLAN P35–P36):

- **Pages on different versions never met.** A room's name included the build. After every deploy, a page loaded before it searched for a room the host wasn't in, with no message, forever. Rooms are now named by the code alone. A different version meets the host and is told to reload both pages, and a joiner who finds no host is told so after the relay timeout.
- **Slow computers blocked the handshake.** Drawing the big holes every frame behind the menus blocked a slow page for up to 5.5 s at a time. The page now draws nothing behind the full-screen art, and a single still frame behind the see-through menus.

| Check | Result |
| --- | --- |
| Live site before the fix, four software-rendered pages by room code | one joiner of three failed; about 170 s of long tasks a page |
| Local build, public relays, software-rendered, four pages by room code | all three joiners in, standings equal, two runs of two; about 0.6 s of long tasks a page |
| A joiner on a different build | turned away in about 1 s with the reload message |
| A joiner in an empty room | "No host answered…" after the relay timeout |
| `./scripts/check.sh` | BUILDER CHECK PASSED. 78 core tests, 12 boundary tests; browser and online checks in Chromium, Firefox and WebKit |

**Everyone must reload once.** A page from before this fix still uses the old room names and can't meet the new build.


## Round 4: survey the hole

Press V or M, or click Survey under the hole's name. The camera rises over the hole until the frog and the lotus are both in view (PLAN P34). Controls while surveying:

- W A S D pan the view.
- The wheel or + and − zoom it.
- Dragging turns it.

A pin on the ground reports how far it is from the frog, how far from it to the lotus, and how much higher or lower it sits. The aiming arc stays drawn and scaled up so it shows from above. V, Space or X comes back down. The survey moves only the person's own camera, so it works on any turn and online it affects nobody else.

| Check | Result |
| --- | --- |
| `./scripts/check.sh` | BUILDER CHECK PASSED. 77 core tests (4 new for the survey), 12 boundary tests; browser check (now opens, zooms and closes the survey) and four-browser online check in Chromium, Firefox and WebKit |
| CI (Linux, Chromium, online included) | success, 14 min 48 s, deployed. The first push failed CI on a test bug that the survey step exposed (SECOND-ORDER S30); fixed in the second |
| Live, browser check | passed in three engines, survey steps included |
| Frame rate, 1280×720, Chromium | 60 fps aiming and 60 fps surveying on the GPU; 5–5.5 fps for both on a software renderer |


## Round 3: playtest changes

All of the designer's and players' notes (BRIEF.md, round 3) are built:

- five holes of par 5 or more (Toadstool Hollow is the new fungus course), each with three crossing routes, cliffs, mesas, tunnels, rivers and waterfalls;
- swimming at no stroke, with a lily pad for the next shot;
- webs set into the ground, 3–9 m, with the Deku-tree funnel;
- blue bumper mushrooms, point bubbles and fly hops;
- the bell's firework on a perfect hit, and a live leader's crown;
- snails and whirlpools in the swamp, and antlions in the desert.

The holes are generated: edit `tools/courses.py`, run it, and it rewrites `data/course.json`.

| Check | Result |
| --- | --- |
| `./scripts/check.sh` | BUILDER CHECK PASSED. 73 core tests, 12 boundary tests; browser check and four-browser online check in Chromium, Firefox and WebKit |
| CI (Linux, Chromium, online included) | success, 12 min 44 s, deployed |
| Live, browser check | passed in three engines |
| Live, four browsers by room code over the public relays | all three joiners in, standings equal, twice with GPU rendering. With software rendering it failed to connect some joiners (SECOND-ORDER S26) |
| CPU probe, 4 seeds a hole | swamp 4–5, desert 4–5, fungus 4–5, city 4–6 (par 5); volcano 4–8 (par 6); slowest tick 3–15 ms |

Website: the page now describes the five holes and the new features. `make check`, `make packet` and the gate pass, and it is deployed. The source is again not pushed: it is ahead of its remote by the other session's `a08d87e` and my two Ribbit King commits.

## Round 2: online online play for up to four browsers is built, deployed and listed. Round 1 (the MVP) follows below.

## Round 2: online

| What | Result |
| --- | --- |
| How it works | Floodline's and Vagrancy's transport (`web/net/rtc.js`, pinned Trystero bundle) with a host's tick stream on top (`crates/core/src/online.rs`, PLAN P16–P20) |
| `./scripts/check.sh`, now seven steps | BUILDER CHECK PASSED. 63 core tests and 12 boundary tests; browser check and four-browser online check in Chromium, Firefox and WebKit |
| CI (Linux, Chromium, online step included) | success, 8 min 46 s, deployed |
| Live game, browser check | passed in three engines |
| Live game, four browsers by **room code over the public relays** (by hand, Chromium) | three joiners reached the lobby; the host saw three online players; after play, all four standings tables were identical |
| Website page and tagline updated, `make check` / `make packet` / gate / `make deploy` / `make gate-live` | no findings / clean / passed / build 9c23bb03 / passed |

**Website source not pushed.** The website repository was one commit ahead of its remote: another session's `a08d87e` ("A missing semicolon blanked nine tiles"), already published as build 39bddafb but not pushed. My commit sits on top of it and is deployed (build 9c23bb03). I did not push the source, so pushing `a08d87e` stays with whoever made it. `git push` in `~/Documents/website` sends both.

**For Sam:**

1. Play a real round with friends on different networks. Everything above ran on one machine, apart from the relays.
2. Strict NATs (SECOND-ORDER S18): if two players cannot connect even by pasted code, a TURN server in `web/net/config.js` fixes it, and it is the one thing that could cost money.
3. The relay list (S14) will need tending as Nostr relays come and go.

## Round 1: the MVP

| What | Where |
| --- | --- |
| The game, live | https://sgilson7.github.io/ribbit-king/ |
| Repository (public, MIT) | https://github.com/sgilson7/ribbit-king |
| Website listing | https://sgilson7.github.io/projects/ribbit-king/ and the games grid on /projects/ |

## What is in it

There are four holes: swamp (par 3), desert oasis (4), city street with traffic (4) and lava flow (5). There are eight frogs, and up to four players, people or CPU. The game has the spinning power meter with a perfect zone, flight control, extra jumps, wings, explosive mushrooms, lily pads and webs, fly lines with a tongue that knocks other frogs, hazards with stroke and distance, golf turn order, a scorecard, a crown for the leader, and a final card. Records are kept in the browser. There are touch buttons on touch screens. The menu art is TikZ (`tools/art.py`): the title, four hole postcards, a crown picture, eight frog portraits and nine icons.

## Checks run

| Check | Result |
| --- | --- |
| `./scripts/check.sh` (fmt, clippy, 56 core tests and 11 boundary tests, wasm build, import audit, browser check in Chromium, Firefox, WebKit) | BUILDER CHECK PASSED |
| GitHub Actions on push to `main` (the same check in Chromium, then deploy) | success, 2 min 44 s |
| `tests/browser/check.py` against the live origin | passed in Chromium, Firefox and WebKit |
| Website `make check` / `make packet` / gate (local, then `make gate-live`) | no findings / 44 routes clean / passed in three engines, twice |
| CPU probe (`cargo test -p rk_core --release --test cpu_probe -- --nocapture`), 6 seeds | swamp 3–5, desert 3–5, city 2–7, volcano 4–9 |

## On the website

Another session committed `db593b2` ("Add Graftwood to the games") from the shared working tree while this listing was being added. That commit picked up the Ribbit King record and route. It published build 20f6efe0, which already included this page and tile. `ac2db33` commits the page and the tile so the source matches what is live. The build id is unchanged, so no second deploy was needed.

## Waiting on Sam

1. **The name** (SECOND-ORDER S8). It is Bandai's title. The page calls this an unaffiliated fan game.
2. **Credit for the friend** (S9). The design is quoted verbatim in BRIEF.md; the README says "a friend".
3. **Play it.** The feel numbers are all in `crates/core/src/balance.rs`: launch speed, gravity, the meter's lap and bar, glide, bounce and the CPU's miss. The holes are in `data/course.json`.

## Next, if wanted

More holes per theme; online or pass-the-pad multiplayer polish; music; a hover clip for the website tile; a wind or slope readout; sounds recorded rather than synthesized.
