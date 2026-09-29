//! Deterministic randomness (DESIGN.md §7b). The world owns one `Rng` per
//! purpose, each derived from the world seed, so a new draw in one system
//! never moves another's rolls; each mod draws from a stream of its own.

use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: mix(seed ^ 0xD1B5_4A32_D192_ED03) }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        // Worked in i64: a span past i32::MAX overflowed. The whole of
        // i32 is 2^32 values, one past u32, and takes all 32 high bits.
        let off = match u32::try_from(hi as i64 - lo as i64 + 1) {
            Ok(n) => self.below(n) as i64,
            Err(_) => (self.next_u64() >> 32) as i64,
        };
        (lo as i64 + off) as i32
    }

    /// Uniform in `[0, 1)`.
    pub fn float(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.float() < p
    }

    /// Round a fractional amount stochastically (keeps rates exact on average).
    pub fn round(&mut self, v: f64) -> i32 {
        let f = v.floor();
        f as i32 + self.chance(v - f) as i32
    }

    /// Resume from a saved `state()`.
    pub fn from_state(state: u64) -> Self {
        Rng { state }
    }

    pub fn state(&self) -> u64 {
        self.state
    }
}

/// Mapgen's spawns, plant spread and new pawns.
pub const SPAWNS: &str = "spawns";
/// Systems' stochastic rounding: needs and health.
pub const SIM: &str = "sim";
/// Pawns' choices: wandering, fleeing, idling, melee damage.
pub const AI: &str = "ai";

/// The stream behind a mod's `rim.random`.
pub fn mod_stream(id: &str) -> String {
    format!("mod:{id}")
}

/// The world's random streams. A stream is opened, from the world seed and
/// its name, the first time something draws from it, and its state is saved
/// with the snapshot. Kept in a sorted map, so saving walks it in one order.
#[derive(Clone, Debug)]
pub struct Streams {
    seed: u64,
    open: BTreeMap<String, Rng>,
}

impl Streams {
    pub fn new(seed: u64) -> Self {
        Streams { seed, open: BTreeMap::new() }
    }

    /// The stream called `name`, opened on first use.
    pub fn stream(&mut self, name: &str) -> &mut Rng {
        if !self.open.contains_key(name) {
            let first = Rng::new(mix(self.seed ^ hash_str(name)));
            self.open.insert(name.to_string(), first);
        }
        self.open.get_mut(name).expect("opened above")
    }

    /// A draw that depends on who draws and when, not on the order anything
    /// draws in: a hash of the stream, an entity, the tick and a draw index.
    /// For a system that iterates entities (DESIGN.md §7b).
    pub fn draw(&self, name: &str, entity: u64, tick: u64, k: u64) -> Rng {
        let who = mix(entity ^ mix(tick ^ mix(k.wrapping_add(0x9E37_79B9_7F4A_7C15))));
        Rng::from_state(mix(self.seed ^ hash_str(name)) ^ who)
    }

    /// Each open stream's state, for a save.
    pub fn states(&self) -> BTreeMap<String, u64> {
        self.open.iter().map(|(n, r)| (n.clone(), r.state())).collect()
    }

    /// Every open stream's state, folded into one number, for a state hash.
    pub fn fingerprint(&self) -> u64 {
        self.open.iter().fold(0, |h, (n, r)| mix(h ^ hash_str(n) ^ r.state()))
    }

    /// Open `name` at `state`, as a save from before streams does: its one
    /// RNG's state, mixed per stream, rather than the game's opening rolls.
    pub fn open_at(&mut self, name: &str, state: u64) {
        self.open.insert(name.to_string(), Rng::from_state(mix(state ^ hash_str(name))));
    }

    /// Streams as a save left them. A stream the save never opened opens
    /// from the seed on first use, as it would have.
    pub fn restore(seed: u64, states: &BTreeMap<String, u64>) -> Self {
        let open = states.iter().map(|(n, &s)| (n.clone(), Rng::from_state(s))).collect();
        Streams { seed, open }
    }
}

/// SplitMix64 finalizer.
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A stable hash of a name (FNV-1a, 64-bit): the same on every machine and
/// in every build, so it can key anything a save or a seed depends on.
pub fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3))
}

/// Stateless hash of a coordinate — for map generation and cosmetic noise.
pub fn hash2(x: i64, y: i64, seed: u64) -> u64 {
    mix(seed ^ mix((x as u64).wrapping_mul(0x9E37_79B9) ^ mix(y as u64 ^ 0x5bd1_e995)))
}

pub fn hash2_f(x: i64, y: i64, seed: u64) -> f64 {
    (hash2(x, y, seed) >> 11) as f64 / (1u64 << 53) as f64
}
