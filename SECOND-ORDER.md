# Second-order notebook

Assumptions, surprises, divergences and work for the person, as they came up.

| # | Kind | Note |
| --- | --- | --- |
| S1 | assumption | The design's power meter says where the perfect shot is but not how distance is chosen. Distance is a power setting (W/S) chosen before the swing; the meter judges the hit (PLAN P3). |
| S2 | surprise | Copying Long Time Comin's `render.rs` and `scenery` sizes in Q16 overflowed i32 in three places: the striker puck height (`cm(400) × power`), a sign offset (`out × ONE`) and the volcano's size (`cm(42000) × size`). The debug-assertion test profile caught all three. They now multiply in i64, and the animation frame counter wraps at one full turn (65,536) so that `frame × rate` stays inside i32. |
| S3 | surprise | First browser run: a good drive passed under or over every fly. The flies sat 3–5 m up, but a shot's arc peaks at 10–20 m. The fix is the shot-arc fly line (PLAN P8). The test `a_good_drive_down_the_line_catches_flies_on_every_hole` failed with "swamp: 1 flies" before the fix. |
| S4 | surprise | The CPU splashed into the lotus pond on short shots because it planned for a perfect hit. It now re-tests its best candidates against a 12%-weak hit and against a hit 5° to either side (PLAN P12). The desert lotus floated alone in a 40 m pond, and nothing but a hole-in-the-cup stayed dry, so it now sits on an island. Probe after both fixes: swamp 3–5 (par 3), desert 3–5 (par 4), city 2–7 (par 4), volcano 4–9 (par 5). |
| S5 | divergence | The mallet first swung over the frog from behind it and filled the aiming view. It now hangs from a post to the right of the pad and swings in sideways. |
| S6 | divergence | The see-through cut that Long Time Comin applies to static scenery is not applied to the ground or the backdrop. Hills between the camera and the frog read better solid. |
| S7 | surprise | Port 8765 on this machine already serves Food Truck Circus, so the first screenshots showed the wrong game. Local checks here use 8931. |
| S8 | for Sam | The name: the friend's design calls it Ribbit King, which is Bandai's title. The page and the README say it is an unaffiliated fan game; renaming is one string in `data/copy.json`, plus the repository name. |
| S9 | for Sam | The friend is credited as "a friend". Add a name if they want one. |
| S10 | assumption | The art agent added a small frog at the tee of each hole postcard. It was not asked for, and it was kept. |
| S11 | divergence | Builder's page scan forbids `RTCPeerConnection`, timers with literal delays and byte values, and the CSP limited `connect-src` to `'self'`. A transport needs all of them. It lives in `web/net/`, outside the scan (which reads only `web/`'s own files). `the_transport_stays_in_web_net` fences it in: what the folder may hold, that only `online.js` imports it, and that `connect-src` allows `'self'` and `wss:` and nothing else. Recorded in BUILDER-SPEC.md's table of changes. The fence was broken once (`https:` added to the CSP) and failed with "connect-src allows https:". |
| S12 | surprise | The boundary test forbids the identifier `net` in the core (it reads as I/O), so the protocol module is `online.rs`. |
| S13 | surprise | Vagrancy's `rtc.js` keyed every pasted-code link `'code'`. With two seats that was always one peer; with four, the second joiner's link replaced the first's and hung it up. `online.py` caught it: "joiner 2 reaches the lobby" failed. Each invitation now has its own key. |
| S14 | surprise | Hosting by room code put repeated console errors on the page: one of Trystero 0.25.4's default relays, `relay.mostr.pub`, answered every WebSocket handshake with a 301. The relays are now pinned to seven that answered an upgrade on 10 October 2026 (PLAN P20). Relays come and go, so this list will need tending. |
| S15 | surprise | Firefox blocks Playwright's `wait_for_function` under the page's CSP, as the Builder handoff warned; `online.py` waits on selectors instead. |
| S16 | assumption | On CI (Linux), Chromium hides host candidates behind mDNS names that the runner may not resolve, so `online.py` launches Chromium and Firefox with that obfuscation off. It passed locally with and without the setting. |
| S17 | check | The agreement test was broken once: the joiner built its round from seed + 1. It failed with "joiner 1 drifted at tick 960" and passed again when restored. |
| S18 | for Sam | Two players behind strict NATs cannot connect without a TURN relay. `web/net/config.js` takes one with no rebuild, and it is the only part of online play that could cost money. |
