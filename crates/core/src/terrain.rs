//! The hole's ground: a height at every grid point, built from the hole's
//! shapes and a little seeded roughness, read back as a height and a slope
//! anywhere, and drawn as one mesh whose vertices carry the ground's colours.
//!
//! Every cell is split into two triangles along the same diagonal for the
//! height lookup and for the mesh, so a frog rolls on exactly the ground it
//! is drawn on.

use crate::content::{HoleDef, Paint, Rgb, Shape, Theme};
use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::render::mix;

/// Grid spacing, in Q16 metres.
pub const CELL: i32 = 2 * ONE;

#[derive(Debug, Clone)]
pub struct Terrain {
    pub x0: i32,
    pub z0: i32,
    pub nx: usize,
    pub nz: usize,
    heights: Vec<i32>,
}

/// The rounded bump all shapes share: 1 at the middle, 0 at `r` and beyond.
fn bump(d: i32, r: i32) -> i32 {
    if d >= r || r <= 0 {
        return 0;
    }
    let t = fx::div(d, r);
    (ONE + fx::cos(fx::mul(t, fx::HALF))) / 2
}

/// Distance from a point to a segment on the ground.
pub fn seg_dist(px: i32, pz: i32, a: (i32, i32), b: (i32, i32)) -> i32 {
    let (dx, dz) = ((b.0 - a.0) as i64, (b.1 - a.1) as i64);
    let (qx, qz) = ((px - a.0) as i64, (pz - a.1) as i64);
    let len2 = dx * dx + dz * dz;
    let t = if len2 == 0 {
        0
    } else {
        ((qx * dx + qz * dz).clamp(0, len2) << 16) / len2
    };
    let cx = a.0 as i64 + ((dx * t) >> 16);
    let cz = a.1 as i64 + ((dz * t) >> 16);
    let (ex, ez) = (px as i64 - cx, pz as i64 - cz);
    fx::isqrt((ex * ex + ez * ez) as u64) as i32
}

fn hash(x: i32, z: i32, seed: u32) -> i32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343) ^ (z as u32).wrapping_mul(0xD816_3841) ^ seed;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0xFFFF) as i32
}

fn smooth(t: i32) -> i32 {
    // 3t² − 2t³
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

/// Smooth value noise in [0, ONE) at a scale.
pub fn noise(x: i32, z: i32, scale: i32, seed: u32) -> i32 {
    let gx = fx::div(x, scale);
    let gz = fx::div(z, scale);
    let (ix, iz) = (gx.div_euclid(ONE), gz.div_euclid(ONE));
    let (fx_, fz) = (smooth(gx.rem_euclid(ONE)), smooth(gz.rem_euclid(ONE)));
    let a = hash(ix, iz, seed);
    let b = hash(ix + 1, iz, seed);
    let c = hash(ix, iz + 1, seed);
    let d = hash(ix + 1, iz + 1, seed);
    fx::lerp(fx::lerp(a, b, fx_), fx::lerp(c, d, fx_), fz)
}

impl Terrain {
    pub fn build(hole: &HoleDef) -> Terrain {
        let [x0, z0, x1, z1] = hole.bounds;
        let nx = ((x1 - x0) / CELL) as usize + 1;
        let nz = ((z1 - z0) / CELL) as usize + 1;
        let mut heights = Vec::with_capacity(nx * nz);
        for j in 0..nz {
            for i in 0..nx {
                let x = x0 + i as i32 * CELL;
                let z = z0 + j as i32 * CELL;
                heights.push(shaped_height(hole, x, z));
            }
        }
        Terrain {
            x0,
            z0,
            nx,
            nz,
            heights,
        }
    }

    fn at(&self, i: usize, j: usize) -> i32 {
        self.heights[j.min(self.nz - 1) * self.nx + i.min(self.nx - 1)]
    }

    fn point(&self, i: usize, j: usize) -> V3 {
        v3(
            self.x0 + i as i32 * CELL,
            self.at(i, j),
            self.z0 + j as i32 * CELL,
        )
    }

    /// The cell a point is over, clamped to the grid, and where in it.
    fn locate(&self, x: i32, z: i32) -> (usize, usize, i32, i32) {
        let gx = fx::div(x - self.x0, CELL).clamp(0, (self.nx as i32 - 1) * ONE - 1);
        let gz = fx::div(z - self.z0, CELL).clamp(0, (self.nz as i32 - 1) * ONE - 1);
        ((gx / ONE) as usize, (gz / ONE) as usize, gx % ONE, gz % ONE)
    }

    /// The triangle under a point: its three corners.
    fn tri(&self, x: i32, z: i32) -> ([V3; 3], i32, i32) {
        let (i, j, u, w) = self.locate(x, z);
        let p00 = self.point(i, j);
        let p10 = self.point(i + 1, j);
        let p01 = self.point(i, j + 1);
        let p11 = self.point(i + 1, j + 1);
        if u + w <= ONE {
            ([p00, p10, p01], u, w)
        } else {
            ([p11, p01, p10], ONE - u, ONE - w)
        }
    }

    /// The ground's height under a point.
    pub fn height(&self, x: i32, z: i32) -> i32 {
        let ([a, b, c], u, w) = self.tri(x, z);
        a.y + fx::mul(b.y - a.y, u) + fx::mul(c.y - a.y, w)
    }

    /// The ground's upward unit normal under a point.
    pub fn normal(&self, x: i32, z: i32) -> V3 {
        let ([a, b, c], _, _) = self.tri(x, z);
        let n = c.sub(a).cross(b.sub(a)).norm();
        if n.y < 0 {
            n.neg()
        } else {
            n
        }
    }

    pub fn lowest_edge(&self) -> i32 {
        let mut low = i32::MAX;
        for i in 0..self.nx {
            low = low.min(self.at(i, 0)).min(self.at(i, self.nz - 1));
        }
        for j in 0..self.nz {
            low = low.min(self.at(0, j)).min(self.at(self.nx - 1, j));
        }
        low
    }

    /// The mesh: two flat-shaded triangles a cell, each tinted by what
    /// ground it is (water's bed, the shore, rock on a slope, the fairway,
    /// a painted road, or the grass's two shades).
    pub fn mesh(&self, hole: &HoleDef, theme: &Theme) -> Vec<i32> {
        let mut out = Vec::with_capacity((self.nx - 1) * (self.nz - 1) * 6 * crate::mesh::VERTEX_VALUES);
        for j in 0..self.nz - 1 {
            for i in 0..self.nx - 1 {
                let p00 = self.point(i, j);
                let p10 = self.point(i + 1, j);
                let p01 = self.point(i, j + 1);
                let p11 = self.point(i + 1, j + 1);
                // colour by the cell's middle, so stripes and paths have square edges;
                // by the triangle's own height and slope for water, shore and rock
                let centre = v3(p00.x + CELL / 2, 0, p00.z + CELL / 2);
                for [a, b, c] in [[p00, p01, p10], [p11, p10, p01]] {
                    let n = b.sub(a).cross(c.sub(a)).norm();
                    let mid = v3(centre.x, (a.y + b.y + c.y) / 3, centre.z);
                    let tint = ground_colour(hole, theme, mid, n);
                    for p in [a, b, c] {
                        out.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, tint[0], tint[1], tint[2]]);
                    }
                }
            }
        }
        out
    }
}

/// The height the hole's shapes give a point, before the grid.
fn shaped_height(hole: &HoleDef, x: i32, z: i32) -> i32 {
    let mut h = hole.base;
    for s in &hole.shapes {
        h += match *s {
            Shape::Hill {
                x: sx,
                z: sz,
                r,
                h: hh,
            } => fx::mul(hh, bump(dist(x - sx, z - sz), r)),
            Shape::Plateau {
                x: sx,
                z: sz,
                r,
                h: hh,
            } => {
                let d = dist(x - sx, z - sz);
                fx::mul(hh, bump((d - r / 2).max(0), r / 2))
            }
            Shape::Ridge { a, b, w, h: hh } => fx::mul(hh, bump(seg_dist(x, z, a, b), w)),
        };
    }
    let (amp, scale) = hole.rough;
    if amp > 0 {
        let n = noise(x, z, scale, seed_of(&hole.id)) - ONE / 2;
        h += fx::mul(n, amp * 2);
    }
    h
}

fn dist(dx: i32, dz: i32) -> i32 {
    fx::isqrt((dx as i64 * dx as i64 + dz as i64 * dz as i64) as u64) as i32
}

pub fn seed_of(id: &str) -> u32 {
    id.bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

fn painted(paints: &[Paint], x: i32, z: i32) -> Option<&Paint> {
    paints.iter().rev().find(|p| seg_dist(x, z, p.a, p.b) <= p.w / 2)
}

fn ground_colour(hole: &HoleDef, theme: &Theme, p: V3, n: V3) -> Rgb {
    let level = hole.hazard.map(|(_, l)| l);
    if let Some(l) = level {
        if p.y < l {
            return theme.bed;
        }
    }
    if let Some(pt) = painted(&hole.paints, p.x, p.z) {
        return theme.paints[&pt.colour];
    }
    if let Some(l) = level {
        if p.y < l + ONE / 2 {
            return theme.shore;
        }
    }
    if n.y < ONE * 80 / 100 {
        return theme.rock;
    }
    let wobble = noise(p.x, p.z, 7 * ONE, seed_of(&hole.id) ^ 0xA5A5);
    let grass = mix(theme.ground[0], theme.ground[1], wobble);
    for &(ax, az, bx, bz, w) in &hole.fairways {
        let d = seg_dist(p.x, p.z, (ax, az), (bx, bz));
        if d <= w / 2 {
            // mown in stripes across the line, four metres a stripe
            let stripe = ((p.z.div_euclid(4 * ONE)) & 1) == 0;
            return if stripe {
                theme.fairway
            } else {
                mix(theme.fairway, grass, ONE / 5)
            };
        }
    }
    grass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{parse_course, tests::files, Palette};
    use crate::fx::int;

    fn swamp() -> HoleDef {
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        parse_course(f.course, &pal).unwrap().remove(0)
    }

    #[test]
    fn a_hill_is_highest_at_its_middle_and_gone_past_its_radius() {
        assert_eq!(bump(0, int(10)), ONE);
        assert!((bump(int(5), int(10)) - ONE / 2).abs() < 8);
        assert_eq!(bump(int(10), int(10)), 0);
        assert_eq!(bump(int(30), int(10)), 0);
    }

    #[test]
    fn the_height_between_grid_points_is_on_the_drawn_triangle() {
        let t = Terrain::build(&swamp());
        // at a grid point the lookup returns the stored height exactly
        let p = t.point(10, 12);
        assert_eq!(t.height(p.x, p.z), p.y);
        // half way along a cell's bottom edge it is the average of the two ends
        let q = t.point(11, 12);
        let mid = t.height((p.x + q.x) / 2, p.z);
        assert!((mid - (p.y + q.y) / 2).abs() <= 2, "{mid} vs {}", (p.y + q.y) / 2);
    }

    #[test]
    fn the_normal_of_flat_ground_points_up_and_a_slope_leans_downhill() {
        let mut h = swamp();
        h.shapes.clear();
        h.rough = (0, ONE);
        let flat = Terrain::build(&h);
        assert_eq!(flat.normal(int(3), int(-7)), V3::UP);
        h.shapes.push(Shape::Hill {
            x: 0,
            z: int(-50),
            r: int(20),
            h: int(8),
        });
        let hill = Terrain::build(&h);
        // east of the summit the ground falls away east, so the normal leans +x
        let n = hill.normal(int(10), int(-50));
        assert!(n.x > ONE / 10 && n.y > 0, "{n:?}");
    }

    #[test]
    fn segment_distance_measures_to_the_nearest_point() {
        let (a, b) = ((0, 0), (int(10), 0));
        assert_eq!(seg_dist(int(5), int(3), a, b), int(3));
        assert_eq!(seg_dist(int(14), int(3), a, b), int(5));
    }

    #[test]
    fn the_mesh_has_two_triangles_a_cell() {
        let h = swamp();
        let f = files();
        let pal = Palette::parse(f.palette).unwrap();
        let t = Terrain::build(&h);
        let m = t.mesh(&h, &pal.themes["swamp"]);
        assert_eq!(m.len(), (t.nx - 1) * (t.nz - 1) * 6 * crate::mesh::VERTEX_VALUES);
    }
}
