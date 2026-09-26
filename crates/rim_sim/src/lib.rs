//! rim_sim — the headless, deterministic simulation engine.
//!
//! The engine provides *mechanisms* (harvestable, buildable, edible, needs,
//! incidents, events). All *content* comes from mods, including `core`.

pub mod ai;
pub mod command;
pub mod data;
pub mod defs;
pub mod field;
pub mod look;
pub mod map;
pub mod mapgen;
pub mod modloader;
pub mod modtest;
pub mod order;
pub mod path;
pub mod profile;
pub mod rng;
pub mod rules;
pub mod savefile;
pub mod savetext;
pub mod script;
pub mod shelter;
pub mod sim;
pub mod snapshot;
pub mod systems;
pub mod terms;
pub mod world;
pub mod zone;

pub use command::Command;
pub use hecs;
pub use sim::Sim;

/// Simulation ticks per in-game day. At 1x speed the sim runs 60 ticks/sec.
pub const TICKS_PER_DAY: u64 = 20_000;

/// Plugin API version. Mods declare `api = "MAJOR.MINOR"` in `mod.toml`.
pub const API_VERSION: (u32, u32) = (0, 6);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct IVec {
    pub x: i32,
    pub y: i32,
}

impl IVec {
    pub const fn new(x: i32, y: i32) -> Self {
        IVec { x, y }
    }
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        IVec::new(self.x + dx, self.y + dy)
    }
    /// King-move distance: 1 means touching (including diagonals).
    pub fn chebyshev(self, o: IVec) -> i32 {
        (self.x - o.x).abs().max((self.y - o.y).abs())
    }
    /// Octile distance in path-cost units (10 straight, 14 diagonal).
    pub fn octile(self, o: IVec) -> u32 {
        let dx = (self.x - o.x).unsigned_abs();
        let dy = (self.y - o.y).unsigned_abs();
        let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
        14 * lo + 10 * (hi - lo)
    }
}
