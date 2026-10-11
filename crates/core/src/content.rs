//! The data files the core reads, parsed and cross-checked here, so a
//! mistake in a data file is an error the page can show rather than a hole
//! that quietly breaks. Lengths in the files are whole centimetres, speeds
//! cm/s, colours `#rrggbb`; they become Q16 metres here.

use crate::balance::{cm, mps};
use crate::fx::ONE;
use serde::Deserialize;
use std::collections::BTreeMap;

pub type Rgb = [i32; 3];
/// A point on the ground: x, z.
pub type Xz = (i32, i32);

/// A colour written `#rrggbb`, as Q16 channels.
pub fn parse_hex(s: &str) -> Result<Rgb, String> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("not a colour: {s}"));
    }
    let ch = |i: usize| i32::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) * ONE / 255;
    Ok([ch(0), ch(2), ch(4)])
}

/// A Q16 colour as the page writes it in CSS.
pub fn hex(c: Rgb) -> String {
    let ch = |v: i32| (v.clamp(0, ONE) * 255 / ONE).clamp(0, 255);
    format!("#{:02x}{:02x}{:02x}", ch(c[0]), ch(c[1]), ch(c[2]))
}

// ---- frogs ----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Plain,
    Spots,
    Bands,
    Warts,
    Blotches,
    Dots,
    Flanks,
}

impl Pattern {
    fn parse(s: &str) -> Result<Pattern, String> {
        Ok(match s {
            "plain" => Pattern::Plain,
            "spots" => Pattern::Spots,
            "bands" => Pattern::Bands,
            "warts" => Pattern::Warts,
            "blotches" => Pattern::Blotches,
            "dots" => Pattern::Dots,
            "flanks" => Pattern::Flanks,
            _ => return Err(format!("frogs: unknown pattern {s}")),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogDef {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub skin: Rgb,
    pub belly: Rgb,
    pub pattern: Pattern,
    pub mark: Rgb,
    pub eye: Rgb,
    pub feet: Rgb,
}

#[derive(Deserialize)]
struct FrogsFile {
    frogs: Vec<FrogFile>,
}

#[derive(Deserialize)]
struct FrogFile {
    id: String,
    name: String,
    kind: String,
    skin: String,
    belly: String,
    pattern: String,
    mark: String,
    eye: String,
    feet: String,
}

pub fn parse_frogs(text: &str) -> Result<Vec<FrogDef>, String> {
    let f: FrogsFile = serde_json::from_str(text).map_err(|e| format!("frogs: {e}"))?;
    let mut out = Vec::new();
    for g in f.frogs {
        let c = |s: &str| parse_hex(s).map_err(|e| format!("frog {}: {e}", g.id));
        out.push(FrogDef {
            skin: c(&g.skin)?,
            belly: c(&g.belly)?,
            pattern: Pattern::parse(&g.pattern)?,
            mark: c(&g.mark)?,
            eye: c(&g.eye)?,
            feet: c(&g.feet)?,
            id: g.id,
            name: g.name,
            kind: g.kind,
        });
    }
    if out.len() < 4 {
        return Err("frogs: four players need at least four frogs".into());
    }
    Ok(out)
}

// ---- palette and themes ---------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub sky: Rgb,
    pub fog: Rgb,
    pub fog_near: i32,
    pub fog_far: i32,
    pub light: Rgb,
    pub window_dark: Rgb,
    pub ink: Rgb,
    pub ground: [Rgb; 2],
    pub fairway: Rgb,
    pub shore: Rgb,
    pub bed: Rgb,
    pub rock: Rgb,
    pub far: Rgb,
    pub hazard: Rgb,
    /// rivers and ponds (the hazard is lava on a lava course)
    pub water: Rgb,
    /// how brightly the hazard's surface glows, out of 100 (lava glows)
    pub hazard_glow: i32,
    pub paints: BTreeMap<String, Rgb>,
    /// building walls (the city), in turn
    pub walls: Vec<Rgb>,
    pub greens: Vec<Rgb>,
}

#[derive(Deserialize)]
struct ThemeFile {
    sky: String,
    fog: String,
    fog_near: i32,
    fog_far: i32,
    light: String,
    window_dark: String,
    ink: String,
    ground: [String; 2],
    fairway: String,
    shore: String,
    bed: String,
    rock: String,
    far: String,
    hazard: String,
    #[serde(default)]
    water: Option<String>,
    hazard_glow: i32,
    #[serde(default)]
    paints: BTreeMap<String, String>,
    #[serde(default)]
    walls: Vec<String>,
    greens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    named: BTreeMap<String, Rgb>,
    pub themes: BTreeMap<String, Theme>,
}

#[derive(Deserialize)]
struct PaletteFile {
    named: BTreeMap<String, String>,
    themes: BTreeMap<String, ThemeFile>,
}

impl Palette {
    pub fn parse(text: &str) -> Result<Palette, String> {
        let f: PaletteFile = serde_json::from_str(text).map_err(|e| format!("palette: {e}"))?;
        let mut named = BTreeMap::new();
        for (k, v) in &f.named {
            named.insert(k.clone(), parse_hex(v).map_err(|e| format!("palette {k}: {e}"))?);
        }
        let mut themes = BTreeMap::new();
        for (k, t) in &f.themes {
            let c = |s: &str| parse_hex(s).map_err(|e| format!("theme {k}: {e}"));
            let list = |v: &[String]| v.iter().map(|s| c(s)).collect::<Result<Vec<_>, _>>();
            let mut paints = BTreeMap::new();
            for (pk, pv) in &t.paints {
                paints.insert(pk.clone(), c(pv)?);
            }
            let theme = Theme {
                sky: c(&t.sky)?,
                fog: c(&t.fog)?,
                fog_near: cm(t.fog_near),
                fog_far: cm(t.fog_far),
                light: c(&t.light)?,
                window_dark: c(&t.window_dark)?,
                ink: c(&t.ink)?,
                ground: [c(&t.ground[0])?, c(&t.ground[1])?],
                fairway: c(&t.fairway)?,
                shore: c(&t.shore)?,
                bed: c(&t.bed)?,
                rock: c(&t.rock)?,
                far: c(&t.far)?,
                hazard: c(&t.hazard)?,
                water: c(t.water.as_deref().unwrap_or(&t.hazard))?,
                hazard_glow: t.hazard_glow,
                paints,
                walls: list(&t.walls)?,
                greens: list(&t.greens)?,
            };
            if theme.greens.is_empty() {
                return Err(format!("theme {k}: needs at least one green"));
            }
            themes.insert(k.clone(), theme);
        }
        Ok(Palette { named, themes })
    }

    /// A named colour. A name the file lacks is drawn in black, and the test
    /// `every_colour_the_core_names_is_in_the_palette` keeps that from happening.
    pub fn c(&self, name: &str) -> Rgb {
        self.named.get(name).copied().unwrap_or([0, 0, 0])
    }

    pub fn has(&self, name: &str) -> bool {
        self.named.contains_key(name)
    }
}

// ---- the course -------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Jump,
    Wings,
}

/// Where a body of water (or lava) lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// a river: along a line, this wide
    Line { a: Xz, b: Xz, w: i32 },
    /// a pond: round
    Disc { x: i32, z: i32, r: i32 },
    /// the whole hole, wherever the ground is below its level (lava)
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Water {
    pub kind: HazardKind,
    pub region: Region,
    pub level: i32,
}

/// A waterfall, drawn where a river drops: where, from what height to what
/// height, how wide, facing which way (degrees).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fall {
    pub x: i32,
    pub z: i32,
    pub top: i32,
    pub bottom: i32,
    pub w: i32,
    pub yaw: i32,
}

/// A rock bridge over a cut: the cut runs along z at x, from z0 to z1, this
/// wide; the roof starts this far above the cut's floor and is this thick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tunnel {
    pub x: i32,
    pub z0: i32,
    pub z1: i32,
    pub w: i32,
    pub clearance: i32,
    pub thickness: i32,
}

/// A snail that crawls to and fro along a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snail {
    pub a: Xz,
    pub b: Xz,
    pub speed: i32,
    pub size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// a rounded hill (or, with a negative height, a hollow)
    Hill { x: i32, z: i32, r: i32, h: i32 },
    /// flat on top for half its radius, then rounding down
    Plateau { x: i32, z: i32, r: i32, h: i32 },
    /// a rounded bank along a line (or a channel, with a negative height)
    Ridge {
        a: (i32, i32),
        b: (i32, i32),
        w: i32,
        h: i32,
    },
    /// flat on top to `r`, then a cliff down over `edge`
    Mesa {
        x: i32,
        z: i32,
        r: i32,
        h: i32,
        edge: i32,
    },
    /// a long flat-topped terrace along a line, `w` wide, cliffs over `edge`
    Bench { a: Xz, b: Xz, w: i32, h: i32, edge: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paint {
    pub colour: String,
    pub a: (i32, i32),
    pub b: (i32, i32),
    pub w: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prop {
    pub kind: String,
    pub x: i32,
    pub z: i32,
    /// size, out of 100
    pub size: i32,
    /// heading in degrees
    pub yaw: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scatter {
    pub kind: String,
    pub count: i32,
    pub seed: i32,
    /// `None`: in the band round the hole's bounds, out of play
    pub region: Option<[i32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A spider web strung flat across a pit in the ground, like a trampoline.
pub struct Web {
    pub x: i32,
    pub z: i32,
    pub r: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lane {
    pub z: i32,
    pub x0: i32,
    pub x1: i32,
    pub speed: i32,
    pub count: i32,
    pub colours: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoleDef {
    pub id: String,
    pub name: String,
    pub theme: String,
    pub par: i32,
    pub bounds: [i32; 4],
    pub base: i32,
    pub rough: (i32, i32),
    pub waters: Vec<Water>,
    pub falls: Vec<Fall>,
    pub tunnels: Vec<Tunnel>,
    /// giant blue mushrooms: x, z, cap radius, cap height
    pub bumpers: Vec<[i32; 4]>,
    /// point bubbles: x, z, height above the ground (or water), points
    pub bubbles: Vec<[i32; 4]>,
    pub snails: Vec<Snail>,
    /// antlion pits: x, z, radius
    pub antlions: Vec<[i32; 3]>,
    /// whirlpools: x, z, radius
    pub whirlpools: Vec<[i32; 3]>,
    pub tee: (i32, i32),
    pub lotus: (i32, i32),
    pub shapes: Vec<Shape>,
    pub fairways: Vec<(i32, i32, i32, i32, i32)>,
    pub paints: Vec<Paint>,
    pub pads: Vec<(i32, i32, i32)>,
    pub webs: Vec<Web>,
    pub mushrooms: Vec<(i32, i32)>,
    pub items: Vec<(ItemKind, i32, i32)>,
    /// lines of flies: from, to, how many, how high above the ground, and
    /// how much higher the middle of the line rises (an arc a shot flies)
    pub flies: Vec<(Xz, Xz, i32, i32, i32)>,
    /// flies along the path of a shot: from, toward, at a power (percent), how many
    pub shot_flies: Vec<(Xz, Xz, i32, i32)>,
    pub props: Vec<Prop>,
    pub scatter: Vec<Scatter>,
    /// the city's blocks: x0, z0, x1, z1, height
    pub buildings: Vec<[i32; 5]>,
    pub lanes: Vec<Lane>,
}

#[derive(Deserialize)]
struct CourseFile {
    holes: Vec<HoleFile>,
}

#[derive(Deserialize)]
struct PropFile {
    kind: String,
    at: [i32; 2],
    #[serde(default = "hundred")]
    size: i32,
    #[serde(default)]
    yaw: i32,
}

fn hundred() -> i32 {
    100
}

#[derive(Deserialize)]
struct ScatterFile {
    kind: String,
    count: i32,
    seed: i32,
    #[serde(default)]
    region: Option<[i32; 4]>,
}

#[derive(Deserialize)]
struct PaintFile {
    colour: String,
    line: [i32; 5],
}

#[derive(Deserialize)]
struct LaneFile {
    z: i32,
    x: [i32; 2],
    speed: i32,
    count: i32,
    colours: Vec<String>,
}

#[derive(Deserialize)]
struct WaterFile {
    #[serde(default = "water_kind")]
    kind: String,
    #[serde(default)]
    line: Option<[i32; 5]>,
    #[serde(default)]
    disc: Option<[i32; 3]>,
    level: i32,
}

fn water_kind() -> String {
    "water".into()
}

#[derive(Deserialize)]
struct HoleFile {
    id: String,
    name: String,
    theme: String,
    par: i32,
    bounds: [i32; 4],
    base: i32,
    #[serde(default)]
    rough: [i32; 2],
    #[serde(default)]
    hazard: Option<(String, i32)>,
    #[serde(default)]
    waters: Vec<WaterFile>,
    #[serde(default)]
    falls: Vec<[i32; 6]>,
    #[serde(default)]
    mesas: Vec<[i32; 5]>,
    #[serde(default)]
    benches: Vec<[i32; 7]>,
    #[serde(default)]
    tunnels: Vec<[i32; 6]>,
    #[serde(default)]
    bumpers: Vec<[i32; 4]>,
    #[serde(default)]
    bubbles: Vec<[i32; 4]>,
    #[serde(default)]
    snails: Vec<[i32; 6]>,
    #[serde(default)]
    antlions: Vec<[i32; 3]>,
    #[serde(default)]
    whirlpools: Vec<[i32; 3]>,
    tee: [i32; 2],
    lotus: [i32; 2],
    #[serde(default)]
    hills: Vec<[i32; 4]>,
    #[serde(default)]
    plateaus: Vec<[i32; 4]>,
    #[serde(default)]
    ridges: Vec<[i32; 6]>,
    #[serde(default)]
    fairways: Vec<[i32; 5]>,
    #[serde(default)]
    paints: Vec<PaintFile>,
    #[serde(default)]
    pads: Vec<[i32; 3]>,
    #[serde(default)]
    webs: Vec<[i32; 3]>,
    #[serde(default)]
    mushrooms: Vec<[i32; 2]>,
    #[serde(default)]
    items: Vec<(String, i32, i32)>,
    #[serde(default)]
    flies: Vec<Vec<i32>>,
    #[serde(default)]
    shot_flies: Vec<[i32; 6]>,
    #[serde(default)]
    props: Vec<PropFile>,
    #[serde(default)]
    scatter: Vec<ScatterFile>,
    #[serde(default)]
    buildings: Vec<[i32; 5]>,
    #[serde(default)]
    lanes: Vec<LaneFile>,
}

pub const PROP_KINDS: &[&str] = &[
    "tree",
    "cypress",
    "reeds",
    "log",
    "rock",
    "cactus",
    "palm",
    "lamp",
    "basalt",
    "boulder",
    "bench",
    "hydrant",
    "planter",
    "vent",
    "dune_grass",
    "skull",
    "volcano",
    "tower",
    "fountain",
    "toadstool",
    "glowcap",
    "puffball",
    "shelf_tree",
    "fungus_tree",
];

pub fn parse_course(text: &str, palette: &Palette) -> Result<Vec<HoleDef>, String> {
    let f: CourseFile = serde_json::from_str(text).map_err(|e| format!("course: {e}"))?;
    let p = |a: i32, b: i32| (cm(a), cm(b));
    let mut out = Vec::new();
    for h in f.holes {
        let err = |m: &str| format!("hole {}: {m}", h.id);
        if !palette.themes.contains_key(&h.theme) {
            return Err(err(&format!("no theme {}", h.theme)));
        }
        let [x0, z0, x1, z1] = h.bounds;
        if x1 <= x0 || z1 <= z0 {
            return Err(err("bounds run from the low corner to the high one"));
        }
        let inside = |x: i32, z: i32| x > x0 && x < x1 && z > z0 && z < z1;
        if !inside(h.tee[0], h.tee[1]) || !inside(h.lotus[0], h.lotus[1]) {
            return Err(err("the tee and the lotus must be inside the bounds"));
        }
        let kind_of = |k: &str| match k {
            "water" => Ok(HazardKind::Water),
            "lava" => Ok(HazardKind::Lava),
            _ => Err(err(&format!("unknown hazard {k}"))),
        };
        let mut waters = Vec::new();
        if let Some((k, level)) = &h.hazard {
            waters.push(Water {
                kind: kind_of(k)?,
                region: Region::All,
                level: cm(*level),
            });
        }
        for w in &h.waters {
            let region = match (w.line, w.disc) {
                (Some([ax, az, bx, bz, ww]), None) => Region::Line {
                    a: p(ax, az),
                    b: p(bx, bz),
                    w: cm(ww),
                },
                (None, Some([x, z, r])) => Region::Disc {
                    x: cm(x),
                    z: cm(z),
                    r: cm(r),
                },
                _ => return Err(err("a water is a line or a disc")),
            };
            waters.push(Water {
                kind: kind_of(&w.kind)?,
                region,
                level: cm(w.level),
            });
        }
        let mut shapes = Vec::new();
        for [x, z, r, hh] in &h.hills {
            shapes.push(Shape::Hill {
                x: cm(*x),
                z: cm(*z),
                r: cm(*r),
                h: cm(*hh),
            });
        }
        for [x, z, r, hh] in &h.plateaus {
            shapes.push(Shape::Plateau {
                x: cm(*x),
                z: cm(*z),
                r: cm(*r),
                h: cm(*hh),
            });
        }
        for [ax, az, bx, bz, w, hh] in &h.ridges {
            shapes.push(Shape::Ridge {
                a: p(*ax, *az),
                b: p(*bx, *bz),
                w: cm(*w),
                h: cm(*hh),
            });
        }
        for [x, z, r, hh, e] in &h.mesas {
            shapes.push(Shape::Mesa {
                x: cm(*x),
                z: cm(*z),
                r: cm(*r),
                h: cm(*hh),
                edge: cm(*e),
            });
        }
        for [ax, az, bx, bz, w, hh, e] in &h.benches {
            shapes.push(Shape::Bench {
                a: p(*ax, *az),
                b: p(*bx, *bz),
                w: cm(*w),
                h: cm(*hh),
                edge: cm(*e),
            });
        }
        let theme = &palette.themes[&h.theme];
        let mut paints = Vec::new();
        for pf in &h.paints {
            if !theme.paints.contains_key(&pf.colour) {
                return Err(err(&format!("theme {} has no paint {}", h.theme, pf.colour)));
            }
            let [ax, az, bx, bz, w] = pf.line;
            paints.push(Paint {
                colour: pf.colour.clone(),
                a: p(ax, az),
                b: p(bx, bz),
                w: cm(w),
            });
        }
        let mut items = Vec::new();
        for (k, x, z) in &h.items {
            let kind = match k.as_str() {
                "jump" => ItemKind::Jump,
                "wings" => ItemKind::Wings,
                _ => return Err(err(&format!("unknown item {k}"))),
            };
            items.push((kind, cm(*x), cm(*z)));
        }
        let mut props = Vec::new();
        for pf in &h.props {
            if !PROP_KINDS.contains(&pf.kind.as_str()) {
                return Err(err(&format!("unknown prop {}", pf.kind)));
            }
            props.push(Prop {
                kind: pf.kind.clone(),
                x: cm(pf.at[0]),
                z: cm(pf.at[1]),
                size: pf.size,
                yaw: pf.yaw,
            });
        }
        let mut scatter = Vec::new();
        for s in &h.scatter {
            if !PROP_KINDS.contains(&s.kind.as_str()) {
                return Err(err(&format!("unknown scatter {}", s.kind)));
            }
            scatter.push(Scatter {
                kind: s.kind.clone(),
                count: s.count,
                seed: s.seed,
                region: s.region.map(|r| r.map(cm)),
            });
        }
        let mut lanes = Vec::new();
        for l in &h.lanes {
            for c in &l.colours {
                if !palette.has(c) {
                    return Err(err(&format!("no colour {c} for a car")));
                }
            }
            if l.colours.is_empty() || l.count < 1 {
                return Err(err("a lane needs cars and their colours"));
            }
            lanes.push(Lane {
                z: cm(l.z),
                x0: cm(l.x[0]),
                x1: cm(l.x[1]),
                speed: mps(l.speed),
                count: l.count,
                colours: l.colours.clone(),
            });
        }
        out.push(HoleDef {
            name: h.name.clone(),
            theme: h.theme.clone(),
            par: h.par,
            bounds: h.bounds.map(cm),
            base: cm(h.base),
            rough: (cm(h.rough[0]), cm(h.rough[1].max(100))),
            waters,
            falls: h
                .falls
                .iter()
                .map(|[x, z, t, b, w, y]| Fall {
                    x: cm(*x),
                    z: cm(*z),
                    top: cm(*t),
                    bottom: cm(*b),
                    w: cm(*w),
                    yaw: *y,
                })
                .collect(),
            tunnels: h
                .tunnels
                .iter()
                .map(|[x, z0, z1, w, c, t]| Tunnel {
                    x: cm(*x),
                    z0: cm(*z0.min(z1)),
                    z1: cm(*z0.max(z1)),
                    w: cm(*w),
                    clearance: cm(*c),
                    thickness: cm(*t),
                })
                .collect(),
            bumpers: h.bumpers.iter().map(|b| b.map(cm)).collect(),
            bubbles: h
                .bubbles
                .iter()
                .map(|[x, z, hh, pts]| [cm(*x), cm(*z), cm(*hh), *pts])
                .collect(),
            snails: h
                .snails
                .iter()
                .map(|[ax, az, bx, bz, sp, sz]| Snail {
                    a: p(*ax, *az),
                    b: p(*bx, *bz),
                    speed: mps(*sp),
                    size: cm(*sz),
                })
                .collect(),
            antlions: h.antlions.iter().map(|a| a.map(cm)).collect(),
            whirlpools: h.whirlpools.iter().map(|a| a.map(cm)).collect(),
            tee: p(h.tee[0], h.tee[1]),
            lotus: p(h.lotus[0], h.lotus[1]),
            shapes,
            fairways: h
                .fairways
                .iter()
                .map(|[a, b, c, d, w]| (cm(*a), cm(*b), cm(*c), cm(*d), cm(*w)))
                .collect(),
            paints,
            pads: h.pads.iter().map(|[x, z, r]| (cm(*x), cm(*z), cm(*r))).collect(),
            webs: h
                .webs
                .iter()
                .map(|[x, z, r]| Web {
                    x: cm(*x),
                    z: cm(*z),
                    r: cm(*r),
                })
                .collect(),
            mushrooms: h.mushrooms.iter().map(|[x, z]| p(*x, *z)).collect(),
            items,
            flies: h
                .flies
                .iter()
                .map(|f| match f.as_slice() {
                    [ax, az, bx, bz, n, hh] => Ok((p(*ax, *az), p(*bx, *bz), *n, cm(*hh), 0)),
                    [ax, az, bx, bz, n, hh, arc] => Ok((p(*ax, *az), p(*bx, *bz), *n, cm(*hh), cm(*arc))),
                    _ => Err(format!("hole {}: a fly line has six or seven numbers", h.id)),
                })
                .collect::<Result<Vec<_>, _>>()?,
            shot_flies: h
                .shot_flies
                .iter()
                .map(|[ax, az, bx, bz, power, n]| (p(*ax, *az), p(*bx, *bz), *power, *n))
                .collect(),
            props,
            scatter,
            buildings: h.buildings.clone(),
            lanes,
            id: h.id,
        });
    }
    if out.is_empty() {
        return Err("course: no holes".into());
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub struct Files {
        pub frogs: &'static str,
        pub course: &'static str,
        pub palette: &'static str,
    }

    pub fn files() -> Files {
        Files {
            frogs: include_str!("../../../data/frogs.json"),
            course: include_str!("../../../data/course.json"),
            palette: include_str!("../../../data/palette.json"),
        }
    }

    #[test]
    fn the_shipped_data_parses() {
        let f = files();
        let frogs = parse_frogs(f.frogs).unwrap();
        assert_eq!(frogs.len(), 8);
        let pal = Palette::parse(f.palette).unwrap();
        let holes = parse_course(f.course, &pal).unwrap();
        assert_eq!(holes.len(), 5);
        assert_eq!(
            holes.iter().map(|h| h.theme.as_str()).collect::<Vec<_>>(),
            ["swamp", "desert", "fungus", "city", "volcano"]
        );
        assert!(holes.iter().all(|h| h.par >= 5), "every hole is at least a par 5");
    }

    #[test]
    fn a_bad_colour_and_an_unknown_pattern_are_errors() {
        assert!(parse_hex("#12345").is_err());
        assert_eq!(parse_hex("#ffffff").unwrap(), [ONE; 3]);
        let bad = include_str!("../../../data/frogs.json").replace("\"flanks\"", "\"zigzag\"");
        assert!(parse_frogs(&bad).unwrap_err().contains("zigzag"));
    }

    #[test]
    fn a_tee_outside_the_bounds_is_an_error() {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(f.course).unwrap();
        v["holes"][0]["tee"] = serde_json::json!([0, 900000]);
        let bad = v.to_string();
        assert!(parse_course(&bad, &pal).unwrap_err().contains("tee"));
    }
}
