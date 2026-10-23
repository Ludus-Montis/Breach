//! The ground's surface, per Decision 56 and specs/23-ground-model.md (round 2;
//! GroundSurface, GroundRelief and GroundChannels until spec 39 step 3a). Each tile has an
//! elevation in cells at its centre; a cell column's surface is interpolated between tile
//! centres (level beyond the outermost ones), so ground slopes smoothly across tile edges.
//! - The **surface cell** is the highest cell holding any ground. Its shape is its four
//!   corner heights in quarters of a cell (NW, NE, SE, SW): 4 is full, 0 is empty.
//! - Units walk on the surface; a step of more than one cell between neighbouring columns
//!   is a **cliff**, which needs climbing (Decision 54).
//! - Digging the surface cell removes only what is left of it (dig_fraction).
//! - Ground at or above the map's ceiling is **capped**: impassable and unsimulated.
//! - On the tile shape go each terrain's seeded **relief** (`noise`) and the map's carved
//!   **channels**, which may hold a liquid (Decision 59).
//!
//! Cells are global: column x runs across the map, CELLS_PER_TILE per tile. The maths is
//! the GDScript's: points in f32 (`Vector2`), heights in f64, the relief's hash in i64.

use crate::content::{Channel, MapLayout, TerrainLibrary, CELLS_PER_TILE};
use crate::maths::V2;

/// A cell column, [x, y].
pub type Column = [i64; 2];

const TILE: f64 = CELLS_PER_TILE as f64;

pub struct Surface {
    ceiling: i64,
    cols: i64,
    rows: i64,
    seed: i64,
    /// Per tile, row-major.
    elevations: Vec<i64>,
    /// Per tile: (amplitude, scale).
    reliefs: Vec<(i64, i64)>,
    /// Each channel and the centre points of its tiles, in cells.
    channels: Vec<(Channel, Vec<V2>)>,
}

impl Surface {
    pub fn new(layout: &MapLayout, library: &TerrainLibrary) -> Surface {
        let (cols, rows) = (layout.cols.max(1), layout.rows.max(1));
        let half = V2::new(1.0, 1.0).scale(TILE / 2.0);
        let channels = layout
            .channels
            .iter()
            .map(|channel| {
                let points = channel.tiles.iter().map(|t| tile_point(*t).add(half)).collect();
                (channel.clone(), points)
            })
            .collect();
        let mut elevations = Vec::new();
        let mut reliefs = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let tile = layout.tiles.iter().find(|tile| tile.cell == [col, row]);
                let id = match tile {
                    Some(tile) if !tile.terrain_id.is_empty() => &tile.terrain_id,
                    _ => &layout.default_terrain_id,
                };
                let terrain = library.terrains.iter().find(|terrain| &terrain.id == id);
                elevations.push(match (tile, terrain) {
                    (Some(tile), _) if tile.elevation != -1 => tile.elevation,
                    (_, Some(terrain)) => terrain.default_elevation,
                    _ => 0,
                });
                reliefs.push(terrain.map_or((0, 0), |t| (t.relief_amplitude, t.relief_scale)));
            }
        }
        Surface {
            ceiling: layout.ceiling,
            cols,
            rows,
            seed: layout.seed,
            elevations,
            reliefs,
            channels,
        }
    }

    /// The level (cells up from 0) of the column's surface cell.
    pub fn surface_level(&self, column: Column) -> i64 {
        let top = self.corner_quarters(column).into_iter().max().unwrap_or(0).max(0);
        (top as f64 / 4.0).ceil() as i64 - 1
    }

    /// The surface cell's corner heights in quarters (NW, NE, SE, SW), 0 to 4.
    pub fn surface_shape(&self, column: Column) -> [i64; 4] {
        let floor = self.surface_level(column) * 4;
        self.corner_quarters(column).map(|quarters| (quarters - floor).clamp(0, 4))
    }

    /// The column's mean ground height, in cells.
    pub fn surface_height(&self, column: Column) -> f64 {
        self.corner_quarters(column).iter().sum::<i64>() as f64 / 16.0
    }

    /// The part of a cell left to dig in the column's surface cell: 1 when it is full.
    pub fn dig_fraction(&self, column: Column) -> f64 {
        self.surface_shape(column).iter().sum::<i64>() as f64 / 16.0
    }

    /// True if the step between two neighbouring columns is more than one cell.
    pub fn is_cliff(&self, from: Column, to: Column) -> bool {
        (self.surface_height(to) - self.surface_height(from)).abs() > 1.0
    }

    /// The material and level of the liquid a channel holds over the column, if its level
    /// is above the ground there.
    pub fn liquid_at(&self, column: Column) -> Option<(String, f64)> {
        let centre = V2::new(column[0] as f32, column[1] as f32).add(V2::new(0.5, 0.5));
        let (material, level) = self.liquid(centre, self.ground_at(centre))?;
        (level > self.surface_height(column)).then_some((material, level))
    }

    /// True if the column's ground reaches the ceiling.
    pub fn is_capped(&self, column: Column) -> bool {
        self.surface_height(column) >= self.ceiling as f64
    }

    /// The column's four corner heights, rounded to quarters of a cell.
    fn corner_quarters(&self, [x, y]: Column) -> [i64; 4] {
        [(0, 0), (1, 0), (1, 1), (0, 1)]
            .map(|(dx, dy)| (self.height_at([x + dx, y + dy]) * 4.0).round() as i64)
    }

    /// Ground height at a cell corner: the tile shape and relief, less any channel cut.
    fn height_at(&self, corner: Column) -> f64 {
        let point = V2::new(corner[0] as f32, corner[1] as f32);
        self.ground_at(point) - self.carve(point)
    }

    /// The uncarved ground at a point: tile elevations and each tile's relief, blended
    /// between tile centres (level beyond the outermost ones).
    fn ground_at(&self, point: V2) -> f64 {
        let u = (point.x as f64 / TILE - 0.5).clamp(0.0, self.cols as f64 - 1.0);
        let v = (point.y as f64 / TILE - 0.5).clamp(0.0, self.rows as f64 - 1.0);
        let col = if self.cols > 1 { (u.floor() as i64).min(self.cols - 2) } else { 0 };
        let row = if self.rows > 1 { (v.floor() as i64).min(self.rows - 2) } else { 0 };
        let (du, dv) = (u - col as f64, v - row as f64);
        let weights = [
            ((col, row), (1.0 - du) * (1.0 - dv)),
            ((col + 1, row), du * (1.0 - dv)),
            ((col, row + 1), (1.0 - du) * dv),
            ((col + 1, row + 1), du * dv),
        ];
        let mut height = 0.0;
        for (tile, weight) in weights {
            let index = self.index(tile);
            let (amplitude, scale) = self.reliefs[index];
            let bump = amplitude as f64 * noise(point, scale, self.seed);
            height += weight * (self.elevations[index] as f64 + bump);
        }
        height
    }

    /// Cells to take off the ground at a point: the deepest cut (or highest bank) there.
    fn carve(&self, point: V2) -> f64 {
        let mut cut: f64 = 0.0;
        for line in &self.channels {
            let depth = line.0.depth as f64 * share(line, point);
            if depth.abs() > cut.abs() {
                cut = depth;
            }
        }
        cut
    }

    /// The material and level of a channel's liquid at a point, given the uncarved ground
    /// height there; none where no channel holds liquid.
    fn liquid(&self, point: V2, ground: f64) -> Option<(String, f64)> {
        self.channels.iter().find_map(|line| {
            let channel = &line.0;
            let holds = !channel.liquid_id.is_empty() && channel.liquid_depth > 0;
            (holds && share(line, point) > 0.0).then(|| {
                let level = ground - channel.depth as f64 + channel.liquid_depth as f64;
                (channel.liquid_id.clone(), level)
            })
        })
    }

    fn index(&self, (col, row): (i64, i64)) -> usize {
        (row.clamp(0, self.rows - 1) * self.cols + col.clamp(0, self.cols - 1)) as usize
    }
}

/// `Vector2(tile) * TILE_CELLS`.
fn tile_point([col, row]: [i64; 2]) -> V2 {
    V2::new(col as f32, row as f32).scale(TILE)
}

/// How much of the channel's depth applies at a point: 1 inside, falling to 0 across the
/// one-cell bank at its edge.
fn share(line: &(Channel, Vec<V2>), point: V2) -> f64 {
    let half = line.0.width as f64 / 2.0;
    (half - distance(&line.1, point)).clamp(0.0, 1.0)
}

fn distance(points: &[V2], point: V2) -> f64 {
    if points.len() == 1 {
        return point.distance_to(points[0]) as f64;
    }
    let mut nearest = f64::INFINITY;
    for pair in points.windows(2) {
        let on_line = closest_on_segment(point, pair[0], pair[1]);
        nearest = nearest.min(point.distance_to(on_line) as f64);
    }
    nearest
}

/// `Geometry2D.get_closest_point_to_segment`, in real_t.
fn closest_on_segment(point: V2, a: V2, b: V2) -> V2 {
    let p = point.sub(a);
    let n = b.sub(a);
    let l2 = n.x * n.x + n.y * n.y;
    if l2 < 1e-20 {
        return a;
    }
    let d = n.dot(p) / l2;
    if d <= 0.0 {
        a
    } else if d >= 1.0 {
        b
    } else {
        V2::new(a.x + n.x * d, a.y + n.y * d)
    }
}

/// Seeded relief noise (Decision 59): smooth value noise in [-1, 1], varying over about
/// `scale` cells and reproducible from the map's seed; 0 when scale is 0.
pub fn noise(point: V2, scale: i64, seed: i64) -> f64 {
    if scale <= 0 {
        return 0.0;
    }
    let s = scale as f64 as f32;
    let grid = V2::new(point.x / s, point.y / s);
    let cell = ((grid.x as f64).floor() as i64, (grid.y as f64).floor() as i64);
    let fx = smoothstep(grid.x as f64 - cell.0 as f64);
    let fy = smoothstep(grid.y as f64 - cell.1 as f64);
    let value = |dx: i64, dy: i64| lattice_value((cell.0 + dx, cell.1 + dy), scale, seed);
    let top = lerp(value(0, 0), value(1, 0), fx);
    let bottom = lerp(value(0, 1), value(1, 1), fx);
    lerp(top, bottom, fy)
}

/// GDScript's `smoothstep(0.0, 1.0, x)`.
fn smoothstep(x: f64) -> f64 {
    let s = x.clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

/// GDScript's `lerpf`.
fn lerp(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}

/// A lattice point's value in [-1, 1]: an integer hash, the same on every machine.
fn lattice_value((x, y): (i64, i64), scale: i64, seed: i64) -> f64 {
    let n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(982451653))
        .wrapping_add(scale.wrapping_mul(144665));
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    let n = n ^ (n >> 16);
    (n & 0xffff) as f64 / 32767.5 - 1.0
}
