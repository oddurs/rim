//! rim_sim — the headless, deterministic simulation engine.
//!
//! The engine provides *mechanisms* (harvestable, buildable, edible, needs,
//! incidents, events). All *content* comes from mods, including `core`.

pub mod ai;
pub mod command;
pub mod data;
pub mod defs;
pub mod field;
pub mod filter;
pub mod look;
pub mod map;
pub mod mapgen;
pub mod modloader;
pub mod modtest;
pub mod near;
pub mod order;
pub mod path;
pub mod plan;
pub mod profile;
pub mod rng;
pub mod rules;
pub mod savefile;
pub mod savetext;
pub mod script;
pub mod shelter;
pub mod sim;
pub mod snapshot;
pub mod stock;
pub mod store;
pub mod systems;
pub mod terms;
pub mod testseed;
pub mod water;
pub mod world;
pub mod zone;

pub use command::Command;
pub use hecs;
pub use sim::Sim;

/// Simulation ticks per in-game day. At 1x speed the sim runs 60 ticks/sec.
pub const TICKS_PER_DAY: u64 = 20_000;

/// Plugin API version. Mods declare `api = "MAJOR.MINOR"` in `mod.toml`.
pub const API_VERSION: (u32, u32) = (0, 6);

/// A cell: `x` and `y` on a level, and the level `z` (0 is the surface,
/// below is negative; DESIGN.md §6d). A position written without `z` is on
/// the surface, and `z` 0 isn't written, so a save, command or script from
/// before levels reads the same.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct IVec {
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "is_surface")]
    pub z: i32,
}

fn is_surface(z: &i32) -> bool {
    *z == 0
}

/// How far apart two cells on different levels count, for `chebyshev`:
/// they never touch, and nothing on one level is near anything on another.
pub const LEVELS_APART: i32 = 1 << 20;

/// What changing a level adds to `octile`: at most what the cheapest way
/// between levels costs, so A* stays admissible.
pub const OCTILE_PER_LEVEL: u32 = 20;

impl IVec {
    /// A cell on the surface.
    pub const fn new(x: i32, y: i32) -> Self {
        IVec { x, y, z: 0 }
    }
    /// A cell on level `z`.
    pub const fn at(x: i32, y: i32, z: i32) -> Self {
        IVec { x, y, z }
    }
    /// The same level, moved across it.
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        IVec::at(self.x + dx, self.y + dy, self.z)
    }
    /// King-move distance on one level: 1 means touching (including
    /// diagonals). Cells on different levels are `LEVELS_APART` or more.
    pub fn chebyshev(self, o: IVec) -> i32 {
        let flat = (self.x - o.x).abs().max((self.y - o.y).abs());
        if self.z == o.z {
            flat
        } else {
            flat.saturating_add(LEVELS_APART)
        }
    }
    /// Octile distance in path-cost units (10 straight, 14 diagonal), plus
    /// `OCTILE_PER_LEVEL` for each level between.
    pub fn octile(self, o: IVec) -> u32 {
        let dx = (self.x - o.x).unsigned_abs();
        let dy = (self.y - o.y).unsigned_abs();
        let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
        14 * lo + 10 * (hi - lo) + OCTILE_PER_LEVEL * (self.z - o.z).unsigned_abs()
    }
}
