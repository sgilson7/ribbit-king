"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would and assert what the page
shows. Expected values are worked out by hand from the data files, never
copied from the game's own output:

- the title is copy.json's "title"; the help lists 12 rows (help_rows) and 9 keys (controls).
- setup starts with three frogs seated (a player and two CPUs, game.rs's
  default slots) and one empty seat; course.json's first hole is "Lily Swamp", par 3.
- a seat cycles Player, CPU, Empty; the first seat is never empty.
- after teeing off, the HUD names the hole and shows the meter; the player's
  first stroke, once whacked, reads "Stroke 1".
- Escape pauses and Resume plays on.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
COURSE = json.loads((ROOT / "data" / "course.json").read_text())


def click(page, selector):
    page.click(selector, timeout=5000)
    page.wait_for_timeout(250)


def mode(page):
    return page.evaluate("() => document.body.dataset.mode")


def run(page, check, engine):
    message = page.inner_text("#message")
    check(message in ("", COPY["no_webgl"]), f"no error message is shown ({message!r})")
    check(mode(page) == "title", f"the game opens on the title ({mode(page)})")
    check(page.text_content("#title h1") == COPY["title"], "the title from data/copy.json is drawn")

    click(page, "#title .open-help")
    check(page.is_visible("#help"), "How to play opens")
    rows = page.query_selector_all("#help-rows dt")
    keys = page.query_selector_all("#help-keys tr")
    check(len(rows) == 12 and len(keys) == 9, f"12 help rows and 9 keys ({len(rows)}, {len(keys)})")
    click(page, "#help-close")

    click(page, "#play")
    check(mode(page) == "setup", f"Play opens the setup ({mode(page)})")
    seated = page.query_selector_all("#slots .slot:not(.off)")
    empty = page.query_selector_all("#slots .slot.off")
    check(len(seated) == 3 and len(empty) == 1, f"three frogs seated, one seat empty ({len(seated)}, {len(empty)})")
    click(page, "#slots .slot:nth-child(4) .kind")
    seated = page.query_selector_all("#slots .slot:not(.off)")
    check(len(seated) == 4, f"the empty seat takes a player ({len(seated)})")
    names = [n.inner_text() for n in page.query_selector_all("#slots .frog-name")]
    check(len(set(names)) == len(names), f"no two seats share a frog {names}")
    for _ in range(2):
        click(page, "#slots .slot:nth-child(4) .kind")
    seated = page.query_selector_all("#slots .slot:not(.off)")
    check(len(seated) == 3, f"and back to empty after CPU ({len(seated)})")
    for _ in range(3):
        click(page, "#slots .slot:nth-child(1) .kind")
    check(not page.query_selector("#slots .slot:nth-child(1).off"), "the first seat is never empty")

    click(page, "#start")
    first = COURSE["holes"][0]
    check(mode(page) == "intro", f"Tee it up shows the hole's card ({mode(page)})")
    check(page.inner_text("#i-name") == first["name"], f"the first hole is {first['name']}")
    click(page, "#tee-off")
    check(mode(page) == "play", f"Tee off starts play ({mode(page)})")
    page.wait_for_timeout(600)
    check(page.inner_text("#h-hole") == first["name"], "the HUD names the hole")
    check(str(first["par"]) in page.inner_text("#h-par"), "and its par")
    check(page.is_visible("#meter-box"), "the meter shows while aiming")

    # whoever plays first, keep pressing Space until the human has whacked
    # (a CPU ignores the keys); a stroke is then on the card
    page.keyboard.press("Space")
    page.wait_for_timeout(600)
    page.keyboard.press("Space")
    # wait for it rather than for a fixed time: on a software renderer the
    # big courses draw few frames a second, and the clock runs slower with them
    want = COPY["stroke"].replace("{n}", "1")
    try:
        page.locator("#p-stroke", has_text=want).wait_for(timeout=20000)
    except Exception:
        pass
    stroke = page.inner_text("#p-stroke")
    check(stroke == COPY["stroke"].replace("{n}", "1"), f"the whack counts a stroke ({stroke!r})")

    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    check(mode(page) == "paused", f"Escape pauses ({mode(page)})")
    click(page, "#resume")
    check(mode(page) == "play", f"Resume plays on ({mode(page)})")
    click_quit(page)
    check(mode(page) == "title", f"Quit returns to the title ({mode(page)})")


def click_quit(page):
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    click(page, "#quit")
