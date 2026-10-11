"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would and assert what the page
shows. Expected values are worked out by hand from the data files, never
copied from the game's own output:

- the title is copy.json's "title"; the help lists 13 rows (help_rows) and 11 keys (controls).
- setup starts with three frogs seated (a player and two CPUs, game.rs's
  default slots) and one empty seat; course.json's first hole is "Lily Swamp", par 3.
- a seat cycles Player, CPU, Empty; the first seat is never empty, so it
  goes Player, CPU, Player.
- after teeing off, the HUD names the hole and shows the meter; the player's
  first stroke, once whacked, reads "Stroke 1".
- V rises into the survey: its pin's tag names the metres to the lotus and
  the prompt is the survey's; V again comes back down.
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
    check(len(rows) == 13 and len(keys) == 11, f"13 help rows and 11 keys ({len(rows)}, {len(keys)})")
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
    # Player, CPU, then past Empty back to Player: two clicks, and the seat
    # ends on a person, so the whack below is the person's and not a CPU's
    kinds = []
    for _ in range(2):
        click(page, "#slots .slot:nth-child(1) .kind")
        kinds.append(page.inner_text("#slots .slot:nth-child(1) .kind"))
    check(not page.query_selector("#slots .slot:nth-child(1).off"), "the first seat is never empty")
    check(kinds == [COPY["slot_cpu"], COPY["slot_human"]], f"it goes CPU, then skips Empty back to Player {kinds}")

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
    check(page.is_visible("#survey-btn"), "the Survey button shows")

    page.keyboard.press("KeyV")
    try:
        page.wait_for_selector("body[data-survey='on'] #survey-tag:not([hidden])", timeout=10000)
    except Exception:
        pass
    check(page.is_visible("#survey-tag"), "V opens the survey and its pin's tag shows")
    to = COPY["survey_to"].split("{m}")[1]
    check(to in page.inner_text("#survey-tag"), f"the tag gives the way on to the lotus ({page.inner_text('#survey-tag')!r})")
    check(page.inner_text("#prompt").startswith(COPY["prompt_survey"].split(" ·")[0]), "the prompt is the survey's")
    page.mouse.move(400, 300)
    page.mouse.wheel(0, 300)
    page.wait_for_timeout(200)
    page.keyboard.press("KeyV")
    try:
        page.wait_for_selector("body[data-survey='off']", state="attached", timeout=10000)
    except Exception:
        pass
    check(page.evaluate("() => document.body.dataset.survey") == "off", "V again comes back down")

    # whoever plays first, keep pressing Space until the human has whacked
    # (a CPU ignores the keys); a stroke is then on the card
    # wait for it rather than for a fixed time: on a software renderer the
    # big courses draw a few frames a second, and two taps can fall in one
    # frame (one press), so tap again until the stroke is in
    want = COPY["stroke"].replace("{n}", "1")
    for _ in range(8):
        page.keyboard.press("Space")
        try:
            page.locator("#p-stroke", has_text=want).wait_for(timeout=2500)
            break
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
