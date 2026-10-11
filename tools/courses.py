#!/usr/bin/env python3
"""Write data/course.json: the five holes, laid out from three routes each.

The playtest (BRIEF.md, round 3) asked for wide holes of at least par 5 with
three distinct routes that cross, cliffs, plateaus, hills, tunnels, rivers,
waterfalls, giant bouncy mushrooms, power-ups out in the rough, webs set into
the ground, point bubbles, and a fungus course; and for snails and
whirlpools in the swamp and antlions in the desert. Writing that much by
hand is error-prone, so every hole is composed here from one plan:

- Three routes run from the tee to the lotus. A heads out left, B right,
  and both swing across the hole so they cross each other at the middle,
  on top of a terrace (a "bench"). C runs straight down the middle and goes
  under that terrace through a tunnel, so the routes cross over one another.
- Each route gets its own character per course (a river with waterfalls and
  whirlpools, a line of mesas with cliffs, a bumper corridor, a dune field
  with antlions...), fairway stripes, point bubbles every so often, fly lines,
  and webs set into the ground at landing distance.
- Power-ups and the bigger bubbles are scattered in the rough between the
  routes by a seeded roll, so finding them means leaving the fairway.
- The lotus floats in a pond at the end: a short shot swims and is played
  again from a lily pad, so the last approach is a chip, not a gamble.

Everything is in metres here and written in centimetres. The same seed gives
the same file.

    python3 tools/courses.py
"""
import json
import math
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "data" / "course.json"

TEE = (0.0, 0.0)
LOTUS = (0.0, -398.0)
BOUNDS = (-135.0, -450.0, 135.0, 30.0)
CROSS_Z = -247.0  # where A and B cross, over C's tunnel

A = [(0, -8), (-66, -85), (-72, -180), (46, -300), (0, -392)]
B = [(0, -8), (66, -85), (72, -180), (-46, -300), (0, -392)]
C = [(0, -8), (0, -130), (0, -265), (0, -392)]
ROUTES = {"A": A, "B": B, "C": C}


def cm(v):
    return int(round(v * 100))


def P(*vals):
    return [cm(v) for v in vals]


def seg_dist(p, a, b):
    ax, az = a
    bx, bz = b
    dx, dz = bx - ax, bz - az
    l2 = dx * dx + dz * dz
    t = 0 if l2 == 0 else max(0, min(1, ((p[0] - ax) * dx + (p[1] - az) * dz) / l2))
    cx, cz = ax + dx * t, az + dz * t
    return math.hypot(p[0] - cx, p[1] - cz)


def route_dist(p, route):
    return min(seg_dist(p, route[i], route[i + 1]) for i in range(len(route) - 1))


def along(route, every, start=20.0, stop=20.0):
    """Points every `every` metres along a route, skipping its ends."""
    pts = []
    total = 0.0
    for i in range(len(route) - 1):
        a, b = route[i], route[i + 1]
        n = math.hypot(b[0] - a[0], b[1] - a[1])
        d = (every - total % every) % every
        while d <= n:
            pts.append((a[0] + (b[0] - a[0]) * d / n, a[1] + (b[1] - a[1]) * d / n, total + d))
            d += every
        total += n
    return [(x, z) for x, z, s in pts if start <= s <= total - stop]


def point_at(route, metres):
    total = 0.0
    for i in range(len(route) - 1):
        a, b = route[i], route[i + 1]
        n = math.hypot(b[0] - a[0], b[1] - a[1])
        if total + n >= metres:
            t = (metres - total) / n
            return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        total += n
    return route[-1]


def rough_spots(rng, n, min_route=22.0, keep_out=(), margin=10.0):
    """Points in the rough: inside the bounds, away from every route's
    fairway and from anything listed in keep_out (x, z, radius)."""
    x0, z0, x1, z1 = BOUNDS
    out = []
    tries = 0
    while len(out) < n and tries < n * 400:
        tries += 1
        p = (rng.uniform(x0 + margin, x1 - margin), rng.uniform(z0 + 40, -30))
        if min(route_dist(p, r) for r in ROUTES.values()) < min_route:
            continue
        if any(math.hypot(p[0] - kx, p[1] - kz) < kr for kx, kz, kr in keep_out):
            continue
        if any(math.hypot(p[0] - q[0], p[1] - q[1]) < 18 for q in out):
            continue
        out.append(p)
    return out


def yaw_of(dx, dz):
    """The heading (degrees) that faces along (dx, dz): facing(yaw) is (-sin, -cos)."""
    return int(round(math.degrees(math.atan2(-dx, -dz)))) % 360


class Hole:
    def __init__(self, id, name, theme, par, base, rough, seed):
        self.rng = random.Random(seed)
        self.d = {
            "id": id, "name": name, "theme": theme, "par": par,
            "bounds": P(*BOUNDS), "base": cm(base), "rough": P(*rough),
            "tee": P(*TEE), "lotus": P(*LOTUS),
            "hills": [], "plateaus": [], "ridges": [], "mesas": [], "benches": [],
            "waters": [], "falls": [], "tunnels": [], "fairways": [], "paints": [],
            "pads": [], "webs": [], "mushrooms": [], "bumpers": [], "items": [],
            "bubbles": [], "flies": [], "shot_flies": [], "snails": [],
            "antlions": [], "whirlpools": [], "props": [], "scatter": [],
            "buildings": [], "lanes": [],
        }
        self.base = base
        self.keep = [(TEE[0], TEE[1], 22), (LOTUS[0], LOTUS[1], 34)]

    def add(self, key, item):
        self.d[key].append(item)

    def fairways(self, width=26):
        for route in ROUTES.values():
            for a, b in zip(route, route[1:]):
                self.add("fairways", P(a[0], a[1], b[0], b[1], width))

    def crossing(self, height, top_width=30, edge=6, rock=True):
        """The terrace A and B cross on, and C's tunnel under it."""
        self.add("benches", P(-150, CROSS_Z, 150, CROSS_Z, top_width, height, edge))
        self.add("tunnels", P(0, CROSS_Z - top_width / 2 - edge, CROSS_Z + top_width / 2 + edge, 14, 5.5, height - 4.5))
        self.keep.append((0, CROSS_Z, 40))
        # a bubble worth the risk in the tunnel's mouth, and one over the crossing
        self.add("bubbles", P(0, CROSS_Z + 26, 2.2) + [50])
        self.add("bubbles", P(0, CROSS_Z, height + 5) + [100])

    def lotus_pond(self, r=24, level=-0.4, pads=6):
        self.add("waters", {"disc": P(LOTUS[0], LOTUS[1], r), "level": cm(self.base + level)})
        for k in range(pads):
            a = 2 * math.pi * k / pads + 0.3
            self.add("pads", P(LOTUS[0] + 11 * math.cos(a), LOTUS[1] + 11 * math.sin(a), 2.0))

    def river(self, points, width, level, whirls=(), pads=()):
        """A river through `points` at a surface level above the base;
        consecutive points at different levels drop over a waterfall."""
        for (a, la), (b, lb) in zip(points, points[1:]):
            if la == lb:
                self.add("waters", {"line": P(a[0], a[1], b[0], b[1], width), "level": cm(self.base + la)})
        for (a, la), (b, lb) in zip(points, points[1:]):
            if la != lb:
                # a fall between two pools: the upper reach ends at a, the lower starts at b
                mx, mz = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
                self.add("falls", P(mx, mz, self.base + la, self.base + lb, width * 0.9) + [yaw_of(b[0] - a[0], b[1] - a[1])])
        for x, z, r in whirls:
            self.add("whirlpools", P(x, z, r))
        for x, z in pads:
            self.add("pads", P(x, z, 2.2))
        for (a, _), (b, _) in zip(points, points[1:]):
            self.keep.append(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, width))

    def route_bubbles(self, route, every=30, height=3.5, points=10):
        for x, z in along(route, every, start=30, stop=40):
            self.add("bubbles", P(x, z, height) + [points])

    def webs(self, spots):
        for x, z, r in spots:
            self.add("webs", P(x, z, r))
            self.keep.append((x, z, r + 6))

    def tee_flies(self, power=74):
        # one line of flies along the opening arc of each route
        for route in ROUTES.values():
            self.add("shot_flies", P(TEE[0], TEE[1], route[1][0], route[1][1]) + [power, 9])
        for route in ROUTES.values():
            a = point_at(route, 150)
            b = point_at(route, 205)
            self.add("flies", P(a[0], a[1], b[0], b[1]) + [8, 300, 900])

    def rough_things(self, items=7, bubbles=10, mushrooms=4):
        spots = rough_spots(self.rng, items + bubbles + mushrooms, keep_out=self.keep)
        kinds = ["jump", "wings", "jump", "wings", "jump", "wings", "jump", "wings", "jump"]
        for i, (x, z) in enumerate(spots[:items]):
            self.add("items", [kinds[i % len(kinds)], cm(x), cm(z)])
        for x, z in spots[items:items + bubbles]:
            self.add("bubbles", P(x, z, 3.0 + self.rng.random() * 3) + [self.rng.choice([25, 25, 50])])
        for x, z in spots[items + bubbles:]:
            self.add("mushrooms", P(x, z))
        # power-ups along the fairways too, a little off the line
        for name, route in ROUTES.items():
            p = point_at(route, 110 if name != "C" else 190)
            side = 12 if name != "B" else -12
            self.add("items", ["jump" if name == "C" else "wings", cm(p[0] + side), cm(p[1])])

    def bumpers(self, spots):
        for x, z, r, h in spots:
            self.add("bumpers", P(x, z, r, h))
            self.keep.append((x, z, r + 8))

    def prop(self, kind, x, z, size=100, yaw=0):
        self.add("props", {"kind": kind, "at": P(x, z), "size": size, "yaw": yaw})

    def scatter(self, kind, count, seed, inside=True):
        entry = {"kind": kind, "count": count, "seed": seed}
        if inside:
            x0, z0, x1, z1 = BOUNDS
            entry["region"] = P(x0 + 6, z0 + 6, x1 - 6, z1 - 12)
        self.add("scatter", entry)

    def done(self):
        return {k: v for k, v in self.d.items() if v != []}


def swamp():
    h = Hole("swamp", "Lily Swamp", "swamp", 5, 1.0, (0.35, 14), 11)
    h.fairways()
    h.crossing(9)
    h.lotus_pond()
    # A: the cypress ridge, a line of hummocks with cliffs
    h.add("mesas", P(-96, -140, 16, 6, 5))
    h.add("mesas", P(-60, -215, 12, 4.5, 4))
    h.add("hills", P(-105, -60, 26, 4))
    # B: the river, from a raised marsh over a waterfall, down the right and
    # across the middle to the lotus pond, with whirlpools
    h.add("mesas", P(104, -58, 24, 7, 5))
    h.river([((108, -40), 6.2), ((88, -70), 6.2), ((80, -80), -0.4), ((70, -140), -0.4),
             ((62, -205), -0.4), ((30, -300), -0.4), ((10, -372), -0.4)], 12,
            None or 0, whirls=[(70, -150, 5.5), (40, -280, 5.0)], pads=[(66, -120), (58, -230)])
    h.add("whirlpools", P(-12, -388, 4.0))
    # C: the bog, with snails creeping across the middle
    h.add("snails", P(-28, -60, 28, -60) + [120, 260])
    h.add("snails", P(25, -170, -25, -170) + [90, 300])
    h.add("snails", P(-70, -120, -78, -205) + [80, 240])
    h.add("hills", P(30, -330, 22, 3))
    h.webs([(-68, -112, 4), (6, -95, 6), (-40, -300, 8), (58, -330, 3.5)])
    h.bumpers([(-30, -150, 4.5, 7), (95, -250, 5.5, 8), (-95, -330, 4, 6.5)])
    for route in ROUTES.values():
        h.route_bubbles(route)
    h.tee_flies()
    h.rough_things()
    for x, z in [(-20, -330), (25, -120), (-60, -260)]:
        h.prop("reeds", x, z)
    h.prop("log", -110, -280, 100, 0)
    h.scatter("cypress", 46, 11)
    h.scatter("reeds", 50, 12)
    h.scatter("rock", 12, 13)
    h.scatter("cypress", 120, 14, inside=False)
    h.scatter("tree", 60, 15, inside=False)
    return h.done()


def desert():
    h = Hole("desert", "Oasis Dunes", "desert", 5, 2.0, (0.6, 22), 21)
    h.fairways(28)
    h.crossing(8)
    h.lotus_pond(r=25)
    h.add("paints", {"colour": "oasis_grass", "line": P(-20, LOTUS[1], 20, LOTUS[1], 70)})
    # A: the dune field, ridges across the route, and antlion pits in the troughs
    for z, hh in [(-60, 3.5), (-115, 4.5), (-165, 4.0), (-205, 3.5)]:
        h.add("ridges", P(-135, z - 6, -20, z + 6, 12, hh))
    for x, z, r in [(-64, -88, 7), (-80, -140, 6.5), (-70, -190, 7.5), (-25, -215, 6), (30, -330, 6.5)]:
        h.add("antlions", P(x, z, r))
        h.keep.append((x, z, r + 6))
    # B: the canyon, mesas with sheer sides; a spring on the tallest one
    # pours over a waterfall into the wadi the middle route follows
    h.add("mesas", P(92, -118, 22, 11, 4))
    h.add("mesas", P(62, -205, 15, 13, 4))
    h.add("mesas", P(110, -320, 20, 9, 4))
    h.river([((96, -118), 10.3), ((76, -118), 10.3), ((66, -122), -0.4), ((18, -150), -0.4),
             ((4, -200), -0.4), ((-6, -320), -0.4), ((0, -372), -0.4)], 10, 0)
    # C: the wadi floor
    h.add("hills", P(-40, -350, 20, 4))
    h.webs([(10, -80, 5), (-50, -260, 7), (75, -270, 4), (-100, -330, 9)])
    h.bumpers([(-105, -230, 4.5, 7.5), (40, -40, 4, 6)])
    for route in ROUTES.values():
        h.route_bubbles(route, height=4.0)
    h.tee_flies()
    h.rough_things()
    for x, z, s in [(-26, -380, 110), (26, -384, 100), (-22, -420, 95), (24, -418, 115), (36, -400, 90)]:
        h.prop("palm", x, z, s)
    h.prop("skull", -90, -95)
    h.scatter("cactus", 40, 21)
    h.scatter("rock", 16, 22)
    h.scatter("dune_grass", 50, 23)
    h.scatter("cactus", 90, 24, inside=False)
    h.scatter("boulder", 50, 25, inside=False)
    return h.done()


def fungus():
    h = Hole("fungus", "Toadstool Hollow", "fungus", 5, 1.5, (0.45, 12), 31)
    h.fairways(26)
    h.crossing(9.5)
    h.lotus_pond()
    # A: the web grove, webs of every size set in the moss
    h.webs([(-66, -100, 4), (-74, -150, 6.5), (-60, -205, 9), (-20, -290, 5), (90, -40, 3.5)])
    # B: the bumper run, giant blue mushrooms along both sides like a pinball table
    h.bumpers([(88, -70, 5, 8), (48, -95, 4.5, 7), (95, -140, 6, 10), (52, -165, 4.5, 7.5),
               (98, -205, 5.5, 9), (-40, -40, 4, 6.5), (-105, -250, 6, 10), (60, -345, 5, 8),
               (-60, -350, 5, 8.5), (25, -215, 4, 6)])
    # C: the hollow: a mossy cliff on the left pours a stream down a waterfall
    # into the middle, which runs on to the lotus pond
    h.add("mesas", P(-108, -195, 24, 8, 5))
    h.river([((-112, -190), 7.2), ((-92, -192), 7.2), ((-80, -196), -0.4), ((-30, -215), -0.4),
             ((-8, -300), -0.4), ((-4, -372), -0.4)], 10, 0)
    h.add("hills", P(40, -300, 24, 4))
    for x, z in [(-30, -70), (35, -120), (-15, -175), (75, -255)]:
        h.add("mushrooms", P(x, z))
    for route in ROUTES.values():
        h.route_bubbles(route)
    h.tee_flies()
    h.rough_things(mushrooms=6)
    h.scatter("shelf_tree", 40, 31)
    h.scatter("glowcap", 40, 32)
    h.scatter("toadstool", 40, 33)
    h.scatter("puffball", 18, 34)
    h.scatter("fungus_tree", 30, 35, inside=False)
    h.scatter("shelf_tree", 80, 36, inside=False)
    return h.done()


def city():
    h = Hole("city", "Main Street", "city", 5, 0.0, (0, 10), 41)
    # three streets: avenues left and right, Main Street down the middle;
    # A runs down the left avenue and B the right, and they swap sides over
    # an elevated crossing; Main Street goes under it
    streets_x = [-62, 0, 62]
    cross_z = [-70, -160, -330]
    for x in streets_x:
        h.add("paints", {"colour": "sidewalk", "line": P(x, 25, x, -365, 30)})
        h.add("paints", {"colour": "asphalt", "line": P(x, 25, x, -365, 20)})
    for z in cross_z:
        h.add("paints", {"colour": "asphalt", "line": P(-135, z, 135, z, 18)})
    h.add("paints", {"colour": "plaza", "line": P(0, -365, 0, -440, 120)})
    h.add("paints", {"colour": "asphalt", "line": P(-62, -8, 62, -8, 18)})
    h.crossing(9, top_width=22, edge=3)
    h.lotus_pond(r=20)
    # the plaza's fountain: a terrace behind the lotus pours a cascade into it
    h.add("mesas", P(0, -432, 18, 6, 3))
    h.add("waters", {"disc": P(0, -434, 6), "level": cm(5.2)})
    h.add("falls", P(0, -416, 5.2, -0.4, 7) + [yaw_of(0, 1)])
    # a canal along the middle cross street, with bridges' worth of bubbles over it
    h.add("waters", {"line": P(-135, -160, 135, -160, 10), "level": cm(-0.6)})
    h.add("whirlpools", P(95, -160, 4))
    # blocks of buildings between the streets
    rng = h.rng
    xs = [(-135, -77), (-47, -15), (15, 47), (77, 135)]
    zs = [(-55, 25), (-145, -85), (-235, -175), (-315, -262), (-360, -345)]
    for bx0, bx1 in xs:
        for bz1, bz0 in [(z[1], z[0]) for z in zs]:
            z0, z1 = bz1, bz0
            lo, hi = min(z0, z1), max(z0, z1)
            if hi - lo < 10:
                continue
            n = 2 if hi - lo > 40 else 1
            step = (hi - lo) / n
            for k in range(n):
                za, zb = lo + k * step + 1.5, lo + (k + 1) * step - 1.5
                if za < -8 < zb and bx0 < 62 and bx1 > -62 and -30 < (bx0 + bx1) / 2 < 30:
                    continue
                height = rng.randint(12, 34)
                h.add("buildings", P(bx0 + 2, za, bx1 - 2, zb, height))
    for z, sp, colours in [(-73, 900, ["car_red", "car_teal"]), (-67, 1100, ["car_yellow", "car_blue"]),
                           (-333, 1000, ["car_cream", "car_red"]), (-327, 1200, ["car_teal", "car_yellow"])]:
        x = [-140, 140] if sp % 200 == 100 else [140, -140]
        h.add("lanes", {"z": cm(z), "x": P(*x), "speed": sp, "count": 3, "colours": colours})
    # routes: fairway-free, but bubbles, webs, flies and bumpers follow them
    h.webs([(-62, -115, 4), (62, -115, 4), (0, -60, 5), (0, -300, 6)])
    h.bumpers([(-30, -205, 4, 6.5), (30, -120, 4, 6.5), (100, -380, 5, 8), (-100, -380, 5, 8)])
    city_routes = {"A": [(0, -8), (-62, -30), (-62, -230), (62, -265), (62, -350), (0, -392)],
                   "B": [(0, -8), (62, -30), (62, -230), (-62, -265), (-62, -350), (0, -392)],
                   "C": C}
    for route in city_routes.values():
        h.route_bubbles(route, every=28, height=4)
        # the routes as fairways (under the street paint, which wins), so the
        # CPU follows the streets
        for a, b in zip(route, route[1:]):
            h.add("fairways", P(a[0], a[1], b[0], b[1], 18))
    for name, route in city_routes.items():
        h.add("shot_flies", P(TEE[0], TEE[1], route[1][0], route[1][1] - 40) + [72, 9])
    h.add("flies", P(-62, -120, -62, -200) + [8, 300, 800])
    h.add("flies", P(62, -120, 62, -200) + [8, 300, 800])
    h.add("flies", P(0, -150, 0, -215) + [7, 300, 600])
    for p in [(-62, -150), (62, -150), (0, -110), (-30, -300), (30, -300), (0, -205)]:
        h.add("items", ["wings" if p[0] == 0 else "jump", cm(p[0] + 8), cm(p[1])])
    for x, z in [(-62, -100), (62, -100), (0, -280)]:
        h.add("mushrooms", P(x + 6, z))
    for x in streets_x:
        for z in range(-20, -360, -45):
            for side in (-1, 1):
                h.prop("lamp", x + side * 13, z)
    h.prop("planter", -30, -380)
    h.prop("planter", 30, -380)
    h.prop("fountain", LOTUS[0], LOTUS[1], 300)
    h.scatter("tower", 110, 41, inside=False)
    h.scatter("tree", 30, 42, inside=False)
    d = h.done()
    d["base"] = 0
    return d


def volcano():
    h = Hole("volcano", "Lava Flow", "volcano", 6, 3.0, (0.7, 18), 51)
    h.fairways(26)
    h.crossing(10)
    h.lotus_pond(r=22)
    # lava rivers across the hole, and a lava fall off a basalt shelf
    h.add("mesas", P(-100, -40, 22, 8, 4))
    h.add("waters", {"kind": "lava", "line": P(-96, -42, -84, -50, 7), "level": cm(3.0 + 7.2)})
    h.add("falls", P(-80, -54, 3.0 + 7.2, 3.0 - 0.5, 6) + [yaw_of(1, -1)])
    for (a, b) in [((-74, -58), (140, -112)), ((140, -165), (-140, -150)), ((-140, -320), (60, -350))]:
        h.add("waters", {"kind": "lava", "line": P(a[0], a[1], b[0], b[1], 16), "level": cm(3.0 - 0.5)})
        h.keep.append(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, 30))
    # A: basalt mesas with cliffs; B: the ridge road; C: under the crossing
    for x, z, r, hh in [(-90, -110, 16, 7), (-78, -200, 13, 9), (-50, -280, 12, 6)]:
        h.add("mesas", P(x, z, r, hh, 3))
    for x, z, r, hh in [(95, -205, 28, 6), (110, -300, 22, 8)]:
        h.add("hills", P(x, z, r, hh))
    h.webs([(70, -80, 4), (-20, -125, 6), (40, -200, 8), (-40, -365, 3.5)])
    h.bumpers([(15, -60, 4, 6.5), (-110, -230, 5, 8), (95, -365, 5.5, 9)])
    for route in ROUTES.values():
        h.route_bubbles(route)
    h.tee_flies()
    h.rough_things()
    h.prop("volcano", -30, -900, 100)
    h.prop("vent", -95, -110)
    h.prop("vent", 100, -205)
    h.scatter("basalt", 30, 51)
    h.scatter("boulder", 16, 52)
    h.scatter("basalt", 80, 53, inside=False)
    h.scatter("boulder", 60, 54, inside=False)
    return h.done()


ABOUT = (
    "Written by tools/courses.py: edit that and run it, not this. The five holes in the order a "
    "round plays them. Lengths in centimetres; x is to the right of the tee and z toward it (the "
    "lotus is at negative z). bounds: x0, z0, x1, z1. base: the ground's height before shapes; "
    "rough: a seeded unevenness's height and scale. hazard: [kind, level] over the whole hole. "
    "waters: rivers (line: x0, z0, x1, z1, width) and ponds (disc: x, z, radius) at a level, "
    "water or lava; the core cuts each one's bed into the ground. falls: x, z, top, bottom, width, "
    "heading. hills and plateaus: [x, z, radius, height]; mesas: [x, z, radius, height, cliff]; "
    "ridges: [x0, z0, x1, z1, radius, height]; benches: flat-topped terraces [x0, z0, x1, z1, "
    "width, height, cliff]. tunnels: a rock roof over a cut along z [x, z0, z1, width, clearance, "
    "thickness]. fairways: [x0, z0, x1, z1, width]. paints: a theme paint along a line. pads: lily "
    "pads [x, z, radius]. webs: set into the ground over a pit [x, z, radius]. mushrooms: "
    "explosive [x, z]. bumpers: giant blue mushrooms [x, z, cap radius, cap height]. items: "
    "[jump or wings, x, z]. bubbles: [x, z, height, points]. flies: lines [x0, z0, x1, z1, how "
    "many, height, arc]; shot_flies: along the arc a shot flies [x0, z0, toward x, toward z, "
    "power, how many]. snails: [x0, z0, x1, z1, speed cm/s, size]. antlions: [x, z, radius]. "
    "whirlpools: [x, z, radius]. props and scatter: scenery. buildings: [x0, z0, x1, z1, height]. "
    "lanes: cars along z between two x."
)


def main():
    holes = [swamp(), desert(), fungus(), city(), volcano()]
    OUT.write_text(json.dumps({"_about": ABOUT, "holes": holes}, indent=1) + "\n")
    for h in holes:
        counts = {k: len(v) for k, v in h.items() if isinstance(v, list) and k not in ("bounds", "tee", "lotus", "rough")}
        print(h["id"], "par", h["par"], {k: n for k, n in counts.items() if n})


if __name__ == "__main__":
    main()
