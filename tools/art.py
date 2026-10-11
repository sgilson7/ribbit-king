#!/usr/bin/env python3
"""Ribbit King's 2D menu art, drawn as TikZ and compiled, never generated
as pixels (Sam's diagrams-as-code prompt: write compilable standalone TikZ,
list what an image generator gets wrong, and check the output against that
list). The structure follows LongTimeComin/tools/art.py.

Each picture is composed from sub-pictures defined once below (a frog
sitting, a frog leaping, the fly, the lotus, a lily pad, a spider web, a
mushroom, the catapult and its mallet, a cloud, a cypress, a saguaro, a
palm, a building with its rooftop clutter, a car, a fountain, a volcano)
and placed by the picture's own function. The look follows
vagrancy/Sam_Comm_final.png and the title screens of Sam's other games: a
warm sky with outlined puffy clouds, salmon, coral and pink, sage and teal,
flat fills, everything outlined in one plum ink with round joins, and lots
of small clutter.

    python3 tools/art.py            # every picture: web/art/**.png, tools/art/**.tex
    python3 tools/art.py title      # one picture
    python3 tools/art.py frogs      # one folder

Facts each picture must get right (the things an image generator gets
wrong), and where this file enforces each:

- no words, letters or digits in any picture; the page sets every word over
  the art. Nothing here emits a \\node or any text: every mark is a path
  (`rect`, `circle`, `ellipse`, `poly`, `blob`, `leaf`, `limb`, `tube`),
  and `check_no_text` refuses a .tex that contains \\node, \\text or a
  label before it is compiled.
- every frog has exactly two big eyes on top of its head and four legs:
  `frog` (sitting) draws two eye sockets above the crown of the head, two
  haunches with two hind feet and two front legs with two front feet;
  `frog_leap` (side view) draws a near and a far eye on top of the head and
  a near and a far pair of legs. Both read their colours from
  data/frogs.json (skin, belly, mark, eye, feet) and draw the `pattern`
  named there in the `mark` colour (`frog_marks`).
- flies are always the same fly, after The Binding of Isaac's: a round dark
  body (#2B2530), two red eyes, two pale translucent wings. Every fly in
  every picture is drawn by `fly`, the only place a fly is defined.
- the lotus goal is always pink (#F48FB1) with a yellow centre: every lotus
  is drawn by `lotus`, the only place a lotus is defined.
- plum ink (#3B2340) outlines every filled shape: every filling helper uses
  the `inked` style (draw=ink, round joins and caps). The exceptions are
  light, not shapes: glints, cheek blush, shading and glows, which are
  unoutlined and translucent.
- the left third of the hole postcards and the crown, and the lower-left
  third of the title, stay calm (plain sky, water or ground): the page's
  text panel sits there. Each picture's function keeps its clutter at
  x > 11 (or above y = 8 on the title).
- repeatable: every random choice comes from a seeded random.Random, so
  running the script twice draws identical pictures.
"""
import json, math, random, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "art"
SRC = ROOT / "tools" / "art"
FROGS = json.loads((ROOT / "data" / "frogs.json").read_text())["frogs"]
FROG = {fr["id"]: fr for fr in FROGS}

# Sam_Comm_final's palette by eye, plus each course's own and the game's.
COLOURS = {
    "ink": "3B2340", "sky": "EFC46A", "skyhi": "F5DA92", "skylow": "E9B060", "cloud": "F2D58A",
    "paper": "FBEFD0", "cream": "FFF6E0", "white": "FFFFFF",
    "wallA": "E88A73", "wallB": "EE9F7E", "wallC": "E47A7E", "wallD": "F0B07E", "wallE": "D9765F", "wallF": "E99A8A",
    "greenA": "8DBB7E", "greenB": "6FA77A", "greenC": "A9C97F", "greenD": "5E9C6A", "greenE": "4A8060",
    "meadow": "B5CF86", "meadowdark": "9DBE74",
    "window": "F6E7A0", "windark": "8C5A64", "metal": "8D9A93", "metaldark": "5E6B66",
    "wood": "B9835A", "wooddark": "8C5A3C", "woodlight": "D9A66F", "rope": "E3C48A",
    "stone": "E7C9A0", "stonedark": "C9A47E", "road": "8E7A86", "roadline": "F6E7A0", "kerb": "D8B48C",
    # carnival
    "red": "E0483E", "redlight": "F27A6A", "gold": "FFB000", "goldlight": "FFD54F", "bell": "F2B53A",
    "teal": "4FA3A0", "tealdark": "3C8582", "magenta": "FF2E88", "cyan": "13CFE3", "violet": "8A5BD6",
    # water and plants
    "water": "6FB3A8", "waterlight": "9FD3C6", "waterdark": "4F8E86",
    "pad": "7DB86A", "paddark": "5C9A52", "padlight": "A6D38A",
    "reed": "8DB26A", "cattail": "8A5A3C", "moss": "A8B88E", "cypress": "5E8A5A", "cypressdark": "4A7048",
    # the goal and the collectible
    "lotus": "F48FB1", "lotusdark": "E06C98", "lotuslight": "FAD0DF", "lotuscentre": "FFD54F", "lotusseed": "E0A82E",
    "flybody": "2B2530", "flyeye": "E0242E", "flywing": "F4F8FA",
    "mushred": "E23B2E", "mushspot": "FFF5E0", "stem": "F2E2C2",
    "blush": "F48FB1", "pupil": "1A1220", "spider": "3B2340",
    # swamp
    "swampskyhi": "E9E3A0", "swampsky": "D9D184", "swampskylow": "C9C478", "swampcloud": "EEE9B4",
    "swampwater": "4F8A7F", "swampwaterlight": "7AAE9C", "swampwaterdark": "3F7268", "log": "9A6A48", "logend": "D9A66F",
    # desert
    "dunehi": "F7DFA0", "dunesky": "F2C47A", "duneskylow": "EBA865",
    "sand": "EBC27C", "sandlight": "F4D69C", "sanddark": "D9A35E", "sanddeep": "C98C4E", "sun": "FFE6A3",
    "cactus": "6FA77A", "cactusdark": "4F8A60", "palmtrunk": "B98A5A", "rock": "C98C6A",
    # city
    "awning": "4FA3A0", "carA": "4FA3A0", "carB": "F2B53A", "carC": "E47A7E", "carD": "8DBB7E", "glass": "BFE3E0",
    # volcano
    "duskhi": "6B4A6E", "dusk": "B9607A", "dusklow": "E9906A", "duskcloud": "8E6680",
    "smoke": "9C8494", "smokedark": "7E6878",
    "basalt": "4A3A4A", "basaltdark": "352838", "basalttop": "6E5868", "rockface": "5E4A5C",
    "lava": "FF6A1A", "lavagold": "FFB000", "lavadark": "C9481A", "pool": "5FB8B0", "poollight": "9FE0D6",
    # crown
    "crown": "FFC23A", "crowndark": "E09A1A", "jewel": "E0483E", "jewelb": "4FA3A0",
}
for fr in FROGS:
    for part in ("skin", "belly", "mark", "eye", "feet"):
        COLOURS[fr["id"] + part] = fr[part].lstrip("#").upper()

WALLS = ["wallA", "wallB", "wallC", "wallD", "wallE", "wallF"]
GREENS = ["greenA", "greenB", "greenC", "greenD"]
SIGNS = ["magenta", "cyan", "gold", "red", "violet", "teal"]
CONFETTI = ["lotus", "gold", "teal", "red", "cyan", "wallD", "greenC", "violet"]

PREAMBLE = r"""\documentclass[tikz,border=0pt]{standalone}
\usepackage{xcolor}
\usetikzlibrary{calc}
% Palette: Sam_Comm_final.png by eye, each course's own, and data/frogs.json.
""" + "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLOURS.items()) + "\n"

# The ink's width, in pt, for each folder: small pictures get a bolder line.
INK = {"": 1.15, "holes": 1.15, "frogs": 1.7, "icons": 1.8}
CUR_INK = 1.15
PT_PER_CM = 28.4528


def f(x):
    return f"{x:.3f}".rstrip("0").rstrip(".") if abs(x) > 0.0005 else "0"


def P(x, y):
    return f"({f(x)},{f(y)})"


def pts(points):
    return " ".join(P(x, y) for x, y in points)


def rect(x0, y0, x1, y1, fill, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {P(x0, y0)} rectangle {P(x1, y1)};\n"


def rrect(x0, y0, x1, y1, fill, r=0.15, extra=""):
    return f"\\filldraw[inked,fill={fill},rounded corners={f(r)}cm{extra}] {P(x0, y0)} rectangle {P(x1, y1)};\n"


def circle(x, y, r, fill, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {P(x, y)} circle ({f(r)});\n"


def ellipse(x, y, rx, ry, fill, rot=0, extra=""):
    turn = f",rotate around={{{f(rot)}:{P(x, y)}}}" if rot else ""
    return f"\\filldraw[inked,fill={fill}{turn}{extra}] {P(x, y)} ellipse ({f(rx)} and {f(ry)});\n"


def ellipse_path(x, y, rx, ry, rot=0):
    turn = f"[rotate around={{{f(rot)}:{P(x, y)}}}]" if rot else ""
    return f"{turn} {P(x, y)} ellipse ({f(rx)} and {f(ry)})"


def poly(points, fill, extra=""):
    path = " -- ".join(P(x, y) for x, y in points)
    return f"\\filldraw[inked,fill={fill}{extra}] {path} -- cycle;\n"


def blob_path(points, tension=0.7):
    return f"plot[smooth cycle, tension={tension}] coordinates {{{pts(points)}}}"


def blob(points, fill, extra="", tension=0.7):
    return f"\\filldraw[inked,fill={fill}{extra}] {blob_path(points, tension)};\n"


def hill(top, base, fill, extra="", tension=0.6):
    """A shape whose top edge is a smooth curve and whose foot is flat."""
    x0, x1 = top[0][0], top[-1][0]
    return f"\\filldraw[inked,fill={fill}{extra}] plot[smooth, tension={tension}] coordinates {{{pts(top)}}} -- {P(x1, base)} -- {P(x0, base)} -- cycle;\n"


def line(a, b, lw=1.1, colour="ink", extra=""):
    return f"\\draw[{colour}, line width={f(lw)}pt, line cap=round{extra}] {P(*a)} -- {P(*b)};\n"


def curve(points, lw=1.1, colour="ink", extra="", tension=0.6):
    return f"\\draw[{colour}, line width={f(lw)}pt, line cap=round, line join=round{extra}] plot[smooth, tension={tension}] coordinates {{{pts(points)}}};\n"


def leaf_path(b, t, width, bend=0.0):
    """A leaf, petal or blade from base b to tip t: two curves through
    control points either side of the (bent) midline."""
    (bx, by), (tx, ty) = b, t
    dx, dy = tx - bx, ty - by
    L = math.hypot(dx, dy) or 1.0
    nx, ny = -dy / L, dx / L
    mx, my = (bx + tx) / 2 + nx * bend, (by + ty) / 2 + ny * bend
    c1 = (mx + nx * width, my + ny * width)
    c2 = (mx - nx * width, my - ny * width)
    return f"{P(bx, by)} .. controls {P(*c1)} .. {P(tx, ty)} .. controls {P(*c2)} .. {P(bx, by)} -- cycle"


def leaf(b, t, width, fill, bend=0.0, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {leaf_path(b, t, width, bend)};\n"


def limb_pts(path, hw0, hw1, cap=False):
    """The outline of a tapering limb along a polyline, as points for a
    smooth closed curve; with cap, the far end is rounded."""
    n = len(path)
    left, right = [], []
    for i, (x, y) in enumerate(path):
        if i == 0:
            dx, dy = path[1][0] - x, path[1][1] - y
        elif i == n - 1:
            dx, dy = x - path[i - 1][0], y - path[i - 1][1]
        else:
            dx, dy = path[i + 1][0] - path[i - 1][0], path[i + 1][1] - path[i - 1][1]
        L = math.hypot(dx, dy) or 1.0
        nx, ny = -dy / L, dx / L
        hw = hw0 + (hw1 - hw0) * i / (n - 1)
        left.append((x + nx * hw, y + ny * hw))
        right.append((x - nx * hw, y - ny * hw))
    if cap:
        (x, y), (px, py) = path[-1], path[-2]
        L = math.hypot(x - px, y - py) or 1.0
        ux, uy = (x - px) / L, (y - py) / L
        left.append((x + ux * hw1 * 0.95, y + uy * hw1 * 0.95))
    return left + right[::-1]


def limb(path, hw0, hw1, fill, extra="", cap=False):
    return blob(limb_pts(path, hw0, hw1, cap), fill, extra, tension=0.5)


def tube(paths, w, fill, tension=0.6):
    """Round-capped tubes w cm wide, outlined as one shape: every ink
    stroke goes down before any fill (cacti, stalks, poles, trunks).
    Absolute coordinates only (a scaled scope would not scale the width)."""
    wpt = w * PT_PER_CM
    out = ""
    for p in paths:
        out += f"\\draw[ink, line width={f(wpt + 2 * CUR_INK)}pt, line cap=round, line join=round] plot[smooth, tension={tension}] coordinates {{{pts(p)}}};\n"
    for p in paths:
        out += f"\\draw[{fill}, line width={f(wpt)}pt, line cap=round, line join=round] plot[smooth, tension={tension}] coordinates {{{pts(p)}}};\n"
    return out


def place(x, y, s, body, rot=0.0, flip=False):
    """Put a sub-picture drawn about its own origin at (x, y), scaled by s,
    turned by rot degrees, mirrored if flip."""
    xs = -s if flip else s
    return f"\\begin{{scope}}[shift={{{P(x, y)}}}, rotate={f(rot)}, xscale={f(xs)}, yscale={f(s)}]\n{body}\\end{{scope}}\n"


def clipped(path, body):
    return f"\\begin{{scope}}\\clip {path};\n{body}\\end{{scope}}\n"


def glint(x, y, r, opacity=0.9):
    return f"\\fill[white, opacity={f(opacity)}] {P(x, y)} circle ({f(r)});\n"


def rot(p, a, o=(0.0, 0.0)):
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a))
    x, y = p[0] - o[0], p[1] - o[1]
    return (o[0] + x * c - y * s, o[1] + x * s + y * c)


# ---- the fly, the lotus, the pad: the same in every picture ---------------

def fly_body():
    """The fly, after The Binding of Isaac's: two pale translucent wings, a
    round dark body, two red eyes. About its own centre, body radius 0.5."""
    s = ""
    for side in (-1, 1):
        s += ellipse(side * 0.5, 0.48, 0.4, 0.24, "flywing", rot=side * -32, extra=",fill opacity=0.65,line width=0.8pt")
        s += f"\\draw[ink, line width=0.5pt, opacity=0.6] {P(side * 0.22, 0.32)} -- {P(side * 0.72, 0.62)};\n"
    s += circle(0, 0, 0.5, "flybody")
    s += f"\\fill[white, opacity=0.18] {P(-0.18, 0.22)} circle (0.2);\n"
    for side in (-1, 1):
        s += circle(side * 0.2, 0.02, 0.13, "flyeye", ",line width=0.6pt")
        s += glint(side * 0.2 - 0.04, 0.06, 0.04)
    return s


def fly(x, y, s, rot=0.0):
    return place(x, y, s, fly_body(), rot)


def lily_pad_path(x, y, rx, ry, notch=270, gap=16):
    a1, a2 = notch + gap, notch + 360 - gap
    return f"{P(x, y)} -- ($({f(x)},{f(y)})+({f(a1)}:{f(rx)} and {f(ry)})$) arc[start angle={f(a1)}, end angle={f(a2)}, x radius={f(rx)}, y radius={f(ry)}] -- cycle"


def lily_pad(x, y, rx, ry, notch=270, fill="pad"):
    """A lily pad seen from above and a little in front: an ellipse with
    a wedge cut out to its middle, and veins."""
    s = f"\\filldraw[inked,fill={fill}] {lily_pad_path(x, y, rx, ry, notch)};\n"
    for k in range(7):
        a = math.radians(notch + 45 + k * 45)
        s += f"\\draw[paddark, line width=0.7pt] {P(x, y)} -- {P(x + math.cos(a) * rx * 0.78, y + math.sin(a) * ry * 0.78)};\n"
    s += f"\\fill[padlight, opacity=0.6] {P(x - rx * 0.35, y + ry * 0.35)} ellipse ({f(rx * 0.22)} and {f(ry * 0.16)});\n"
    return s


def lotus_body(pad=True, part="all"):
    """The goal: a pink lotus with a yellow seed-pod centre, on its pad.
    About the foot of the flower; about 2.4 wide, 1.7 tall. part "back"
    draws the pad, the back petals and the centre; "front" the petals in
    front of a throne's sitter; "all" the whole flower."""
    s = ""
    if part in ("all", "back"):
        if pad:
            s += lily_pad(0, 0, 1.55, 0.5, notch=300)
        # back row, deep pink, fanned
        for a in (-62, -31, 0, 31, 62):
            tip = rot((0, 1.55), a)
            s += leaf((0, 0.12), (tip[0], tip[1] + 0.1), 0.42, "lotusdark")
    if part == "back":
        for a in (-50, 50):
            tip = rot((0, 1.3), a)
            s += leaf((0, 0.05), (tip[0] * 1.1, tip[1] + 0.05), 0.4, "lotus")
        s += ellipse(0, 0.8, 0.62, 0.26, "lotuscentre")
        for dx in (-0.3, 0, 0.3):
            s += f"\\fill[lotusseed] {P(dx, 0.82)} circle (0.06);\n"
        return s
    # outer front row, pink, low and wide
    for a in ((-80, 80) if part == "front" else (-78, -48, 48, 78, -20, 20)):
        tip = rot((0, 1.2), a)
        s += leaf((0, 0.05), (tip[0] * 1.15, tip[1] + 0.05), 0.4, "lotus")
    if part == "front":
        return s
    # the seed pod, yellow, with its fringe of stamens
    for k in range(9):
        a = math.radians(180 + k * 22.5)
        s += circle(math.cos(a) * 0.55, 0.86 + math.sin(-a) * 0.12 + 0.1, 0.07, "lotuscentre", ",line width=0.5pt")
    s += ellipse(0, 0.86, 0.5, 0.21, "lotuscentre")
    for dx in (-0.24, 0, 0.24):
        s += f"\\fill[lotusseed] {P(dx, 0.88)} circle (0.055);\n"
    # inner petals, short so the centre shows above them
    for a in (-12, 14):
        tip = rot((0, 0.78), a)
        s += leaf((0, 0.0), (tip[0], tip[1] + 0.05), 0.33, "lotus")
    s += f"\\fill[lotuslight, opacity=0.7] (-0.12,0.18) .. controls (-0.2,0.45) .. (-0.08,0.68) .. controls (0.0,0.45) .. (-0.12,0.18);\n"
    return s


def lotus(x, y, s, pad=True):
    return place(x, y, s, lotus_body(pad))


# ---- frogs ------------------------------------------------------------------

# How each kind differs from the plain squat frog: width and height of the
# body, eye size, pupil shape, toe pads, mouth width, and its features.
FROG_KIND = {
    "lime": dict(w=0.9, h=1.05, eye=1.16, pupil="slit", toe=1.35),
    "azure": dict(w=1.0, eye=1.0),
    "bumble": dict(w=0.94, h=1.02, eye=1.02),
    "tomato": dict(w=1.14, h=0.93, eye=0.9, mouth=1.1),
    "bufo": dict(w=1.12, h=0.97, eye=0.92, pupil="bar", parotoid=True, brow=True),
    "goldie": dict(w=0.94, h=1.0, eye=1.0, toe=0.9),
    "chomp": dict(w=1.2, h=0.94, eye=0.86, pupil="bar", horns=True, mouth=1.4),
    "glassy": dict(w=0.9, h=1.02, eye=1.14, pupil="bar", innards=True, toe=1.2),
}


def foot(x, y, angles, length, toe_w, pad_r, fill, palm=0.17):
    """A frog's foot: toes fanned at the given angles, round toe pads, a
    palm over their roots."""
    s = ""
    for a in angles:
        tip = (x + math.cos(math.radians(a)) * length, y + math.sin(math.radians(a)) * length)
        s += leaf((x, y), tip, toe_w, fill)
        s += circle(tip[0], tip[1], pad_r, fill)
    s += circle(x, y, palm, fill)
    return s


def frog_marks(fr, rng, region, kind, bbox):
    """The pattern named in data/frogs.json, in the mark colour, for one
    region of the frog (body, belly, haunch), inside bbox. Drawn clipped."""
    pat, mark = fr["pattern"], fr["id"] + "mark"
    x0, y0, x1, y1 = bbox
    s = ""
    thin = ",line width=0.7pt"
    if pat == "spots":
        placed = []
        for _ in range(60 if region == "body" else 14):
            r = rng.uniform(0.12, 0.26) if region == "body" else rng.uniform(0.09, 0.17)
            x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
            if all(math.hypot(x - px, y - py) > r + pr + 0.12 for px, py, pr in placed):
                placed.append((x, y, r))
                s += circle(x, y, r, mark, thin)
    elif pat == "dots":
        placed = []
        for _ in range(140 if region == "body" else 30):
            r = rng.uniform(0.045, 0.085)
            x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
            if all(math.hypot(x - px, y - py) > 0.2 for px, py, pr in placed):
                placed.append((x, y, r))
                s += circle(x, y, r, mark, ",line width=0.5pt")
    elif pat == "warts":
        placed = []
        for _ in range(120 if region == "body" else 30):
            r = rng.uniform(0.07, 0.15)
            x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
            if all(math.hypot(x - px, y - py) > r + pr + 0.08 for px, py, pr in placed):
                placed.append((x, y, r))
                s += circle(x, y, r, mark, ",line width=0.6pt")
                s += f"\\fill[{fr['id']}belly, opacity=0.55] {P(x - r * 0.3, y + r * 0.3)} circle ({f(r * 0.35)});\n"
    elif pat == "blotches":
        placed = []
        for _ in range(40 if region == "body" else 8):
            r = rng.uniform(0.22, 0.4) if region == "body" else rng.uniform(0.15, 0.25)
            x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
            if all(math.hypot(x - px, y - py) > r + pr + 0.15 for px, py, pr in placed):
                placed.append((x, y, r))
                ring = [(x + math.cos(2 * math.pi * k / 7) * r * rng.uniform(0.65, 1.15),
                         y + math.sin(2 * math.pi * k / 7) * r * rng.uniform(0.55, 1.0)) for k in range(7)]
                s += blob(ring, mark, thin + f",fill opacity=0.92")
                s += f"\\fill[{fr['id']}belly, opacity=0.35] {P(x, y)} circle ({f(r * 0.25)});\n"
    elif pat == "bands":
        # broad wavy yellow bands across a black frog
        cx = (x0 + x1) / 2
        for c, t in kind.get("bands", []):
            top = [(x0 - 0.3 + i * (x1 - x0 + 0.6) / 8, c + t + 0.07 * math.sin(i * 1.7 + c)) for i in range(9)]
            bot = [(x0 - 0.3 + i * (x1 - x0 + 0.6) / 8, c - t + 0.07 * math.sin(i * 2.1 + c * 2)) for i in range(9)]
            s += f"\\filldraw[inked,fill={mark}] plot[smooth] coordinates {{{pts(top)}}} -- plot[smooth] coordinates {{{pts(bot[::-1])}}} -- cycle;\n"
    elif pat == "flanks":
        if region == "body":
            for side in (-1, 1):
                s += ellipse(side * (x1 - 0.05), 1.05, 0.62, 1.0, mark)
                for k in range(3):
                    bx = side * (x1 - 0.42 + k * 0.17)
                    s += leaf((bx, 0.45 + k * 0.15), (bx + side * 0.12, 1.55 - k * 0.05), 0.05, "limebelly", extra=",line width=0.6pt")
        elif region == "haunch":
            s += ellipse((x0 + x1) / 2, y0 + 0.1, (x1 - x0) * 0.45, 0.35, mark)
    else:  # plain: the mark colour as shade under the body
        s += f"\\fill[{mark}, opacity=0.5] {P((x0 + x1) / 2, y0 - 0.25)} ellipse ({f((x1 - x0) * 0.6)} and {f((y1 - y0) * 0.38)});\n"
        if region == "body":
            s += f"\\fill[{mark}, opacity=0.35] {P(0, y1 - 0.1)} ellipse (0.5 and 0.18);\n"
    return s


def frog(fr, arms="down", crown=False, seed=None):
    """A squat frog sitting, seen from the front and a little to one side,
    origin at the middle of its seat. Two big eyes on top of the head, a
    wide smile, two haunches with two hind feet, two front legs with two
    front feet: four legs. About 4.4 wide and 3.8 tall at s = 1."""
    i = fr["id"]
    k = dict(w=1.0, h=1.0, eye=1.0, pupil="round", toe=1.0, mouth=1.0)
    k.update(FROG_KIND.get(i, {}))
    if fr["pattern"] == "bands":
        k["bands"] = [(2.28, 0.26), (1.3, 0.2), (0.45, 0.18)]
    rng = random.Random(seed if seed is not None else sum(ord(c) * (n + 1) for n, c in enumerate(i)))
    skin, belly, feet, eye = i + "skin", i + "belly", i + "feet", i + "eye"
    w, h, e = k["w"], k["h"], k["eye"]
    turn = 0.1  # three-quarter: the face sits a little to the right
    body = [(x * w, y * h) for x, y in [(0, 0), (1.45, 0.1), (1.98, 0.8), (1.88, 1.95), (1.4, 2.85), (0, 3.18),
                                          (-1.4, 2.85), (-1.88, 1.95), (-1.98, 0.8), (-1.45, 0.1)]]
    bpath = blob_path(body)
    s = ""
    # hind feet, behind the haunches, toes splayed outward
    for side in (-1, 1):
        ang = [-62, -36, -10, 16]
        s += foot(side * 1.98 * w, 0.05, [a if side > 0 else 180 - a for a in ang], 0.52 * k["toe"] ** 0.5,
                  0.11, 0.11 * k["toe"], feet, palm=0.22)
    # the body and its pattern
    s += f"\\fill[{skin}] {bpath};\n"
    s += clipped(bpath, frog_marks(fr, rng, "body", k, (-2.0 * w, 0.1, 2.0 * w, 3.2 * h)))
    s += f"\\draw[inked] {bpath};\n"
    # parotoid glands behind the eyes (toads)
    if k.get("parotoid"):
        for side in (-1, 1):
            s += ellipse(side * 1.5 * w, 2.5 * h, 0.42, 0.2, i + "mark", rot=side * -35)
            s += f"\\fill[{belly}, opacity=0.4] {P(side * 1.5 * w - 0.08, 2.55 * h)} ellipse (0.18 and 0.07);\n"
    # the belly and throat
    bel = [(x * w, y * h) for x, y in [(0, 0.12), (0.95, 0.22), (1.15, 0.9), (0.9, 1.6), (0, 1.8), (-0.9, 1.6), (-1.15, 0.9), (-0.95, 0.22)]]
    bel = [(x + turn * 0.5, y) for x, y in bel]
    belpath = blob_path(bel)
    s += f"\\fill[{belly}] {belpath};\n"
    if fr["pattern"] == "bands":
        s += clipped(belpath, frog_marks(fr, rng, "belly", k, (-1.2, 0.1, 1.2, 2.0)))
    if k.get("innards"):
        s += f"\\fill[mushred, opacity=0.55] {P(0.18, 1.3)} circle (0.17);\n"
        s += f"\\fill[white, opacity=0.6] {P(-0.25, 0.9)} ellipse (0.28 and 0.18);\n"
        s += f"\\fill[white, opacity=0.5] {P(0.35, 0.75)} ellipse (0.2 and 0.14);\n"
    s += f"\\draw[inked] {belpath};\n"
    # haunches, the folded hind legs
    for side in (-1, 1):
        hx, hy = side * 1.5 * w, 0.66
        hp = ellipse_path(hx, hy, 0.8 * w, 0.58, side * -18)
        s += f"\\fill[{skin}] {hp};\n"
        s += clipped(hp, frog_marks(fr, rng, "haunch", k, (hx - 0.8 * w, hy - 0.55, hx + 0.8 * w, hy + 0.55)))
        s += f"\\draw[inked] {hp};\n"
    # front legs and feet
    for side in (-1, 1):
        if arms == "up":
            path = [(side * 1.15 * w, 1.75), (side * 1.9 * w, 2.0), (side * 2.2 * w, 2.9)]
            ang = [50, 80, 108, 135] if side > 0 else [130, 100, 72, 45]
        else:
            path = [(side * 1.0 * w + turn, 1.75), (side * 1.18 * w + turn, 0.9), (side * 0.7 * w + turn, 0.2)]
            ang = [-140, -108, -76, -44] if side > 0 else [-40, -72, -104, -136]
        s += limb(path, 0.24, 0.17, skin)
        if fr["pattern"] in ("spots", "blotches", "warts", "dots"):
            mx, my = path[1]
            s += circle(mx, my, 0.09, i + "mark", ",line width=0.5pt")
        if fr["pattern"] == "bands":
            mx, my = path[1]
            s += ellipse(mx, my, 0.22, 0.08, i + "mark", rot=20 * side, extra=",line width=0.6pt")
        hx, hy = path[-1]
        s += foot(hx, hy, ang, 0.36 * k["toe"] ** 0.5, 0.08, 0.09 * k["toe"], feet, palm=0.17)
    # the face: nostrils, blush, the wide smile
    for side in (-1, 1):
        s += f"\\fill[ink] {P(side * 0.2 + turn, 2.72 * h)} circle (0.05);\n"
        s += f"\\fill[blush, opacity=0.6] {P(side * 1.3 * w + turn, 2.12 * h)} ellipse (0.26 and 0.14);\n"
    mw = 1.0 * k["mouth"] * w
    my = 2.3 * h
    s += f"\\draw[ink, line width={f(CUR_INK * 1.25)}pt, line cap=round] {P(-mw + turn, my + 0.08)} .. controls {P(-mw * 0.5 + turn, my - 0.5)} and {P(mw * 0.5 + turn, my - 0.5)} .. {P(mw + turn, my + 0.08)};\n"
    for side in (-1, 1):
        s += f"\\draw[ink, line width={f(CUR_INK)}pt, line cap=round] {P(side * mw + turn - side * 0.08, my + 0.0)} -- {P(side * (mw + 0.08) + turn, my + 0.14)};\n"
    # a crown, set down between the eyes so the eyes sit in front of it

    # the eyes: two, big, on top of the head
    for side in (-1, 1):
        far = 0.94 if side > 0 else 1.0
        ex, ey = side * 0.97 * w + turn, 3.28 * h
        er = 0.74 * e * far
        s += circle(ex, ey, er, skin)
        if k.get("horns"):
            tip = (ex + side * 0.36, ey + er + 0.3)
            s += poly([(ex - side * 0.12, ey + er * 0.95), tip, (ex + side * 0.48, ey + er * 0.62)], skin)
            s += poly([(tip[0] - side * 0.13, tip[1] - 0.2), tip, (tip[0] + side * 0.02, tip[1] - 0.24)], i + "mark", ",line width=0.6pt")
        if k.get("brow"):
            s += f"\\draw[{i}mark, line width={f(CUR_INK * 2.4)}pt, line cap=round] ([shift={{(150:{f(er * 0.92)})}}]{f(ex)},{f(ey)}) arc[start angle=150, end angle=30, radius={f(er * 0.92)}];\n"
        br = 0.57 * e * far
        bx, by = ex, ey + 0.03
        s += circle(bx, by, br, eye)
        s += f"\\draw[white, opacity=0.22, line width={f(CUR_INK * 1.6)}pt] {P(bx, by)} circle ({f(br * 0.78)});\n"
        px, py = bx - side * 0.06 + 0.04, by - 0.04
        if k["pupil"] == "slit":
            s += f"\\fill[pupil] {P(px, py)} ellipse ({f(br * 0.2)} and {f(br * 0.62)});\n"
        elif k["pupil"] == "bar":
            s += f"\\fill[pupil] {P(px, py)} ellipse ({f(br * 0.62)} and {f(br * 0.3)});\n"
        else:
            s += f"\\fill[pupil] {P(px, py)} circle ({f(br * 0.5)});\n"
        s += glint(bx - br * 0.35, by + br * 0.38, br * 0.26)
        s += glint(bx + br * 0.32, by - br * 0.36, br * 0.11)
        s += f"\\draw[inked] {P(bx, by)} circle ({f(br)});\n"
    if crown:
        # the crown perched on the top of the head, over the eyes, tipped
        s += place(turn + 0.05, 3.62 * h + 0.25 * e, 1.0, crown_shape(0, 0, 1.55), rot=-8)
    return s


def frog_leap(fr):

    """A frog in flight, side view, facing right, origin at the middle of
    its body: legs stretched, a near and a far eye on top of the head, a
    near and a far pair of legs (four)."""
    i = fr["id"]
    k = dict(eye=1.0, pupil="round")
    k.update(FROG_KIND.get(i, {}))
    skin, belly, feet, eye, mark = i + "skin", i + "belly", i + "feet", i + "eye", i + "mark"
    s = ""
    # far legs, behind the body
    s += limb([(-0.7, 0.05), (-1.7, 0.4), (-2.55, 0.25)], 0.26, 0.12, skin)
    s += foot(-2.6, 0.25, [150, 175, 200, 222], 0.42, 0.07, 0.07, feet, palm=0.14)
    s += limb([(0.75, -0.2), (1.4, -0.05), (1.9, 0.18)], 0.15, 0.11, skin)
    s += foot(1.95, 0.2, [-10, 20, 50, 80], 0.28, 0.055, 0.065, feet, palm=0.11)
    # far eye, peeking over the head
    er = 0.38 * k["eye"]
    s += circle(1.02, 0.78, er * 0.86, skin)
    s += circle(1.04, 0.8, er * 0.66, eye)
    s += f"\\fill[pupil] {P(1.1, 0.79)} circle ({f(er * 0.3)});\n"
    s += glint(0.98, 0.88, er * 0.16)
    # the body
    body = [(1.42, 0.12), (1.15, 0.58), (0.3, 0.74), (-0.7, 0.55), (-1.25, 0.05), (-0.8, -0.42), (0.3, -0.52), (1.15, -0.28)]
    bp = blob_path(body)
    s += f"\\fill[{skin}] {bp};\n"
    inner = f"\\fill[{belly}] (0.1,-0.62) ellipse (1.45 and 0.42);\n\\draw[inked] (-1.35,-0.32) .. controls (-0.2,-0.12) and (0.7,-0.14) .. (1.45,-0.08);\n"
    if fr["pattern"] == "flanks":
        inner = f"\\fill[{mark}] (-0.4,-0.2) ellipse (1.0 and 0.32);\n" + inner
        for k2 in range(4):
            bx = -1.0 + k2 * 0.42
            inner += leaf((bx, -0.4), (bx + 0.18, 0.02), 0.04, belly, extra=",line width=0.6pt")
    s += clipped(bp, inner)
    s += f"\\draw[inked] {bp};\n"
    # mouth and nostril
    s += f"\\draw[ink, line width={f(CUR_INK * 1.2)}pt, line cap=round] (1.4,0.02) .. controls (1.2,-0.08) and (0.95,-0.06) .. (0.78,0.06);\n"
    s += f"\\fill[ink] (1.3,0.38) circle (0.035);\n"
    s += f"\\fill[blush, opacity=0.55] (0.9,0.05) ellipse (0.17 and 0.09);\n"
    # near eye
    s += circle(0.55, 0.72, er, skin)
    s += circle(0.58, 0.75, er * 0.78, eye)
    if k["pupil"] == "slit":
        s += f"\\fill[pupil] {P(0.64, 0.74)} ellipse ({f(er * 0.14)} and {f(er * 0.46)});\n"
    else:
        s += f"\\fill[pupil] {P(0.64, 0.74)} circle ({f(er * 0.36)});\n"
    s += glint(0.5, 0.85, er * 0.2)
    s += f"\\draw[inked] (0.58,0.75) circle ({f(er * 0.78)});\n"
    # near legs, in front
    s += limb([(-0.55, -0.25), (-1.6, -0.48), (-2.65, -0.35)], 0.3, 0.13, skin)
    s += foot(-2.7, -0.35, [160, 182, 205, 228], 0.48, 0.08, 0.08, feet, palm=0.15)
    s += limb([(0.55, -0.35), (1.15, -0.55), (1.75, -0.35)], 0.16, 0.12, skin)
    s += foot(1.8, -0.33, [-30, 0, 30, 60], 0.3, 0.06, 0.07, feet, palm=0.12)
    return s


def crown_shape(x, y, s):
    """A gold crown, five points with jewels, base centred at (x, y)."""
    out = ""
    pts_ = [(-0.55, 0), (0.55, 0), (0.62, 0.62), (0.36, 0.34), (0.2, 0.78), (0, 0.4), (-0.2, 0.78), (-0.36, 0.34), (-0.62, 0.62)]
    out += poly([(x + px * s, y + py * s) for px, py in pts_], "crown")
    out += rect(x - 0.55 * s, y, x + 0.55 * s, y + 0.2 * s, "crowndark")
    for px, py in ((-0.62, 0.62), (0.62, 0.62), (-0.2, 0.78), (0.2, 0.78)):
        out += circle(x + px * s, y + py * s, 0.07 * s, "crown")
    out += circle(x, y + 0.1 * s, 0.07 * s, "jewel", ",line width=0.6pt")
    out += circle(x - 0.32 * s, y + 0.1 * s, 0.055 * s, "jewelb", ",line width=0.6pt")
    out += circle(x + 0.32 * s, y + 0.1 * s, 0.055 * s, "jewelb", ",line width=0.6pt")
    out += f"\\fill[white, opacity=0.5] {P(x - 0.42 * s, y + 0.3 * s)} -- {P(x - 0.36 * s, y + 0.28 * s)} -- {P(x - 0.5 * s, y + 0.52 * s)} -- cycle;\n"
    return out


# ---- swamp, carnival and course things ------------------------------------

def sky(W, H, top="skyhi", mid="sky", low="skylow", a=0.78, b=0.3):
    s = f"\\fill[{mid}] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[{top}] (0,{f(H * a)}) rectangle {P(W, H)};\n"
    s += f"\\fill[{low}] (0,0) rectangle {P(W, H * b)};\n"
    return s


def cloud(x, y, s, fill="cloud"):
    """Puffs, outlined, as the clouds over Sam_Comm_final's city."""
    puffs = [(-1.0, 0, 0.55), (-0.45, 0.25, 0.7), (0.2, 0.35, 0.8), (0.85, 0.15, 0.6), (1.35, -0.05, 0.45)]
    out = ""
    for dx, dy, r in puffs:
        out += circle(x + dx * s, y + dy * s, r * s, fill)
    out += f"\\fill[{fill}] {P(x - 1.0 * s, y - 0.15 * s)} rectangle {P(x + 1.35 * s, y + 0.05 * s)};\n"
    out += line((x - 1.25 * s, y - 0.15 * s), (x + 1.6 * s, y - 0.15 * s), lw=CUR_INK)
    return out


def tuft(x, y, s, fill="greenB"):
    out = ""
    for a, l in ((-28, 0.55), (-8, 0.8), (14, 0.7), (34, 0.5)):
        tip = (x + math.sin(math.radians(a)) * l * s, y + math.cos(math.radians(a)) * l * s)
        out += leaf((x + a * 0.004 * s, y), tip, 0.07 * s, fill, bend=0.05 * s * (1 if a > 0 else -1))
    return out


def cattail(x, y, h, lean=0.0):
    """A cattail: a stalk, a brown head, a spike."""
    top = (x + lean, y + h)
    out = tube([[(x, y), (x + lean * 0.5, y + h * 0.5), top]], 0.07, "reed")
    hx, hy = x + lean * 0.88, y + h * 0.82
    out += ellipse(hx, hy, 0.13, 0.42, "cattail", rot=-math.degrees(math.atan2(lean, h)))
    return out


def reeds(x, y, s, rng, n=5, fill="reed"):
    out = ""
    for k in range(n):
        bx = x + (k - n / 2) * 0.18 * s
        a = rng.uniform(-22, 22)
        l = rng.uniform(1.2, 2.2) * s
        tip = (bx + math.sin(math.radians(a)) * l, y + math.cos(math.radians(a)) * l)
        out += leaf((bx, y), tip, 0.1 * s, rng.choice([fill, "greenB", "greenC"]), bend=0.15 * s * (1 if a > 0 else -1))
    return out


def mushroom(x, y, s, fuse=False):
    """A red mushroom with white spots. With a fuse: the explosive one."""
    out = poly([(x - 0.32 * s, y), (x + 0.32 * s, y), (x + 0.24 * s, y + 0.9 * s), (x - 0.24 * s, y + 0.9 * s)], "stem")
    out += f"\\filldraw[inked,fill=mushred] {P(x - 1.0 * s, y + 0.75 * s)} .. controls {P(x - 1.0 * s, y + 1.95 * s)} and {P(x + 1.0 * s, y + 1.95 * s)} .. {P(x + 1.0 * s, y + 0.75 * s)} .. controls {P(x + 0.4 * s, y + 0.62 * s)} and {P(x - 0.4 * s, y + 0.62 * s)} .. cycle;\n"
    for dx, dy, r in ((-0.55, 1.05, 0.17), (0.05, 1.42, 0.2), (0.55, 1.1, 0.15), (-0.18, 0.95, 0.1), (0.3, 0.86, 0.08)):
        out += circle(x + dx * s, y + dy * s, r * s, "mushspot", ",line width=0.7pt")
    out += f"\\fill[white, opacity=0.35] {P(x - 0.62 * s, y + 1.32 * s)} ellipse ({f(0.14 * s)} and {f(0.08 * s)});\n"
    for side in (-1, 1):
        out += f"\\fill[ink] {P(x + side * 0.1 * s, y + 0.45 * s)} circle ({f(0.035 * s)});\n"
    if fuse:
        fx, fy = x + 0.05 * s, y + 1.62 * s
        out += curve([(fx, fy), (fx + 0.25 * s, fy + 0.35 * s), (fx + 0.05 * s, fy + 0.65 * s), (fx + 0.3 * s, fy + 0.9 * s)], lw=2.6, colour="ink")
        out += curve([(fx, fy), (fx + 0.25 * s, fy + 0.35 * s), (fx + 0.05 * s, fy + 0.65 * s), (fx + 0.3 * s, fy + 0.9 * s)], lw=1.2, colour="rope")
        out += spark(fx + 0.3 * s, fy + 0.95 * s, 0.32 * s)
    return out


def spark(x, y, r):
    star = []
    for k in range(16):
        rr = r if k % 2 == 0 else r * 0.42
        a = math.pi / 2 + k * math.pi / 8
        star.append((x + math.cos(a) * rr, y + math.sin(a) * rr))
    out = poly(star, "gold")
    out += circle(x, y, r * 0.25, "goldlight", ",line width=0.6pt")
    return out


def web(cx, cy, r, anchors, rng, spokes=9, rings=5, spider=True):
    """A spider web: spokes, a spiral of rings, threads out to its anchors,
    drawn as pale thread over plum ink, and its spider."""
    angs = [2 * math.pi * k / spokes + rng.uniform(-0.12, 0.12) for k in range(spokes)]
    rads = [r * rng.uniform(0.85, 1.05) for _ in angs]
    paths = []
    for a, rr in zip(angs, rads):
        paths.append([(cx, cy), (cx + math.cos(a) * rr, cy + math.sin(a) * rr)])
    for j in range(1, rings + 1):
        ring = []
        for a, rr in zip(angs + angs[:1], rads + rads[:1]):
            q = rr * j / rings
            ring.append((cx + math.cos(a) * q, cy + math.sin(a) * q))
        paths.append(ring)
    for ax, ay in anchors:
        best = min(range(spokes), key=lambda n: math.hypot(cx + math.cos(angs[n]) * rads[n] - ax, cy + math.sin(angs[n]) * rads[n] - ay))
        paths.append([(cx + math.cos(angs[best]) * rads[best], cy + math.sin(angs[best]) * rads[best]), (ax, ay)])
    out = ""
    for p in paths:
        out += f"\\draw[ink, line width={f(CUR_INK * 1.6)}pt, line cap=round, line join=round] {' -- '.join(P(*q) for q in p)};\n"
    for p in paths:
        out += f"\\draw[cream, line width={f(CUR_INK * 0.6)}pt, line cap=round, line join=round] {' -- '.join(P(*q) for q in p)};\n"
    if spider:
        sx, sy = cx + r * 0.12, cy - r * 0.18
        sr = r * 0.11
        for side in (-1, 1):
            for k in range(4):
                a = math.radians(30 - k * 22)
                kx, ky = sx + side * math.cos(a) * sr * 1.9, sy + math.sin(a) * sr * 1.3 + sr * 0.6
                tx, ty = sx + side * math.cos(a) * sr * 2.8, sy + math.sin(a) * sr * 2.2 - sr * 0.8
                out += f"\\draw[ink, line width={f(CUR_INK * 1.1)}pt, line cap=round, line join=round] {P(sx, sy)} -- {P(kx, ky)} -- {P(tx, ty)};\n"
        out += circle(sx, sy - sr * 0.6, sr * 1.1, "spider")
        out += circle(sx, sy + sr * 0.5, sr * 0.75, "spider")
        for side in (-1, 1):
            out += f"\\fill[white] {P(sx + side * sr * 0.3, sy + sr * 0.6)} circle ({f(sr * 0.22)});\n"
            out += f"\\fill[ink] {P(sx + side * sr * 0.3, sy + sr * 0.55)} circle ({f(sr * 0.1)});\n"
    return out


def post(x, y0, y1, w=0.35, fill="wood", cap=True):
    out = rect(x - w / 2, y0, x + w / 2, y1, fill)
    for k in range(int((y1 - y0) / 0.9)):
        yy = y0 + 0.5 + k * 0.9
        out += f"\\draw[wooddark, line width=0.8pt] {P(x - w / 2 + 0.06, yy)} -- {P(x + w / 2 - 0.08, yy + 0.05)};\n"
    if cap:
        out += ellipse(x, y1, w * 0.62, w * 0.22, "woodlight")
    return out


def cypress(x, y, s, rng):
    """A bald cypress: a flared trunk standing in the water, boughs, flat
    irregular crowns, and Spanish moss hanging in strands from each."""
    out = poly([(x - 1.0 * s, y), (x - 0.4 * s, y + 0.8 * s), (x - 0.22 * s, y + 6.4 * s), (x + 0.22 * s, y + 6.4 * s),
                (x + 0.4 * s, y + 0.8 * s), (x + 1.0 * s, y), (x + 0.5 * s, y + 0.22 * s), (x, y + 0.06 * s), (x - 0.5 * s, y + 0.22 * s)], "wooddark")
    for k in range(3):
        out += line((x - 0.14 * s + k * 0.14 * s, y + 0.7 * s), (x - 0.08 * s + k * 0.08 * s, y + 5.6 * s), lw=0.7, colour="ink")
    layers = [(-1.3, 3.7, 1.6), (1.2, 4.4, 1.5), (-0.7, 5.4, 1.55), (0.8, 6.3, 1.25), (-0.1, 7.1, 0.9)]
    for dx, dy, rw in layers:
        bx, by = x + dx * s, y + dy * s
        out += tube([[(x, by - 0.9 * s), (bx, by - 0.1 * s)]], 0.12 * s, "wooddark")
    for dx, dy, rw in layers:
        cx, cy = x + dx * s, y + dy * s
        out += blob([(cx - rw * s, cy), (cx - rw * 0.6 * s, cy + 0.45 * s), (cx - 0.1 * s, cy + 0.55 * s), (cx + rw * 0.55 * s, cy + 0.42 * s),
                     (cx + rw * s, cy + 0.02 * s), (cx + rw * 0.45 * s, cy - 0.25 * s), (cx - rw * 0.5 * s, cy - 0.22 * s)],
                    rng.choice(["cypress", "greenD", "cypressdark"]))
        # Spanish moss: ragged drapes, and loose strands between them
        n = max(2, int(rw * 2))
        for m in range(n):
            mx = cx - rw * 0.65 * s + m * (rw * 1.3 * s) / max(1, n - 1) + rng.uniform(-0.1, 0.1) * s
            ml = rng.uniform(0.7, 1.5) * s
            top = cy - 0.12 * s
            hw = rng.uniform(0.14, 0.22) * s
            drape = [(mx - hw, top), (mx + hw, top), (mx + hw * 0.7, top - ml * 0.55), (mx + hw * 0.35, top - ml * 0.75),
                     (mx + hw * 0.1, top - ml * 0.6), (mx - hw * 0.15, top - ml), (mx - hw * 0.4, top - ml * 0.7), (mx - hw * 0.75, top - ml * 0.8)]
            out += poly(drape, "moss", ",line width=0.7pt")
            sx = mx + (rw * 0.65 * s) / max(1, n - 1)
            if m < n - 1:
                out += curve([(sx, top), (sx + 0.05 * s, top - ml * 0.4), (sx - 0.03 * s, top - ml * 0.75)], lw=1.6, colour="moss")

    return out


def saguaro(x, y, s, arms=((-1, 0.45, 0.75), (1, 0.6, 0.55))):
    """A saguaro: a trunk and upturned arms, round-capped, with ribs."""
    paths = [[(x, y), (x, y + 3.2 * s)]]
    for side, at, up in arms:
        ay = y + at * 3.2 * s
        paths.append([(x, ay), (x + side * 0.8 * s, ay + 0.05 * s), (x + side * 0.9 * s, ay + 0.4 * s), (x + side * 0.9 * s, ay + (0.4 + up * 1.6) * s)])
    out = tube(paths, 0.62 * s, "cactus", tension=0.4)
    out += f"\\draw[cactusdark, line width=0.9pt, line cap=round] {P(x - 0.12 * s, y + 0.2 * s)} -- {P(x - 0.12 * s, y + 3.15 * s)};\n"
    out += f"\\draw[cactusdark, line width=0.9pt, line cap=round] {P(x + 0.13 * s, y + 0.2 * s)} -- {P(x + 0.13 * s, y + 3.1 * s)};\n"
    out += circle(x, y + 3.55 * s, 0.13 * s, "lotus", ",line width=0.6pt")
    return out


def palm(x, y, s, lean=0.6, rng=None):
    """A palm: a curved ringed trunk and drooping fronds."""
    top = (x + lean * s, y + 4.2 * s)
    trunk = [(x, y), (x + lean * 0.15 * s, y + 1.6 * s), (x + lean * 0.55 * s, y + 3.2 * s), top]
    out = tube([trunk], 0.38 * s, "palmtrunk")
    for k in range(1, 9):
        t = k / 9
        px = x + lean * s * t ** 1.6
        py = y + 4.2 * s * t
        out += f"\\draw[wooddark, line width=0.8pt] {P(px - 0.17 * s, py)} -- {P(px + 0.17 * s, py + 0.06 * s)};\n"
    for a, l, droop in ((165, 2.2, -0.5), (135, 2.0, -0.35), (100, 1.4, 0.3), (60, 1.9, 0.4), (25, 2.3, 0.55), (-5, 2.0, 0.5), (200, 1.6, -0.3)):
        tip = (top[0] + math.cos(math.radians(a)) * l * s, top[1] + math.sin(math.radians(a)) * l * s - 0.6 * s)
        out += leaf(top, tip, 0.32 * s, "greenB" if a % 2 else "greenD", bend=droop * s)
        out += f"\\draw[greenE, line width=0.7pt] {P(*top)} .. controls {P((top[0] + tip[0]) / 2 + 0, (top[1] + tip[1]) / 2 + abs(droop) * 0.6 * s)} .. {P(*tip)};\n"
    for dx in (-0.15, 0.12, 0.0):
        out += circle(top[0] + dx * s, top[1] - 0.25 * s + abs(dx) * s, 0.15 * s, "wooddark")
    return out


def bush(x, y, s, rng):
    out = ""
    for dx, dy, r in ((-0.5, 0.3, 0.45), (0.1, 0.5, 0.55), (0.6, 0.28, 0.42)):
        out += circle(x + dx * s, y + dy * s, r * s, rng.choice(GREENS))
    return out


def stone(x, y, w, h, fill="stonedark"):
    return blob([(x - w, y), (x - w * 0.7, y + h * 0.8), (x, y + h), (x + w * 0.8, y + h * 0.7), (x + w, y), (x, y - h * 0.15)], fill)


def bunting(a, b, sag, n, rng):
    """A string of pennants between two points, sagging."""
    (ax, ay), (bx, by) = a, b
    def at(t):
        return (ax + (bx - ax) * t, ay + (by - ay) * t - sag * 4 * t * (1 - t))
    out = curve([at(k / 12) for k in range(13)], lw=CUR_INK)
    for k in range(n):
        t0, t1 = (k + 0.2) / n, (k + 0.8) / n
        p0, p1 = at(t0), at(t1)
        mid = at((t0 + t1) / 2)
        out += poly([p0, p1, (mid[0], mid[1] - 0.55)], SIGNS[k % len(SIGNS)] if k % 2 else "cream", ",line width=0.8pt")
    return out


def tent(x, y, w, h):
    """A striped carnival tent with a pennant."""
    out = ""
    n = 6
    for k in range(n):
        x0 = x - w / 2 + k * w / n
        out += poly([(x0, y), (x0 + w / n, y), (x0 + w / n, y + h * 0.55), (x0, y + h * 0.55)], "red" if k % 2 == 0 else "cream")
    roof = [(x - w / 2 - 0.15, y + h * 0.55), (x + w / 2 + 0.15, y + h * 0.55), (x, y + h)]
    out += poly(roof, "redlight")
    for k in range(1, n):
        rx = x - w / 2 - 0.15 + k * (w + 0.3) / n
        out += line((rx, y + h * 0.55), (x, y + h), lw=0.9)
    for k in range(n):
        x0 = x - w / 2 - 0.15 + k * (w + 0.3) / n
        out += f"\\filldraw[inked,fill={'cream' if k % 2 else 'red'}] {P(x0, y + h * 0.55)} arc[start angle=180, end angle=360, radius={f((w + 0.3) / n / 2)}] -- cycle;\n"
    out += poly([(x - 0.35, y), (x + 0.35, y), (x + 0.35, y + h * 0.4), (x - 0.35, y + h * 0.4)], "windark")
    out += line((x, y + h), (x, y + h + 0.6), lw=1.2)
    out += poly([(x, y + h + 0.6), (x + 0.55, y + h + 0.45), (x, y + h + 0.3)], "gold", ",line width=0.8pt")
    return out


def bell(x, y, s):
    out = f"\\filldraw[inked,fill=bell] {P(x - 0.55 * s, y)} .. controls {P(x - 0.45 * s, y + 0.3 * s)} and {P(x - 0.45 * s, y + 0.9 * s)} .. {P(x, y + 0.95 * s)} .. controls {P(x + 0.45 * s, y + 0.9 * s)} and {P(x + 0.45 * s, y + 0.3 * s)} .. {P(x + 0.55 * s, y)} -- cycle;\n"
    out += circle(x, y - 0.05 * s, 0.13 * s, "crowndark")
    out += circle(x, y + 1.02 * s, 0.1 * s, "bell")
    out += f"\\fill[white, opacity=0.45] {P(x - 0.22 * s, y + 0.45 * s)} ellipse ({f(0.07 * s)} and {f(0.25 * s)});\n"
    return out


def mallet_body():
    """A wooden mallet about the middle of its head; the handle runs along
    +x, the striking face is the head's bottom end."""
    s = ""
    s += rrect(0.45, -0.17, 3.6, 0.17, "woodlight", r=0.08)
    s += rect(3.2, -0.2, 3.65, 0.2, "red")
    s += rrect(-0.62, -0.95, 0.62, 0.95, "wood", r=0.18)
    for yy in (-0.62, 0.62):
        s += rect(-0.66, yy - 0.1, 0.66, yy + 0.1, "metal")
    s += f"\\draw[wooddark, line width=0.8pt] (-0.3,-0.4) .. controls (-0.2,0) .. (-0.32,0.4);\n"
    s += f"\\fill[white, opacity=0.25] (-0.45,-0.45) rectangle (-0.32,0.45);\n"
    return s


def catapult(x, y, s, frog_on=None):
    """The tee: a seesaw plank on a wooden A-frame, a cup at the low end
    for the frog, a red striker pad at the high end, and a carnival
    high-striker post whose mallet is coming down on the pad. (x, y) is
    the foot of the A-frame."""
    out = ""
    # ground shadow
    out += f"\\fill[ink, opacity=0.12] {P(x + 0.6 * s, y - 0.05 * s)} ellipse ({f(4.2 * s)} and {f(0.35 * s)});\n"
    # the striker post, its stripes and bell
    px = x + 5.4 * s
    out += rect(px - 0.75 * s, y - 0.1 * s, px + 0.75 * s, y + 0.35 * s, "wooddark")
    out += rect(px - 0.25 * s, y + 0.35 * s, px + 0.25 * s, y + 4.7 * s, "cream")
    for k in range(6):
        y0 = y + 0.55 * s + k * 0.7 * s
        out += poly([(px - 0.25 * s, y0), (px + 0.25 * s, y0 + 0.2 * s), (px + 0.25 * s, y0 + 0.5 * s), (px - 0.25 * s, y0 + 0.3 * s)], "red", ",line width=0.7pt")
    out += bell(px, y + 4.85 * s, 0.7 * s)
    # the A-frame
    fy = y + 1.45 * s
    out += poly([(x - 0.95 * s, y), (x - 0.65 * s, y), (x + 0.12 * s, fy), (x - 0.12 * s, fy)], "wood")
    out += poly([(x + 0.95 * s, y), (x + 0.65 * s, y), (x - 0.12 * s, fy), (x + 0.12 * s, fy)], "wood")
    out += rect(x - 0.7 * s, y + 0.45 * s, x + 0.7 * s, y + 0.62 * s, "wooddark")
    # the plank, low at the left (the frog's cup), high at the right (the pad)
    a = math.radians(20)
    def on(t, h=0.0):
        return (x + math.cos(a) * t * s - math.sin(a) * h * s, fy + math.sin(a) * t * s + math.cos(a) * h * s)
    plank = [on(-3.4, -0.18), on(3.4, -0.18), on(3.4, 0.18), on(-3.4, 0.18)]
    out += poly(plank, "woodlight")
    for t in (-2.6, -1.2, 1.2, 2.6):
        out += poly([on(t - 0.25, -0.18), on(t + 0.25, -0.18), on(t + 0.25, 0.18), on(t - 0.25, 0.18)], "red", ",line width=0.7pt")
    out += circle(x, fy, 0.2 * s, "metal")
    out += f"\\fill[ink] {P(x, fy)} circle ({f(0.05 * s)});\n"
    # the cup the frog sits in
    c0, c1 = on(-3.35, 0.18), on(-2.0, 0.18)
    cupx, cupy = (c0[0] + c1[0]) / 2, (c0[1] + c1[1]) / 2
    out += poly([on(-3.35, 0.18), on(-2.0, 0.18), on(-2.05, 0.5), on(-3.3, 0.5)], "wood")
    # the striker pad
    out += poly([on(2.55, 0.18), on(3.35, 0.18), on(3.3, 0.48), on(2.6, 0.48)], "mushred")
    out += f"\\fill[white, opacity=0.5] {P(*on(2.75, 0.33))} circle ({f(0.08 * s)});\n"
    pad_top = on(2.95, 0.5)
    if frog_on:
        out += frog_on(cupx - 0.05 * s, cupy + 0.32 * s)
    # the mallet: pinned to the post, its face just above the pad
    pin = (px, y + 3.6 * s)
    head = (pad_top[0], pad_top[1] + 1.15 * s)
    ang = math.degrees(math.atan2(pin[1] - head[1], pin[0] - head[0]))
    L = math.hypot(pin[0] - head[0], pin[1] - head[1]) / s
    body = mallet_body().replace("3.6,0.17", f"{f(L)},0.17").replace("(3.2,-0.2) rectangle (3.65,0.2)", f"({f(L - 0.4)},-0.2) rectangle ({f(L + 0.05)},0.2)")
    out += place(head[0], head[1], s, body, ang)
    out += circle(pin[0], pin[1], 0.16 * s, "metal")
    # swoosh: the arc the head has come down
    R = L * s
    for k, (a0, a1) in enumerate(((168, 186), (163, 190), (171, 184))):
        rr = R + (k - 1) * 0.55 * s
        st = ang + 180 - 48 + k * 5
        out += f"\\draw[ink, line width={f(CUR_INK * 1.4)}pt, line cap=round] ([shift={{({f(st)}:{f(rr)})}}]{f(pin[0])},{f(pin[1])}) arc[start angle={f(st)}, end angle={f(ang + 180 - 16 - k * 4)}, radius={f(rr)}];\n"
    # impact ticks at the pad
    for a in (200, 225, 320, 345):
        u = (math.cos(math.radians(a)), math.sin(math.radians(a)))
        out += line((pad_top[0] + u[0] * 0.75 * s, pad_top[1] + 0.2 * s + u[1] * 0.3 * s), (pad_top[0] + u[0] * 1.2 * s, pad_top[1] + 0.2 * s + u[1] * 0.55 * s), lw=CUR_INK * 1.3)
    return out


def distant_town(rng, x0, x1, y0, hmin, hmax):
    """Far-off buildings with tiny windows and a few antennas."""
    out = ""
    x = x0
    while x < x1:
        w = rng.uniform(0.8, 1.5)
        h = rng.uniform(hmin, hmax)
        wall = rng.choice(WALLS)
        out += rect(x, y0, x + w, y0 + h, wall)
        cols, rows = max(1, int((w - 0.2) / 0.32)), max(1, int((h - 0.3) / 0.42))
        gx = (w - cols * 0.14) / (cols + 1)
        for i in range(cols):
            for j in range(rows):
                wx, wy = x + gx + i * (0.14 + gx), y0 + 0.25 + j * 0.42
                if wy + 0.2 < y0 + h - 0.1:
                    out += f"\\fill[{'window' if rng.random() < 0.5 else 'windark'}] {P(wx, wy)} rectangle {P(wx + 0.14, wy + 0.2)};\n"
        if rng.random() < 0.45:
            ax = x + rng.uniform(0.2, w - 0.2)
            out += line((ax, y0 + h), (ax, y0 + h + rng.uniform(0.4, 0.9)), lw=0.8)
            out += line((ax - 0.15, y0 + h + 0.35), (ax + 0.15, y0 + h + 0.35), lw=0.7)
        elif rng.random() < 0.5:
            bx = x + rng.uniform(0.1, w - 0.5)
            out += rect(bx, y0 + h, bx + 0.35, y0 + h + 0.22, "metal", ",line width=0.7pt")
        x += w + rng.uniform(-0.15, 0.1)
    return out


def volcano(x, y, w, h, rng, smoke=True, lava=True, rock="rockface"):
    """A cone with a notched crater, lava glowing in it and running down,
    smoke rising."""
    out = ""
    top = h
    cone = [(x - w / 2, y), (x - w * 0.22, y + top * 0.62), (x - w * 0.09, y + top), (x - w * 0.04, y + top * 0.95), (x + w * 0.04, y + top * 0.97),
            (x + w * 0.1, y + top), (x + w * 0.25, y + top * 0.58), (x + w / 2, y)]
    out += f"\\filldraw[inked,fill={rock}] {P(*cone[0])} .. controls {P(x - w * 0.3, y + top * 0.3)} .. {P(*cone[1])} -- {P(*cone[2])} -- {P(*cone[3])} -- {P(*cone[4])} -- {P(*cone[5])} -- {P(*cone[6])} .. controls {P(x + w * 0.32, y + top * 0.3)} .. {P(*cone[7])} -- cycle;\n"
    # a lit ridge on one flank
    out += f"\\fill[white, opacity=0.12] {P(x - w * 0.09, y + top)} -- {P(x - w * 0.22, y + top * 0.62)} .. controls {P(x - w * 0.3, y + top * 0.3)} .. {P(x - w / 2, y)} -- {P(x - w * 0.3, y)} -- cycle;\n"
    if lava:
        out += f"\\fill[lava, opacity=0.45] {P(x, y + top)} ellipse ({f(w * 0.16)} and {f(top * 0.08)});\n"
        out += f"\\filldraw[inked,fill=lava] {P(x - w * 0.06, y + top * 0.97)} .. controls {P(x - w * 0.02, y + top * 0.7)} and {P(x + w * 0.06, y + top * 0.5)} .. {P(x + w * 0.02, y + top * 0.12)} -- {P(x + w * 0.07, y + top * 0.12)} .. controls {P(x + w * 0.1, y + top * 0.55)} and {P(x + w * 0.05, y + top * 0.75)} .. {P(x + w * 0.05, y + top * 0.97)} -- cycle;\n"
        out += f"\\draw[lavagold, line width=1.3pt, line cap=round] {P(x, y + top * 0.92)} .. controls {P(x + w * 0.02, y + top * 0.7)} and {P(x + w * 0.07, y + top * 0.5)} .. {P(x + w * 0.045, y + top * 0.16)};\n"
    if smoke:
        for k, (dx, dy, r) in enumerate(((0.0, 0.12, 0.55), (0.06, 0.28, 0.75), (-0.03, 0.45, 0.95))):
            cx, cy = x + dx * w + rng.uniform(-0.1, 0.1), y + top + dy * top * 1.2
            out += cloud(cx, cy, r * w * 0.13, "smoke")
    return out


def lava_river(points, width, glow=True):
    """A river of lava: an orange band with a gold core and a glow."""
    wpt = width * PT_PER_CM
    out = ""
    if glow:
        out += f"\\draw[lava, opacity=0.25, line width={f(wpt * 2.0)}pt, line cap=round, line join=round] plot[smooth] coordinates {{{pts(points)}}};\n"
    out += f"\\draw[ink, line width={f(wpt + 2 * CUR_INK)}pt, line cap=round, line join=round] plot[smooth] coordinates {{{pts(points)}}};\n"
    out += f"\\draw[lava, line width={f(wpt)}pt, line cap=round, line join=round] plot[smooth] coordinates {{{pts(points)}}};\n"
    out += f"\\draw[lavagold, line width={f(wpt * 0.38)}pt, line cap=round, line join=round] plot[smooth] coordinates {{{pts(points)}}};\n"
    return out


def basalt(x, y, cols, rng, w=0.9):
    """A ledge of basalt columns: hexagonal tops, dark faces."""
    out = ""
    for k, hgt in enumerate(cols):
        cx = x + k * w * 0.82
        top = y + hgt
        out += poly([(cx - w / 2, y), (cx + w / 2, y), (cx + w / 2, top), (cx - w / 2, top)], "basalt")
        out += line((cx, y + 0.1), (cx, top - 0.2), lw=0.7, colour="basaltdark")
        hexa = [(cx - w / 2, top), (cx - w / 4, top + 0.2), (cx + w / 4, top + 0.2), (cx + w / 2, top), (cx + w / 4, top - 0.2), (cx - w / 4, top - 0.2)]
        out += poly(hexa, "basalttop")
    return out


def building(rng, x, y0, w, h, wall=None, detail=True, lit_pct=45, windows=True):
    """A building as LongTimeComin draws it: windows, an awning or two,
    rooftop clutter (AC boxes, a water tank, antennas, a billboard of
    squiggles, never words)."""
    wall = wall or rng.choice(WALLS)
    s = rect(x, y0, x + w, y0 + h, wall)
    s += rect(x - 0.08, y0 + h, x + w + 0.08, y0 + h + 0.22, wall)
    if not windows:
        return s
    cols = max(1, int((w - 0.3) / 0.55))
    rows = max(1, int((h - 0.6) / 0.75))
    gap_x = (w - cols * 0.32) / (cols + 1)
    for i in range(cols):
        for j in range(rows):
            wx = x + gap_x + i * (0.32 + gap_x)
            wy = y0 + 0.45 + j * 0.75
            if wy + 0.42 > y0 + h - 0.1:
                continue
            fill = "window" if rng.random() * 100 < lit_pct else "windark"
            s += f"\\filldraw[inked,line width=0.6pt,fill={fill}] {P(wx, wy)} rectangle {P(wx + 0.32, wy + 0.42)};\n"
            if detail and rng.random() < 0.07:
                s += rect(wx - 0.05, wy - 0.28, wx + 0.37, wy - 0.02, "metal", ",line width=0.7pt")
    if detail:
        s += roof(rng, x, y0 + h + 0.22, w)
        if rng.random() < 0.35:
            g = rng.choice(GREENS)
            vx = x + rng.uniform(0.2, w - 0.5)
            s += poly([(vx, y0 + h + 0.1), (vx + 0.35, y0 + h + 0.1), (vx + 0.25, y0 + h - rng.uniform(1.5, 3.5)), (vx + 0.05, y0 + h - rng.uniform(1, 2.5))], g)
    return s


def roof(rng, x, y, w):
    s = ""
    for _ in range(rng.randint(1, 3)):
        kind = rng.choice(["ac", "ac", "tank", "antenna", "billboard", "dish"])
        px = x + rng.uniform(0.15, max(0.3, w - 1.0))
        if kind == "ac":
            s += rect(px, y, px + 0.6, y + 0.4, "metal", ",line width=0.9pt")
            s += circle(px + 0.3, y + 0.2, 0.13, "metaldark", ",line width=0.6pt")
        elif kind == "tank":
            for lx in (0.08, 0.62):
                s += line((px + lx, y), (px + lx, y + 0.5), lw=0.9)
            s += rect(px, y + 0.5, px + 0.7, y + 1.15, "wood", ",line width=0.9pt")
            s += poly([(px - 0.05, y + 1.15), (px + 0.75, y + 1.15), (px + 0.35, y + 1.45)], "wallE", ",line width=0.9pt")
        elif kind == "antenna":
            top = y + rng.uniform(1.0, 2.0)
            s += line((px + 0.3, y), (px + 0.3, top), lw=0.9)
            for k in range(1, 4):
                yy = y + (top - y) * k / 4
                s += line((px + 0.05, yy), (px + 0.55, yy), lw=0.7)
        elif kind == "dish":
            s += line((px + 0.3, y), (px + 0.3, y + 0.35), lw=0.9)
            s += f"\\filldraw[inked,fill=paper,line width=0.8pt] {P(px, y + 0.6)} arc[start angle=150, end angle=330, radius=0.3] -- cycle;\n"
        else:
            c = rng.choice(SIGNS)
            s += line((px + 0.2, y), (px + 0.2, y + 0.6), lw=1)
            s += line((px + 1.2, y), (px + 1.2, y + 0.6), lw=1)
            s += rect(px, y + 0.6, px + 1.4, y + 1.3, c)
            s += f"\\draw[paper, line width=1.5pt, line cap=round, line join=round] {P(px + 0.25, y + 0.95)} -- {P(px + 0.55, y + 1.1)} -- {P(px + 0.85, y + 0.85)} -- {P(px + 1.15, y + 1.05)};\n"
    return s


def car(x, y, s, body="carA", flip=False):
    """A small rounded car, wheels, windows, a headlamp."""
    b = ""
    b += f"\\filldraw[inked,fill={body},rounded corners=0.25cm] (-1.5,0.25) rectangle (1.5,0.95);\n"
    b += f"\\filldraw[inked,fill={body},rounded corners=0.3cm] (-0.95,0.9) -- (-0.65,1.55) -- (0.6,1.55) -- (1.0,0.9) -- cycle;\n"
    b += f"\\filldraw[inked,fill=glass,line width=0.8pt] (-0.75,0.98) -- (-0.55,1.42) -- (-0.08,1.42) -- (-0.08,0.98) -- cycle;\n"
    b += f"\\filldraw[inked,fill=glass,line width=0.8pt] (0.05,0.98) -- (0.05,1.42) -- (0.5,1.42) -- (0.82,0.98) -- cycle;\n"
    b += "\\fill[white, opacity=0.5] (-0.55,1.05) -- (-0.45,1.05) -- (-0.3,1.35) -- (-0.4,1.35) -- cycle;\n"
    b += circle(1.38, 0.62, 0.12, "window", ",line width=0.7pt")
    b += rect(-1.55, 0.35, -1.35, 0.55, "red", ",line width=0.7pt")
    b += line((-0.1, 0.5), (0.2, 0.5), lw=1.2)
    for wx in (-0.9, 0.9):
        b += circle(wx, 0.28, 0.32, "road")
        b += circle(wx, 0.28, 0.13, "metal", ",line width=0.7pt")
    return place(x, y, s, b, flip=flip)


def lamppost(x, y, h):
    out = tube([[(x, y), (x, y + h - 0.4), (x + 0.15, y + h), (x + 0.6, y + h)]], 0.14, "metaldark", tension=0.4)
    out += rect(x - 0.22, y, x + 0.22, y + 0.35, "metaldark")
    out += poly([(x + 0.35, y + h + 0.05), (x + 0.95, y + h + 0.05), (x + 0.8, y + h - 0.25), (x + 0.5, y + h - 0.25)], "metaldark")
    out += ellipse(x + 0.65, y + h - 0.28, 0.16, 0.08, "window", extra=",line width=0.7pt")
    return out


def fountain(x, y, s):
    """A plaza fountain: a wide stone basin of water with the lotus in it,
    a column, an upper bowl and arcs of water."""
    out = ""
    out += f"\\fill[ink, opacity=0.12] {P(x, y - 0.1 * s)} ellipse ({f(3.4 * s)} and {f(0.5 * s)});\n"
    # basin wall
    out += f"\\filldraw[inked,fill=stone] {P(x - 3.2 * s, y + 0.9 * s)} -- {P(x - 3.2 * s, y + 0.1 * s)} arc[start angle=180, end angle=360, x radius={f(3.2 * s)}, y radius={f(0.55 * s)}] -- {P(x + 3.2 * s, y + 0.9 * s)} -- cycle;\n"
    for k in range(7):
        a = math.radians(200 + k * 23)
        bx = x + math.cos(a) * 3.2 * s
        out += line((bx, y + 0.9 * s + math.sin(a) * 0.55 * s), (bx, y + 0.1 * s + math.sin(a) * 0.55 * s), lw=0.8, colour="stonedark")
    out += ellipse(x, y + 0.9 * s, 3.2 * s, 0.75 * s, "stonedark")
    out += ellipse(x, y + 0.95 * s, 2.9 * s, 0.6 * s, "pool")
    for k in range(4):
        out += f"\\draw[poollight, line width=1.2pt] {P(x - 2.2 * s + k * 1.3 * s, y + 0.75 * s + (k % 2) * 0.25 * s)} -- {P(x - 1.6 * s + k * 1.3 * s, y + 0.75 * s + (k % 2) * 0.25 * s)};\n"
    # the column and upper bowl, behind the lotus
    cx = x - 1.3 * s
    out += rect(cx - 0.22 * s, y + 0.95 * s, cx + 0.22 * s, y + 2.6 * s, "stone")
    out += f"\\filldraw[inked,fill=stone] {P(cx - 1.0 * s, y + 2.7 * s)} .. controls {P(cx - 0.8 * s, y + 2.2 * s)} and {P(cx + 0.8 * s, y + 2.2 * s)} .. {P(cx + 1.0 * s, y + 2.7 * s)} -- cycle;\n"
    out += ellipse(cx, y + 2.7 * s, 1.0 * s, 0.22 * s, "pool")
    out += rect(cx - 0.1 * s, y + 2.7 * s, cx + 0.1 * s, y + 3.3 * s, "stone")
    out += circle(cx, y + 3.4 * s, 0.17 * s, "stone")
    for side in (-1, 1):
        out += f"\\draw[ink, line width={f(CUR_INK * 3.4)}pt, line cap=round] {P(cx, y + 3.5 * s)} .. controls {P(cx + side * 0.6 * s, y + 4.3 * s)} and {P(cx + side * 1.0 * s, y + 3.6 * s)} .. {P(cx + side * 1.05 * s, y + 2.8 * s)};\n"
        out += f"\\draw[poollight, line width={f(CUR_INK * 1.6)}pt, line cap=round] {P(cx, y + 3.5 * s)} .. controls {P(cx + side * 0.6 * s, y + 4.3 * s)} and {P(cx + side * 1.0 * s, y + 3.6 * s)} .. {P(cx + side * 1.05 * s, y + 2.8 * s)};\n"
        out += f"\\draw[ink, line width={f(CUR_INK * 3.0)}pt, line cap=round] {P(cx + side * 0.9 * s, y + 2.65 * s)} .. controls {P(cx + side * 1.4 * s, y + 2.6 * s)} .. {P(cx + side * 1.55 * s, y + 1.25 * s)};\n"
        out += f"\\draw[poollight, line width={f(CUR_INK * 1.3)}pt, line cap=round] {P(cx + side * 0.9 * s, y + 2.65 * s)} .. controls {P(cx + side * 1.4 * s, y + 2.6 * s)} .. {P(cx + side * 1.55 * s, y + 1.25 * s)};\n"
    out += lotus(x + 1.15 * s, y + 0.9 * s, 0.95 * s)
    return out


def confetti(rng, n, x0, x1, y0, y1, avoid=None):
    out = ""
    for _ in range(n):
        x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
        if avoid and avoid(x, y):
            continue
        a = rng.uniform(0, 180)
        w, h = rng.uniform(0.18, 0.32), rng.uniform(0.08, 0.14)
        c = rng.choice(CONFETTI)
        if rng.random() < 0.3:
            out += circle(x, y, 0.1, c, ",line width=0.6pt")
        else:
            corners = [rot((x + dx, y + dy), a, (x, y)) for dx, dy in ((-w, -h), (w, -h), (w, h), (-w, h))]
            out += poly(corners, c, ",line width=0.6pt")
    return out


def ripples(rng, n, x0, x1, y0, y1, colour, lw=1.3):
    out = ""
    for _ in range(n):
        x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
        out += f"\\draw[{colour}, line width={f(lw)}pt, line cap=round] {P(x, y)} -- {P(x + rng.uniform(0.5, 1.5), y)};\n"
    return out


def bezier(p0, c, p2, t):
    return ((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * c[0] + t ** 2 * p2[0],
            (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * c[1] + t ** 2 * p2[1])


def bezier_angle(p0, c, p2, t):
    dx = 2 * (1 - t) * (c[0] - p0[0]) + 2 * t * (p2[0] - c[0])
    dy = 2 * (1 - t) * (c[1] - p0[1]) + 2 * t * (p2[1] - c[1])
    return math.degrees(math.atan2(dy, dx))


def fly_line(p0, c, p2, t0, t1, n, s, rng):
    """Flies strung along a flight path, as the course lays them out."""
    out = ""
    for k in range(n):
        t = t0 + (t1 - t0) * k / max(1, n - 1)
        x, y = bezier(p0, c, p2, t)
        out += fly(x + rng.uniform(-0.08, 0.08), y + rng.uniform(-0.1, 0.1), s, rng.uniform(-12, 12))
    return out


def trail(p0, c, p2, t0, t1, n=14):
    out = ""
    for k in range(n):
        ta, tb = t0 + (t1 - t0) * k / n, t0 + (t1 - t0) * (k + 0.5) / n
        out += f"\\draw[ink, line width={f(CUR_INK * 1.3)}pt, line cap=round, opacity=0.55] {P(*bezier(p0, c, p2, ta))} -- {P(*bezier(p0, c, p2, tb))};\n"
    return out


def speed_lines(x, y, ang, n=3, length=1.2, gap=0.35):
    out = ""
    a = math.radians(ang)
    ux, uy = math.cos(a), math.sin(a)
    nx, ny = -uy, ux
    for k in range(n):
        off = (k - (n - 1) / 2) * gap
        sx, sy = x + nx * off - ux * (0.2 + 0.25 * (k % 2)), y + ny * off - uy * (0.2 + 0.25 * (k % 2))
        out += line((sx, sy), (sx - ux * length, sy - uy * length), lw=CUR_INK * 1.3)
    return out


def turtle(x, y, s):
    """A turtle's shell and head poking out of the water."""
    out = ellipse(x + 0.95 * s, y + 0.15 * s, 0.26 * s, 0.2 * s, "greenC")
    out += f"\\fill[ink] {P(x + 1.02 * s, y + 0.2 * s)} circle ({f(0.04 * s)});\n"
    out += f"\\filldraw[inked,fill=greenD] {P(x - 0.8 * s, y)} arc[start angle=180, end angle=0, x radius={f(0.8 * s)}, y radius={f(0.55 * s)}] -- cycle;\n"
    out += poly([(x - 0.25 * s, y + 0.15 * s), (x + 0.25 * s, y + 0.15 * s), (x + 0.32 * s, y + 0.38 * s), (x, y + 0.5 * s), (x - 0.32 * s, y + 0.38 * s)], "greenB", ",line width=0.7pt")
    out += f"\\draw[swampwaterlight, line width=1.2pt] {P(x - 1.1 * s, y - 0.05 * s)} -- {P(x + 1.4 * s, y - 0.05 * s)};\n"
    return out


def dragonfly(x, y, s):
    out = ""
    for side in (-1, 1):
        for dx in (-0.12, 0.12):
            out += ellipse(x + dx * s, y + side * 0.32 * s, 0.1 * s, 0.34 * s, "flywing", rot=side * dx * 120, extra=",fill opacity=0.65,line width=0.7pt")
    out += ellipse(x + 0.35 * s, y, 0.75 * s, 0.07 * s, "teal", extra=",line width=0.8pt")
    out += circle(x - 0.42 * s, y, 0.13 * s, "tealdark", ",line width=0.8pt")
    return out


def tumbleweed(x, y, r, rng):
    out = circle(x, y, r, "sanddark", ",fill opacity=0.35")
    for _ in range(9):
        a0, a1 = rng.uniform(0, 360), rng.uniform(0, 360)
        out += f"\\draw[wooddark, line width=0.9pt] {P(x + math.cos(math.radians(a0)) * r * 0.9, y + math.sin(math.radians(a0)) * r * 0.9)} .. controls {P(x + rng.uniform(-r, r) * 0.5, y + rng.uniform(-r, r) * 0.5)} .. {P(x + math.cos(math.radians(a1)) * r * 0.9, y + math.sin(math.radians(a1)) * r * 0.9)};\n"
    out += f"\\fill[ink, opacity=0.12] {P(x, y - r - 0.05)} ellipse ({f(r * 0.8)} and 0.1);\n"
    return out


# ---- pictures --------------------------------------------------------------

def title():
    """Key art. A frolf tee where a carnival meets the swamp: the catapult
    with a frog in its cup and the mallet coming down on the pad; a frog
    already flying along a line of flies toward the lotus on the pond; a
    spider web between two posts; a red mushroom; a dune, a town and a
    volcano on the horizon. The lower left stays calm for the title box."""
    W, H = 32.0, 18.0
    rng = random.Random(1997)
    hz = 6.6
    s = sky(W, H)
    for (cx, cy, cs) in [(3.5, 15.9, 1.35), (11.5, 16.7, 0.95), (24.5, 16.3, 1.25), (30.5, 14.6, 0.85), (7.5, 12.6, 0.7)]:
        s += cloud(cx, cy, cs)
    # the horizon: a dune to the left, a town, a volcano to the right
    s += hill([(-0.5, hz), (3.5, hz + 1.6), (6.5, hz + 2.4), (9.5, hz + 1.3), (12.5, hz)], hz - 0.2, "sanddark")
    s += hill([(-0.5, hz), (1.5, hz + 1.0), (4.0, hz + 1.4), (7.5, hz + 0.6), (9.5, hz)], hz - 0.2, "sand")
    s += saguaro(6.4, hz + 2.2, 0.32)
    s += volcano(27.8, hz - 0.1, 9.5, 5.4, rng)
    s += distant_town(rng, 15.6, 24.0, hz - 0.1, 1.2, 3.8)
    s += tent(13.2, hz - 0.1, 2.6, 2.3)
    # the meadow and a hedge along the horizon
    s += rect(-0.5, -0.5, W + 0.5, hz, "meadow")
    for bx in [12.0, 14.6, 16.8, 19.3, 21.5, 23.8, 26.0, 28.2, 30.4]:
        s += bush(bx + rng.uniform(-0.3, 0.3), hz - 0.55, rng.uniform(0.75, 1.05), rng)
    s += rect(-0.5, -0.5, W + 0.5, hz - 0.35, "meadow")
    s += f"\\draw[ink, line width={f(CUR_INK)}pt] (-0.5,{f(hz - 0.35)}) -- ({f(W + 0.5)},{f(hz - 0.35)});\n"
    s += f"\\fill[meadowdark] (-0.5,-0.5) rectangle ({f(W + 0.5)},1.3);\n"
    s += f"\\draw[ink, line width={f(CUR_INK)}pt] (-0.5,1.3) .. controls (8,1.6) and (16,1.0) .. ({f(W + 0.5)},1.4);\n"
    # the pond, its pads, reeds and the lotus
    pond = [(19.8, 2.4), (22.0, 4.5), (26.5, 5.1), (30.8, 4.7), (33.0, 4.0), (33.0, -1.0), (19.0, -1.0), (18.6, 0.8)]
    s += blob(pond, "water")
    s += clipped(blob_path(pond), ripples(rng, 14, 19.5, 32, 0.3, 4.6, "waterlight"))
    s += web(29.7, 7.4, 1.35, [(28.1, 9.3), (31.35, 9.3), (28.1, 6.0), (31.35, 6.6)], rng)
    s += post(28.0, 3.6, 9.5, 0.36)
    s += post(31.45, 3.9, 9.4, 0.36)
    for (px, py, rx, ry, n) in [(21.6, 1.1, 0.95, 0.34, 250), (23.6, 2.9, 0.75, 0.27, 300), (29.6, 1.6, 0.95, 0.34, 220), (31.3, 3.0, 0.6, 0.22, 280)]:
        s += lily_pad(px, py, rx, ry, n)
    s += lotus(26.3, 2.65, 1.25)
    for cx_ in (22.3, 22.75, 23.2):
        s += cattail(cx_, 4.4 + (cx_ - 22.3) * 0.3, 2.0 + rng.uniform(-0.3, 0.4), lean=rng.uniform(-0.3, 0.3))
    s += reeds(22.7, 4.5, 0.8, rng, n=6)
    for cx_ in (32.0, 31.6):
        s += cattail(cx_, 0.0, 2.6 + rng.uniform(0, 0.5), lean=-0.4)
    s += reeds(31.4, -0.2, 1.1, rng, n=5)
    # the mushroom and a little one
    s += mushroom(12.4, 0.25, 0.95)
    s += mushroom(13.5, 0.15, 0.48)
    # bunting from a striped pole behind the catapult to the striker post
    s += rect(11.3, 1.8, 11.55, 8.4, "cream")
    for k in range(8):
        s += poly([(11.3, 2.0 + k * 0.8), (11.55, 2.15 + k * 0.8), (11.55, 2.45 + k * 0.8), (11.3, 2.3 + k * 0.8)], "teal", ",line width=0.6pt")
    s += circle(11.425, 8.55, 0.2, "gold")
    s += bunting((11.43, 8.3), (13.2, 8.95), 0.35, 3, rng)

    # the tee
    tomato = FROG["tomato"]
    s += catapult(14.9, 1.55, 1.0, frog_on=lambda fx, fy: place(fx, fy, 0.42, frog(tomato), rot=0))
    # the flight: a dashed trail, the frog, flies leading on to the lotus
    p0, c, p2 = (11.7, 3.4), (18.0, 24.5), (26.2, 3.6)
    s += trail(p0, c, p2, 0.06, 0.27)
    s += fly_line(p0, c, p2, 0.47, 0.88, 8, 0.36, rng)
    t = 0.335
    fx, fy = bezier(p0, c, p2, t)
    ang = bezier_angle(p0, c, p2, t)
    u = (math.cos(math.radians(ang)), math.sin(math.radians(ang)))
    s += speed_lines(fx - u[0] * 3.1, fy - u[1] * 3.1, ang, n=3, length=1.3, gap=0.9)
    s += place(fx, fy, 1.0, frog_leap(FROG["lime"]), rot=ang - 12)
    # small clutter on the meadow, kept off the lower left
    for (tx, ty, ts) in [(12.8, 4.9, 0.7), (17.4, 5.2, 0.6), (18.6, 0.6, 0.8), (16.2, 4.6, 0.5), (14.6, 0.3, 0.6)]:
        s += tuft(tx, ty, ts)
    s += stone(16.6, 0.5, 0.45, 0.32)
    return s, (W, H)


def swamp():
    """Lily Swamp: a greenish-gold sky, murky teal water, lily pads in a
    stepping-stone line, cypresses with moss, cattails, a log, the lotus
    on the water, flies leading the way. The left third is open water."""
    W, H = 32.0, 18.0
    rng = random.Random(7)
    hz = 7.4
    s = sky(W, H, "swampskyhi", "swampsky", "swampskylow")
    for (cx, cy, cs) in [(5.0, 15.6, 1.2), (14.0, 16.5, 0.9), (22.0, 15.4, 1.3), (29.5, 16.6, 0.9)]:
        s += cloud(cx, cy, cs, "swampcloud")
    # the far bank: low cypress shapes and bushes
    for k in range(18):
        bx = -0.5 + k * 1.9 + rng.uniform(-0.3, 0.3)
        s += bush(bx, hz - 0.5, rng.uniform(0.9, 1.3), rng)
    s += cypress(12.5, hz - 0.3, 0.55, rng)
    s += cypress(17.0, hz - 0.3, 0.75, rng)
    s += cypress(22.5, hz - 0.3, 0.6, rng)
    # the water
    s += rect(-0.5, -0.5, W + 0.5, hz, "swampwater")
    s += f"\\fill[swampwaterdark] (-0.5,-0.5) rectangle ({f(W + 0.5)},2.0);\n"
    s += ripples(rng, 22, 0, W - 1, 0.3, hz - 0.4, "swampwaterlight")
    # the big cypress on the right with its moss and knees
    s += cypress(28.8, 3.4, 1.35, rng)
    for kx in (31.4,):
        s += poly([(kx - 0.25, 3.2), (kx + 0.25, 3.2), (kx + 0.08, 4.0), (kx - 0.1, 4.05)], "wooddark")
    # the log
    s += f"\\filldraw[inked,fill=log,rotate around={{-6:(18.5,5.6)}}] (15.6,5.25) rectangle (21.2,5.95);\n"
    s += ellipse(21.2, 5.0, 0.22, 0.36, "logend", rot=-6)
    s += f"\\draw[wooddark, line width=0.8pt] (16.4,5.6) -- (18.6,5.4) (17.6,5.75) -- (20.4,5.48);\n"
    s += tuft(17.2, 5.85, 0.5, "moss")
    s += mushroom(19.8, 5.7, 0.38)
    # stepping-stone pads leading to the lotus
    pads = [(12.4, 1.5, 1.45, 0.5, 260), (15.6, 2.4, 1.2, 0.42, 300), (18.8, 3.4, 1.05, 0.36, 230), (21.6, 4.6, 0.9, 0.3, 280), (22.0, 2.6, 0.95, 0.33, 300)]

    for (px, py, rx, ry, n) in pads:
        s += lily_pad(px, py, rx, ry, n)
    s += lotus(24.9, 3.9, 1.45)
    # a frog waiting on the first pad
    s += place(12.4, 1.55, 0.34, frog(FROG["azure"]))
    # flies along the way
    s += fly_line((13.5, 4.5), (19.0, 11.5), (25.8, 6.6), 0.12, 0.95, 8, 0.34, rng)
    # cattails and reeds at the near edges
    for k, cx_ in enumerate((29.3, 29.9, 30.6, 31.3)):
        s += cattail(cx_, -0.3, 3.2 + rng.uniform(-0.4, 0.6), lean=rng.uniform(-0.5, 0.2))
    s += reeds(30.0, -0.3, 1.3, rng, n=7)
    s += reeds(23.0, 0.0, 0.9, rng, n=5)
    s += cattail(22.5, -0.2, 2.6, lean=0.3)
    # a turtle shell, a dragonfly
    s += turtle(18.6, 0.75, 1.0)
    s += dragonfly(20.5, 9.6, 1.0)
    return s, (W, H)


def desert():
    """Oasis Dunes: warm dunes, saguaros and palms, an oasis pond with the
    lotus, a spider web between two cacti, a mushroom, flies. The left
    third is open dune."""
    W, H = 32.0, 18.0
    rng = random.Random(23)
    s = sky(W, H, "dunehi", "dunesky", "duneskylow", a=0.72, b=0.42)
    s += f"\\fill[sun, opacity=0.45] (27.5,13.6) circle (2.6);\n"
    s += circle(27.5, 13.6, 1.8, "sun")
    for (cx, cy, cs) in [(16.0, 16.2, 1.1), (29.5, 12.6, 1.0), (3.0, 16.6, 0.8), (22.5, 16.4, 0.8)]:
        s += cloud(cx, cy, cs)
    # dunes, back to front
    s += hill([(-0.5, 7.0), (4, 9.0), (9, 8.2), (14, 9.6), (20, 8.4), (26, 10.0), (32.5, 8.6)], -0.5, "sanddeep")
    s += saguaro(21.5, 8.6, 0.42)
    s += saguaro(13.2, 9.3, 0.3, arms=((1, 0.5, 0.6),))
    s += hill([(-0.5, 6.0), (6, 7.6), (12, 6.6), (19, 7.8), (25, 6.8), (32.5, 7.6)], -0.5, "sanddark")
    s += hill([(-0.5, 4.2), (5, 5.4), (11, 4.6), (17, 5.6), (24, 5.0), (32.5, 5.8)], -0.5, "sand")
    s += f"\\draw[sandlight, line width=2pt, line cap=round] (2,5.0) .. controls (5,5.6) and (7,5.4) .. (9,4.9);\n"
    s += f"\\draw[sandlight, line width=2pt, line cap=round] (14,5.4) .. controls (16,5.8) and (18,5.7) .. (20,5.3);\n"
    s += hill([(-0.5, 2.0), (8, 2.6), (16, 2.0), (24, 2.4), (32.5, 2.2)], -0.5, "sandlight")
    # the web between two cacti
    s += saguaro(14.6, 4.9, 1.0, arms=((1, 0.5, 0.45),))
    s += saguaro(19.6, 5.3, 0.9, arms=((-1, 0.55, 0.5),))
    s += web(17.1, 7.2, 1.45, [(15.0, 8.6), (19.2, 8.4), (15.0, 6.0), (19.2, 6.3)], rng)
    # the oasis
    pond = [(21.5, 1.8), (23.5, 3.6), (27.5, 3.9), (31.0, 3.2), (31.6, 1.6), (29.0, 0.6), (24.0, 0.6)]
    s += blob([(x + (x - 26.5) * 0.08, y + (y - 2.2) * 0.25) for x, y in pond], "greenC")
    s += blob(pond, "water")
    s += clipped(blob_path(pond), ripples(rng, 8, 21.5, 31.5, 0.8, 3.6, "waterlight"))
    s += palm(23.0, 3.3, 1.25, lean=-0.8)
    s += palm(30.6, 2.8, 1.4, lean=0.5)
    s += palm(28.0, 3.9, 0.95, lean=0.3)
    s += lily_pad(23.8, 1.6, 0.7, 0.24, 300)
    s += lily_pad(29.2, 1.3, 0.75, 0.26, 230)
    s += lotus(26.5, 1.9, 1.05)
    for k in range(4):
        s += tuft(21.6 + k * 0.5, 1.4 + (k % 2) * 0.3, 0.6, "greenB")
    for k in range(3):
        s += tuft(30.9 + k * 0.4, 0.8 + (k % 2) * 0.4, 0.6, "greenD")
    # the mushroom, rocks, a little cactus
    s += mushroom(20.2, 1.3, 0.75)
    s += stone(13.0, 1.5, 0.6, 0.4, "rock")
    s += stone(13.8, 1.3, 0.35, 0.25, "rock")
    s += saguaro(12.0, 2.0, 0.35, arms=((1, 0.4, 0.4),))
    s += saguaro(25.5, 9.4, 0.25, arms=((-1, 0.5, 0.5),))

    s += tumbleweed(17.6, 2.6, 0.55, rng)
    s += stone(18.6, 2.3, 0.3, 0.2, "rock")
    s += place(13.6, 1.9, 0.38, frog(FROG["goldie"]))
    # flies to the lotus
    s += fly_line((12.5, 9.5), (21.0, 13.5), (26.5, 4.8), 0.05, 0.95, 9, 0.34, rng)
    return s, (W, H)


def city():
    """Main Street: salmon and pink buildings with windows, rooftop AC
    units, antennas and billboards of squiggles, cars on the road, a web
    strung between two lampposts, and at the end a plaza fountain with the
    lotus in its basin. The left third is plain wall and road."""
    W, H = 32.0, 18.0
    rng = random.Random(311)
    s = sky(W, H)
    for (cx, cy, cs) in [(4.0, 16.0, 1.3), (13.5, 16.8, 0.9), (22.0, 16.0, 1.1), (29.5, 16.8, 0.8)]:
        s += cloud(cx, cy, cs)
    base = 4.2
    # back row, then front row (plainer on the left)
    x = -0.5
    while x < W:
        w = rng.uniform(1.6, 2.6)
        h = rng.uniform(6.0, 10.5) if x > 10 else rng.uniform(3.0, 4.5)
        s += building(rng, x, base + 1.0, w, h, detail=x > 10, lit_pct=55, windows=x > 10)
        x += w + rng.uniform(-0.2, 0.1)
    x = -0.4
    while x < W:
        w = rng.uniform(2.0, 3.2)
        h = rng.uniform(3.5, 7.5) if x > 10.5 else rng.uniform(1.6, 2.6)
        if 21.5 < x < 27.5:  # the plaza opens up here
            x += 0.6
            continue
        s += building(rng, x, base, w, h, detail=x > 10.5, lit_pct=40, windows=x > 10.5)
        if x > 10.5:
            # an awning over a shop door
            ax = x + 0.3
            c = rng.choice(["awning", "red", "gold"])
            s += rect(ax + 0.3, base, ax + 0.9, base + 1.1, "windark", ",line width=0.8pt")
            for k in range(4):
                s += poly([(ax + k * 0.35, base + 1.6), (ax + (k + 1) * 0.35, base + 1.6), (ax + (k + 1) * 0.35, base + 1.3), (ax + k * 0.35, base + 1.3)], c if k % 2 == 0 else "cream", ",line width=0.7pt")
        x += w + rng.uniform(0.0, 0.15)
    # the plaza
    s += rect(21.5, base - 0.05, W + 0.5, base + 0.3, "kerb")
    # sidewalk and road
    s += rect(-0.5, base - 0.9, W + 0.5, base, "stone")
    for k in range(17):
        s += line((k * 2.0, base - 0.9), (k * 2.0 + 0.2, base), lw=0.7, colour="stonedark")
    s += rect(-0.5, -0.5, W + 0.5, base - 0.9, "road")
    for k in range(9):
        s += f"\\fill[roadline] {P(0.8 + k * 3.8, 1.45)} rectangle {P(2.4 + k * 3.8, 1.65)};\n"
    s += fountain(26.0, base - 0.2, 1.05)
    # lampposts with the web between them
    s += lamppost(14.2, base - 0.6, 5.4)
    s += lamppost(19.6, base - 0.6, 5.4)
    s += web(16.9, base + 3.4, 1.55, [(14.2, base + 4.6), (19.6, base + 4.6), (14.2, base + 2.0), (19.6, base + 2.2)], rng)
    # a fire hydrant, a bench, a pigeon-less sign post
    s += rect(11.2, base - 0.6, 11.55, base + 0.05, "red", ",line width=0.9pt")
    s += circle(11.375, base + 0.1, 0.2, "red")
    s += rect(20.6, base - 0.55, 21.9, base - 0.35, "wood", ",line width=0.9pt")
    s += rect(20.6, base - 0.2, 21.9, base + 0.05, "wood", ",line width=0.9pt")
    s += place(12.95, base - 0.75, 0.36, frog(FROG["bumble"]))

    # traffic

    s += car(16.0, 0.35, 0.95, "carB")
    s += car(23.5, 1.6, 0.85, "carC", flip=True)
    s += car(30.4, 0.3, 0.95, "carD")
    # flies along the street to the fountain
    s += fly_line((12.0, 9.0), (19.5, 13.8), (26.9, 7.9), 0.0, 0.92, 9, 0.34, rng)
    return s, (W, H)


def volcano_hole():
    """Lava Flow: a dusky orange-plum sky, dark rock, a smoking volcano,
    glowing lava rivers, basalt ledges, and a cool crater pool holding the
    lotus. The left third is plain dark rock and sky."""
    W, H = 32.0, 18.0
    rng = random.Random(66)
    s = sky(W, H, "duskhi", "dusk", "dusklow", a=0.72, b=0.5)
    for (cx, cy, cs) in [(4.5, 15.8, 1.1), (13.0, 16.6, 0.8)]:
        s += cloud(cx, cy, cs, "duskcloud")
    for k in range(6):
        s += f"\\fill[lava, opacity=0.06] (22.5,10.5) ellipse ({f(11 - k * 1.5)} and {f(5.5 - k * 0.7)});\n"

    s += hill([(-0.5, 7.6), (5, 8.3), (10, 7.7), (14, 8.2), (18, 7.6), (24, 8.0), (28, 8.6), (32.5, 7.8)], -0.5, "rockface")
    s += volcano(22.5, 5.4, 17.0, 9.7, rng, smoke=False)
    for (cx, cy, r) in [(22.6, 15.9, 1.0), (23.8, 17.2, 1.35), (26.5, 17.6, 1.6), (20.5, 17.3, 1.1)]:
        s += cloud(cx, cy, r, "smoke")
    for k in range(10):
        ex, ey = 22.5 + rng.uniform(-2.5, 3.5), 15.3 + rng.uniform(0, 2.4)
        s += circle(ex, ey, rng.uniform(0.05, 0.1), "lavagold", ",line width=0.5pt")
    # far ridge and the rock floor

    s += hill([(-0.5, 6.2), (8, 6.8), (16, 6.0), (24, 6.9), (32.5, 6.3)], -0.5, "basalt")
    # lava rivers from the cone across the floor
    s += lava_river([(24.0, 7.2), (23.2, 5.8), (20.0, 4.6), (16.5, 4.3), (13.0, 3.2), (10.5, 2.6)], 0.55)
    s += lava_river([(25.5, 6.9), (27.5, 5.6), (29.0, 3.6), (31.0, 2.8), (33.0, 2.6)], 0.45)
    s += lava_river([(18.8, 4.5), (18.2, 2.5), (17.0, 0.8), (16.5, -0.5)], 0.4)
    # basalt ledges, the steps of the hole
    s += basalt(12.0, 3.8, [1.0, 1.4, 1.2], rng)
    s += place(12.74, 5.32, 0.3, frog(FROG["bufo"]))

    s += basalt(19.5, 5.4, [0.8, 1.1, 0.9, 1.3], rng, w=0.8)
    s += basalt(28.5, 4.2, [1.0, 1.5, 1.1, 0.8], rng)
    s += basalt(20.2, 0.6, [1.2, 0.9, 1.4], rng, w=1.0)
    # the crater pool with the lotus
    s += ellipse(25.0, 1.9, 3.3, 1.2, "basaltdark")
    s += ellipse(25.0, 2.0, 2.8, 0.9, "pool")
    s += f"\\draw[poollight, line width=1.4pt] (23.4,1.6) -- (24.4,1.6) (25.8,2.4) -- (26.6,2.4);\n"
    s += lotus(25.1, 1.75, 1.2)
    # glowing cracks in the floor
    for (cx_, cy_) in [(14.5, 2.2), (21.8, 3.4), (28.0, 0.9), (17.5, 5.6), (30.8, 4.6), (12.5, 0.8)]:
        pts_ = [(cx_ + k * 0.45, cy_ + rng.uniform(-0.18, 0.18)) for k in range(5)]
        s += f"\\draw[lava, opacity=0.35, line width=5pt, line cap=round, line join=round] {' -- '.join(P(*q) for q in pts_)};\n"
        s += f"\\draw[ink, line width=2.6pt, line cap=round, line join=round] {' -- '.join(P(*q) for q in pts_)};\n"
        s += f"\\draw[lavagold, line width=1.0pt, line cap=round, line join=round] {' -- '.join(P(*q) for q in pts_)};\n"
    s += stone(22.6, 0.4, 0.5, 0.3, "rockface")
    s += stone(18.8, 6.1, 0.35, 0.22, "rockface")
    s += stone(27.6, 4.0, 0.45, 0.3, "rockface")
    # rocks, embers, a hardy mushroom

    s += stone(14.8, 0.6, 0.6, 0.4, "rockface")
    s += stone(30.6, 0.6, 0.7, 0.45, "rockface")
    s += mushroom(31.2, 1.1, 0.5)
    for k in range(14):
        ex, ey = rng.uniform(12, 31), rng.uniform(0.5, 9)
        s += f"\\fill[lavagold, opacity=0.85] {P(ex, ey)} circle ({f(rng.uniform(0.04, 0.08))});\n"
    # flies across the lava to the pool
    s += fly_line((13.0, 7.6), (19.0, 12.5), (24.8, 4.4), 0.02, 0.95, 9, 0.34, rng)
    return s, (W, H)


def crown():
    """End of the round: the winner wearing the gold crown, sitting on a
    giant lotus throne on the pond, flies circling, confetti, the other
    frogs cheering below. The left third is calm sky and water."""
    W, H = 32.0, 18.0
    rng = random.Random(8)
    s = sky(W, H)
    # rays behind the throne, kept off the left third
    for d0, d1 in ((-28, -15), (5, 18), (35, 48), (65, 78), (95, 108), (125, 136)):
        a0 = math.radians(d0)
        a1 = math.radians(d1)
        s += f"\\fill[skyhi, opacity=0.6] (21.5,9.5) -- {P(21.5 + math.cos(a0) * 20, 9.5 + math.sin(a0) * 20)} -- {P(21.5 + math.cos(a1) * 20, 9.5 + math.sin(a1) * 20)} -- cycle;\n"
    for (cx, cy, cs) in [(4.0, 15.4, 1.3), (9.0, 12.2, 0.7), (29.0, 16.4, 1.0)]:
        s += cloud(cx, cy, cs)
    # the far bank and the pond
    for k in range(16):
        s += bush(-0.5 + k * 2.2 + rng.uniform(-0.3, 0.3), 5.0, rng.uniform(0.8, 1.1), rng)
    s += rect(-0.5, -0.5, W + 0.5, 5.3, "water")
    s += ripples(rng, 16, 0.5, W - 1, 0.5, 5.0, "waterlight")
    # bunting across the top
    s += bunting((11.5, 15.8), (32.5, 15.2), 1.6, 18, rng)
    # the throne: a giant lotus with the winner in it
    s += lily_pad(21.5, 3.6, 5.6, 1.4, 300)
    s += place(21.5, 3.7, 3.5, lotus_body(pad=False, part="back"))
    # flies circling the crown: the back half of the ring behind the winner
    ring = []
    for k in range(10):
        a = 2 * math.pi * k / 10

        ring.append((math.sin(a), 21.6 + math.cos(a) * 4.4, 12.0 + math.sin(a) * 0.85, 0.4 * (0.85 - 0.15 * math.sin(a)), math.degrees(a) * 0.1))
    s += f"\\draw[ink, line width=0.8pt, dash pattern=on 3pt off 5pt, opacity=0.5] (21.6,12.0) ellipse (4.4 and 0.85);\n"
    for depth, x_, y_, sc, r_ in ring:
        if depth > 0:
            s += fly(x_, y_, sc, r_)
    s += place(21.55, 6.25, 0.88, frog(FROG["lime"], crown=True))
    s += place(21.5, 3.7, 3.5, lotus_body(pad=False, part="front"))
    for depth, x_, y_, sc, r_ in ring:
        if depth <= 0:
            s += fly(x_, y_, sc, r_)
    # the others cheering below
    crowd = ["azure", "bumble", "tomato", "bufo", "goldie", "chomp", "glassy"]
    xs = [12.7, 15.65, 18.6, 21.55, 24.5, 27.45, 30.4]
    s += confetti(rng, 150, 11.5, 32.0, 2.6, 17.5, avoid=lambda x, y: (abs(x - 21.6) < 3.2 and 5 < y < 12.6))
    for n, (fid, cx_) in enumerate(zip(crowd, xs)):
        s += lily_pad(cx_, 0.45, 1.35, 0.38, 280 + n * 20)
        s += place(cx_, 0.5, 0.47, frog(FROG[fid], arms="up"))

    return s, (W, H)


# ---- portraits and icons ------------------------------------------------------

def portrait(fid):
    """A frog's portrait for the picker: the frog sitting on a lily pad on
    a pale disc. Colours and pattern from data/frogs.json."""
    W, H = 6.0, 6.0
    fr = FROG[fid]
    s = circle(3.0, 3.0, 2.88, "paper")
    s += f"\\fill[water, opacity=0.35] (3.0,1.15) ellipse (2.55 and 0.75);\n"
    s += lily_pad(3.0, 1.2, 2.35, 0.62, 285)
    s += f"\\fill[ink, opacity=0.18] (3.0,1.08) ellipse (1.9 and 0.28);\n"
    k = FROG_KIND.get(fid, {})
    sc = min(1.05 / max(1.0, k.get("w", 1.0) * 0.98), 4.35 / (3.28 * k.get("h", 1.0) + 0.74 * k.get("eye", 1.0)))
    s += place(3.0, 1.15, sc, frog(fr))
    return s, (W, H)


def icon(kind):
    """A 6 cm square icon on a pale disc."""
    W, H = 6.0, 6.0
    rng = random.Random(sum(map(ord, kind)))
    s = circle(3.0, 3.0, 2.88, "paper")
    if kind == "jump":
        # a coiled spring with a frog's foot on top, and an up arrow
        for k in range(5):
            y = 0.95 + k * 0.42
            s += f"\\draw[ink, line width={f(CUR_INK * 3.2)}pt, line cap=round] {P(2.0, y)} .. controls {P(2.3, y + 0.38)} and {P(3.7, y + 0.38)} .. {P(4.0, y + 0.2)};\n"
            s += f"\\draw[metal, line width={f(CUR_INK * 1.6)}pt, line cap=round] {P(2.0, y)} .. controls {P(2.3, y + 0.38)} and {P(3.7, y + 0.38)} .. {P(4.0, y + 0.2)};\n"
        s += rrect(1.8, 0.6, 4.2, 0.95, "metaldark", r=0.1)
        s += rrect(1.8, 3.0, 4.2, 3.3, "metaldark", r=0.1)
        s += foot(3.0, 3.35, [35, 75, 105, 145], 0.7, 0.12, 0.15, "limeskin", palm=0.3)
        s += poly([(3.0, 5.65), (4.05, 4.6), (3.45, 4.6), (3.45, 4.15), (2.55, 4.15), (2.55, 4.6), (1.95, 4.6)], "pad")
    elif kind == "wings":
        # a pair of white feathered wings, as on a winged cap: a curved arm
        # along the top, round-ended flight feathers hanging below it
        def W_(x, y, side):
            return (3.0 + side * x * 0.9, y)

        for side in (-1, 1):
            for (a, b, c) in (((0.75, 2.85), (1.25, 2.35), (1.55, 1.95)),
                              ((1.05, 3.25), (1.75, 2.85), (2.15, 2.55)),
                              ((1.4, 3.65), (2.15, 3.45), (2.55, 3.25)),
                              ((1.75, 4.05), (2.35, 4.05), (2.6, 4.0))):
                s += limb([W_(*a, side), W_(*b, side), W_(*c, side)], 0.22, 0.27, "white", cap=True)
                s += f"\\draw[metal, line width=0.9pt, line cap=round] {P(*W_(*b, side))} -- {P(*W_(c[0] - 0.12, c[1] + 0.06, side))};\n"
            arm = [W_(0.45, 2.6, side), W_(0.9, 3.4, side), W_(1.7, 4.25, side), W_(2.5, 4.75, side), W_(2.75, 4.6, side),
                   W_(2.1, 4.05, side), W_(1.4, 3.35, side), W_(0.85, 2.6, side)]
            s += blob(arm, "white", tension=0.6)
            for k in range(3):
                x_, y_ = W_(0.85 + k * 0.45, 3.05 + k * 0.42, side)
                s += f"\\draw[metal, line width=0.9pt] {P(x_ - 0.14, y_)} arc[start angle=200, end angle=340, radius=0.15];\n"
        s += circle(3.0, 2.75, 0.55, "red")
        s += glint(2.8, 2.95, 0.15)
    elif kind == "mushroom":
        s += mushroom(2.9, 0.75, 1.75, fuse=True)
    elif kind == "fly":
        s += fly(3.0, 2.55, 2.9)
    elif kind == "tongue":
        # a frog's head at the lower left, its tongue lashing out in a curl
        tng = [(1.9, 1.8), (2.8, 2.3), (3.9, 3.0), (4.6, 3.9), (4.3, 4.6), (3.6, 4.5), (3.5, 3.9), (4.0, 3.6)]
        s += tube([tng], 0.32, "lotus", tension=0.55)
        s += circle(4.05, 3.62, 0.3, "lotus")
        s += f"\\fill[white, opacity=0.5] (3.95,3.72) circle (0.08);\n"
        s += f"\\draw[lotusdark, line width=1.2pt, line cap=round] plot[smooth, tension=0.55] coordinates {{(2.2,1.98) (2.8,2.33) (3.85,3.05)}};\n"
        s += f"\\begin{{scope}}\\clip (3,3) circle (2.88);\n"
        s += circle(0.9, 0.9, 1.6, "limeskin")
        s += circle(1.25, 2.55, 0.6, "limeskin")
        s += circle(1.3, 2.6, 0.43, "limeeye")
        s += f"\\fill[pupil] (1.36,2.56) ellipse (0.09 and 0.28);\n"
        s += glint(1.18, 2.75, 0.1)
        s += f"\\draw[inked] (1.3,2.6) circle (0.43);\n"
        s += f"\\filldraw[inked,fill=mushred] (1.6,1.45) .. controls (1.9,1.6) and (2.1,1.9) .. (2.15,2.0) .. controls (1.9,2.0) and (1.6,1.85) .. (1.45,1.7) -- cycle;\n"
        s += "\\end{scope}\n"
        for a in (40, 70, 100):
            s += line(rot((4.95, 4.25), a - 70, (4.3, 4.2)), rot((5.35, 4.3), a - 70, (4.3, 4.2)), lw=CUR_INK * 1.2)
    elif kind == "lotus":
        s += lotus(3.0, 1.75, 1.75)
    elif kind == "web":
        s += web(3.0, 3.0, 2.3, [], rng, spokes=10, rings=6)
    elif kind == "lilypad":
        s += f"\\fill[water, opacity=0.5] (3.0,2.7) ellipse (2.6 and 1.35);\n"
        s += lily_pad(3.0, 2.85, 2.3, 1.15, 300)
        s += circle(2.0, 3.3, 0.14, "waterlight", ",line width=0.7pt")
        s += glint(1.96, 3.34, 0.05)
        s += f"\\draw[waterlight, line width=1.4pt] (1.0,1.3) -- (1.8,1.3) (3.8,1.2) -- (4.9,1.2);\n"
    elif kind == "stroke":
        s += place(2.35, 3.55, 1.25, mallet_body()
.replace("3.6,0.17", "2.9,0.17").replace("(3.2,-0.2) rectangle (3.65,0.2)", "(2.5,-0.2) rectangle (2.95,0.2)"), -55)

    return s, (W, H)


PICTURES = {
    "title": title,
    "holes/swamp": swamp,
    "holes/desert": desert,
    "holes/city": city,
    "holes/volcano": volcano_hole,
    "crown": crown,
}
for fr in FROGS:
    PICTURES[f"frogs/{fr['id']}"] = (lambda i=fr["id"]: portrait(i))
for k in ("jump", "wings", "mushroom", "fly", "tongue", "lotus", "web", "lilypad", "stroke"):
    PICTURES[f"icons/{k}"] = (lambda k=k: icon(k))

# dpi for each folder: 32 cm key art at 150 dpi is 1890 px wide, as
# LongTimeComin's title; 6 cm portraits and icons at 160 dpi are 378 px.
DPI = {"": 150, "holes": 150, "frogs": 160, "icons": 160}


def check_no_text(name, tex):
    """The no-words rule, checked on the source before it is compiled."""
    for bad in ("\\node", "\\text", "label=", "\\pgftext", "\\put"):
        if bad in tex:
            raise SystemExit(f"{name}: {bad} found; pictures carry no words")


def document(body, size, ink):
    W, H = size
    head = PREAMBLE + f"\\newcommand{{\\Ink}}{{{ink}pt}}\n\\tikzset{{inked/.style={{draw=ink, line width=\\Ink, line join=round, line cap=round}}}}\n"
    return head + "\\begin{document}\n\\begin{tikzpicture}\n" + f"\\clip (0,0) rectangle {P(W, H)};\n" + body + "\\end{tikzpicture}\n\\end{document}\n"


def main():
    global CUR_INK
    only = set(sys.argv[1:])
    for name, make in PICTURES.items():
        if only and name not in only and name.split("/")[0] not in only:
            continue
        folder = name.split("/")[0] if "/" in name else ""
        CUR_INK = INK.get(folder, 1.15)
        body, size = make()
        tex = document(body, size, CUR_INK)
        check_no_text(name, tex)
        dpi = DPI.get(folder, 150)
        with tempfile.TemporaryDirectory() as tmp:
            t = Path(tmp)
            (t / "art.tex").write_text(tex)
            r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "art.tex"], cwd=t, capture_output=True, text=True)
            if r.returncode != 0:
                print(r.stdout[-3000:])
                raise SystemExit(f"{name}: pdflatex failed")
            # pictures in a folder sit on the page's own panels: clear corners
            clear = ["-transp"] if folder else []
            subprocess.run(["pdftocairo", "-png", *clear, "-r", str(dpi), "-singlefile", str(t / "art.pdf"), str(t / "art")], check=True)
            out = OUT / f"{name}.png"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(t / "art.png", out)
            src = SRC / f"{name}.tex"
            src.parent.mkdir(parents=True, exist_ok=True)
            src.write_text(tex)
        print("wrote", out)


if __name__ == "__main__":
    main()
