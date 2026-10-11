#!/usr/bin/env python3
"""Four browsers play one round online, introduced by pasted codes.

After Vagrancy's testing/online.py. `?ice=none` drops the STUN servers, so the
browsers connect on host candidates alone and no request leaves the origin;
the room-code path (Trystero over public relays) and STUN are checked by hand
(HANDOFF.md). The host opens three seats and lets in three joiners one at a
time: an invitation from the host, a reply from the joiner. Then everyone
taps Space for a while, which swings whenever it is that browser's turn.

It fails on: a joiner never reaching the lobby, a page error or console
error, a request off the origin, any browser showing the copies disagree,
or the four standings tables not matching once play has settled.

    python3 tests/browser/online.py [chromium firefox ...]   (default: chromium)
"""
import functools
import http.server
import json
import pathlib
import re
import sys
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
failures = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *a):
        pass


def serve():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(DIST)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def run(p, engine, origin):
    print(f"\n[{engine}] online, four browsers")
    # Browsers hide a machine's own addresses behind mDNS names, which a CI
    # runner may not resolve; the four browsers here share one machine, so
    # plain local addresses are what lets them meet without STUN.
    options = {
        "chromium": {"args": ["--disable-features=WebRtcHideLocalIpsWithMdns"]},
        "firefox": {"firefox_user_prefs": {"media.peerconnection.ice.obfuscate_host_addresses": False}},
    }.get(engine, {})
    browser = getattr(p, engine).launch(**options)
    problems, offsite = [], []
    pages = []
    for who in ["host", "joiner 1", "joiner 2", "joiner 3"]:
        ctx = browser.new_context()
        page = ctx.new_page()
        page.on("console", lambda m, w=who: problems.append(f"{w}: {m.text}") if m.type == "error" else None)
        page.on("pageerror", lambda e, w=who: problems.append(f"{w}: {e}"))
        page.on("request", lambda r: None if r.url.startswith((origin, "blob:", "data:")) else offsite.append(r.url))
        page.goto(origin + "/index.html?ice=none")
        page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        pages.append(page)
    host = pages[0]
    host.click("#play")
    host.click("#host-code")
    host.wait_for_timeout(200)
    # seats 2, 3: CPU, off, open; seat 4: off, open
    for seat, clicks in [(2, 2), (3, 2), (4, 1)]:
        for _ in range(clicks):
            host.click(f"#slots .slot:nth-child({seat}) .kind")
            host.wait_for_timeout(100)
    kinds = [b.inner_text() for b in host.query_selector_all("#slots .kind")]
    check(kinds.count(COPY["slot_open"]) == 3, f"the host opens three seats {kinds}")

    for k, joiner in enumerate(pages[1:], start=1):
        host.evaluate("delete document.body.dataset.invite")
        host.click("#new-invite")
        host.wait_for_selector("body[data-invite='ready']", state="attached", timeout=15000)
        invite = host.input_value("#invite-out")
        joiner.click("#online-open")
        joiner.click("#online details summary")
        joiner.fill("#invite-in", invite)
        joiner.click("#make-reply")
        joiner.wait_for_selector("body[data-reply='ready']", state="attached", timeout=15000)
        reply = joiner.input_value("#reply-out")
        host.fill("#reply-in", reply)
        host.click("#accept-reply")
        try:
            joiner.wait_for_selector("#joiner-info:not([hidden])", timeout=15000)
            # selectors, not wait_for_function: Firefox's CSP blocks that (Builder handoff)
            joiner.locator("#joiner-status", has_text=COPY["online_waiting_host"]).wait_for(timeout=15000)
            ok = True
        except Exception:
            ok = False
        check(ok, f"joiner {k} reaches the lobby")
        mine = joiner.query_selector_all("#slots .slot.you")
        check(len(mine) == 1, f"joiner {k} sees one seat as its own ({len(mine)})")
    host.wait_for_timeout(500)
    remote = host.query_selector_all("#slots .slot.remote")
    check(len(remote) == 3, f"the host sees three online players ({len(remote)})")

    # a joiner changes its frog; the host's lobby shows it
    before = host.inner_text("#slots .slot:nth-child(2) .frog-name")
    pages[1].click("#slots .slot.you .row button:last-child")
    host.wait_for_timeout(800)
    after = host.inner_text("#slots .slot:nth-child(2) .frog-name")
    check(before != after, f"joiner 1 picks another frog and the host sees it ({before} -> {after})")

    host.click("#start")
    for page in pages:
        page.wait_for_selector("body[data-mode='intro']", state="attached", timeout=15000)
    check(True, "every browser shows the first hole's card")
    host.click("#tee-off")
    for page in pages:
        page.wait_for_selector("body[data-mode='play']", state="attached", timeout=15000)
    check(True, "the host tees off and every browser plays")

    for _ in range(40):
        for page in pages:
            page.keyboard.press("Space")
        pages[0].wait_for_timeout(450)
    pages[0].wait_for_timeout(6000)
    standings = [page.inner_text("#h-standings") for page in pages]
    strokes = [page.inner_text("#p-stroke") for page in pages]
    print("   standings:", standings[0].replace("\n", " | "))
    check(len(set(standings)) == 1, "all four browsers show the same standings")
    check(len(set(strokes)) == 1, f"and the same stroke for whoever is up {strokes}")
    played = sum(int(n) for n in re.findall(r"(\d+)\s*✿?\s*$", standings[0], re.M))
    check(played >= 4, f"strokes were played ({played})")
    badges = [page.inner_text("#h-online") for page in pages]
    check(COPY["online_desync"] not in badges, f"no browser reports the copies disagree {badges}")
    check(not problems, f"no page or console error {problems[:3]}")
    check(not offsite, f"no request off the origin {offsite[:3]}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/ is missing: run scripts/build.sh first")
    engines = sys.argv[1:] or ["chromium"]
    srv = serve()
    origin = "http://127.0.0.1:%d" % srv.server_address[1]
    with sync_playwright() as p:
        for engine in engines:
            run(p, engine, origin)
    srv.shutdown()
    if failures:
        print(f"\nONLINE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE CHECK PASSED in {', '.join(engines)}")


if __name__ == "__main__":
    main()
