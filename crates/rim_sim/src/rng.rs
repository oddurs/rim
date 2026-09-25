//! Deterministic RNG. The world owns exactly one; scripts draw from it too.

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
            lo
        } else {
            lo + self.below((hi - lo + 1) as u32) as i32
        }
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

/// SplitMix64 finalizer.
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stateless hash of a coordinate — for map generation and cosmetic noise.
pub fn hash2(x: i64, y: i64, seed: u64) -> u64 {
    mix(seed ^ mix((x as u64).wrapping_mul(0x9E37_79B9) ^ mix(y as u64 ^ 0x5bd1_e995)))
}

pub fn hash2_f(x: i64, y: i64, seed: u64) -> f64 {
    (hash2(x, y, seed) >> 11) as f64 / (1u64 << 53) as f64
}
