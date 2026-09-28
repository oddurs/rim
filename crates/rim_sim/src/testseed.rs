//! Seeds for tests that don't care which map they get (DESIGN.md §7b).
//!
//! A test that names no seed gets one from its own name, shifted by
//! `RIM_SEED_SHIFT`: 0 on a PR, the date at night. So a test that secretly
//! depends on its map passes every PR and fails some night, instead of never.
//! `RIM_SEED` pins every such seed, which is how a printed repro line brings a
//! failure back.

use crate::rng::mix;

/// How a test's seed was made, for the line that brings a failure back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// From the test's name, shifted.
    Derived { shift: u64 },
    /// Pinned by `RIM_SEED`.
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seed {
    pub seed: u64,
    pub how: How,
    /// What the seed was derived from.
    pub name: String,
}

impl Seed {
    /// "seed 123 (from "name", shift 0)" or "seed 123 (RIM_SEED)".
    pub fn describe(&self) -> String {
        match self.how {
            How::Derived { shift } => format!("seed {} (from {:?}, shift {shift})", self.seed, self.name),
            How::Pinned => format!("seed {} (RIM_SEED)", self.seed),
        }
    }
}

/// A name's seed: stable across platforms, runs and builds, and under 2^53,
/// so a Luau test can read it back with `w:seed()` and pass it on exactly.
pub fn hash_name(name: &str) -> u64 {
    // FNV-1a, then mixed so neighbouring names land far apart.
    let h = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    mix(h) & ((1 << 53) - 1)
}

/// The seed a test named `name` gets, reading the environment through `var`.
pub fn seed_with(name: &str, var: impl Fn(&str) -> Option<String>) -> Seed {
    let num = |key: &str| var(key).and_then(|v| v.trim().parse::<u64>().ok());
    match num("RIM_SEED") {
        Some(seed) => Seed { seed, how: How::Pinned, name: name.to_string() },
        None => {
            let shift = num("RIM_SEED_SHIFT").unwrap_or(0);
            Seed { seed: hash_name(name) ^ shift, how: How::Derived { shift }, name: name.to_string() }
        }
    }
}

/// The seed a test named `name` gets.
pub fn seed(name: &str) -> Seed {
    seed_with(name, |k| std::env::var(k).ok())
}

/// A Rust test's seed, from the test's own name. If the test panics, dropping
/// this prints the seed and the command that runs the one test again with it.
///
/// ```ignore
/// let seed = TestSeed::new();
/// let mut sim = Sim::new(&mods, seed.seed).unwrap();
/// ```
pub struct TestSeed {
    pub seed: u64,
    repro: String,
}

impl TestSeed {
    #[track_caller]
    pub fn new() -> TestSeed {
        // libtest names the test's thread after the test, and nextest runs
        // each test in its own process.
        let name = std::thread::current().name().unwrap_or("unnamed").to_string();
        let s = seed(&name);
        let rerun = rerun_command(std::panic::Location::caller().file(), &name, s.seed);
        TestSeed { seed: s.seed, repro: format!("{}; rerun: {rerun}", s.describe()) }
    }

    /// The line printed on a failure.
    pub fn repro(&self) -> &str {
        &self.repro
    }
}

impl Default for TestSeed {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestSeed {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("{}", self.repro);
        }
    }
}

/// The nextest command that runs one test with its seed pinned, from the
/// test's source file (`crates/<crate>/tests/<binary>.rs` or a `src` file).
pub fn rerun_command(file: &str, test: &str, seed: u64) -> String {
    let file = file.replace('\\', "/");
    let krate = file.split('/').skip_while(|p| *p != "crates").nth(1).unwrap_or("rim_sim");
    let binary = match file.split_once("/tests/") {
        Some((_, rest)) => format!(" --test {}", rest.split('/').next().unwrap_or(rest).trim_end_matches(".rs")),
        None => String::new(),
    };
    format!("RIM_SEED={seed} cargo nextest run --release -p {krate}{binary} -E 'test(={test})'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn a_name_always_gets_the_same_seed_and_names_get_different_ones() {
        assert_eq!(hash_name("core/tests/a.luau/one"), hash_name("core/tests/a.luau/one"));
        assert_ne!(hash_name("core/tests/a.luau/one"), hash_name("core/tests/a.luau/two"));
        // Pinned, so a change to the hash is a deliberate one: it moves every
        // derived seed.
        assert_eq!(hash_name(""), mix(0xcbf2_9ce4_8422_2325) & ((1 << 53) - 1));
        assert!(hash_name("core/tests/a.luau/one") < 1 << 53);
    }

    #[test]
    fn the_shift_moves_the_seed_and_rim_seed_pins_it() {
        let plain = seed_with("t", env(&[]));
        assert_eq!(plain.how, How::Derived { shift: 0 });
        assert_eq!(plain.seed, hash_name("t"));
        let night = seed_with("t", env(&[("RIM_SEED_SHIFT", "20260928")]));
        assert_eq!(night.seed, hash_name("t") ^ 20260928);
        let pinned = seed_with("t", env(&[("RIM_SEED", "42"), ("RIM_SEED_SHIFT", "20260928")]));
        assert_eq!((pinned.seed, pinned.how), (42, How::Pinned));
        assert_eq!(pinned.describe(), "seed 42 (RIM_SEED)");
    }

    #[test]
    fn the_rerun_command_names_the_crate_the_binary_and_the_test() {
        assert_eq!(
            rerun_command("crates/rim_sim/tests/weather.rs", "rain_falls", 7),
            "RIM_SEED=7 cargo nextest run --release -p rim_sim --test weather -E 'test(=rain_falls)'"
        );
        assert_eq!(
            rerun_command("crates\\rim_client\\src\\draw.rs", "tests::the_overlay", 7),
            "RIM_SEED=7 cargo nextest run --release -p rim_client -E 'test(=tests::the_overlay)'"
        );
    }
}
