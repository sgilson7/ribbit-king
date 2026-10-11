# Handoff

10 October 2026, round 2: online play for up to four browsers is built, deployed and listed. Round 1 (the MVP) follows below.

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
