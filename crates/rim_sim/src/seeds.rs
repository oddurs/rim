//! Maps worth keeping, and runs over many seeds (DESIGN.md §7b).
//!
//! - The corpus, `tests/seeds.toml`: seeds with a reason to keep them.
//! - `Where`: a question about a map near its start, for `rim seeds find`.
//! - `soak`: one seed played for some days, then saved and loaded, for the
//!   nightly sweep over 200 seeds made from the date.

use crate::snapshot::Snapshot;
use crate::testseed::hash_name;
use crate::world::World;
use crate::{IVec, Sim, TICKS_PER_DAY};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// A seed kept in the corpus, with why.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusSeed {
    pub id: String,
    pub seed: u64,
    pub why: String,
    /// The cairn item it came from, when a sweep found it.
    #[serde(default)]
    pub item: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusFile {
    #[serde(default)]
    seed: Vec<CorpusSeed>,
}

/// The corpus in `path` (`tests/seeds.toml`).
pub fn corpus(path: &Path) -> Result<Vec<CorpusSeed>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: CorpusFile = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut ids: Vec<&str> = file.seed.iter().map(|s| s.id.as_str()).collect();
    ids.sort();
    if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
        return Err(format!("{}: two seeds are called {}", path.display(), w[0]));
    }
    Ok(file.seed)
}

/// A question about the land near the colony's start: `water near start`,
/// `no granite near start`, `oak within 6 of start`. The feature is a terrain
/// id, a terrain tag or a thing id, bare or qualified.
#[derive(Clone, Debug, PartialEq)]
pub struct Where {
    pub no: bool,
    pub feature: String,
    /// Cells from the start, in either direction.
    pub within: i32,
}

/// How near "near" is.
pub const NEAR: i32 = 12;

impl Where {
    pub fn parse(s: &str) -> Result<Where, String> {
        let bad = || {
            format!("can't read '{s}': write '<feature> near start', 'no <feature> near start' or '<feature> within N of start'")
        };
        let words: Vec<&str> = s.split_whitespace().collect();
        let (no, words) = match words.split_first() {
            Some((&"no", rest)) => (true, rest),
            _ => (false, &words[..]),
        };
        match words {
            [feature, "near", "start"] => Ok(Where { no, feature: feature.to_string(), within: NEAR }),
            [feature, "within", n, "of", "start"] => {
                let within = n.parse().map_err(|_| bad())?;
                Ok(Where { no, feature: feature.to_string(), within })
            }
            _ => Err(bad()),
        }
    }

    /// Whether a cell holds the feature.
    fn at(&self, w: &World, p: IVec) -> bool {
        let same = |id: &str| id == self.feature || id.split_once(':').is_some_and(|(_, bare)| bare == self.feature);
        let t = &w.defs.terrain[w.map.terrain[w.map.idx(p)] as usize];
        if same(&t.id) || t.tags.contains(&self.feature) {
            return true;
        }
        [w.map.fixture_at(p), w.map.item_at(p)]
            .into_iter()
            .flatten()
            .filter_map(|e| w.thing(e))
            .any(|thing| same(&w.defs.thing(thing.def).id))
    }

    /// The nearest cell to `start` with the feature, within reach; for `no`,
    /// `start` itself when none is there.
    pub fn check(&self, w: &World, start: IVec) -> Option<IVec> {
        let mut hit = None;
        'rings: for r in 0..=self.within {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let p = start.offset(dx, dy);
                    if w.map.inb(p) && self.at(w, p) {
                        hit = Some(p);
                        break 'rings;
                    }
                }
            }
        }
        match (self.no, hit) {
            (false, found) => found,
            (true, None) => Some(start),
            (true, Some(_)) => None,
        }
    }
}

/// The start a question is asked about: where the colony stands.
pub fn start_of(sim: &Sim) -> Option<IVec> {
    sim.world.colony_center()
}

/// The `n` seeds of a night's sweep, made from its date (`20260928`).
pub fn date_seeds(date: u64, n: usize) -> Vec<u64> {
    (0..n).map(|i| hash_name(&format!("sweep/{date}/{i}"))).collect()
}

/// Why a soak failed, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub tick: u64,
    pub reason: String,
}

/// One seed played for `days` with the mods `enabled`, then saved and
/// loaded: the state hash, or what went wrong. A panic, a script error, or a
/// save that doesn't load as it was is a failure; a colony dying isn't.
pub fn soak(mods: &Path, enabled: &dyn Fn(&str) -> bool, seed: u64, days: u64) -> Result<u64, Failure> {
    let tick = std::cell::Cell::new(0);
    let run = catch_unwind(AssertUnwindSafe(|| -> Result<u64, Failure> {
        let fail = |tick: u64, reason: String| Failure { tick, reason };
        let mut s = Sim::with_mods(mods, seed, enabled).map_err(|e| fail(0, format!("it didn't load: {e}")))?;
        for _ in 0..days * TICKS_PER_DAY {
            s.step();
            tick.set(s.world.tick);
        }
        if let Some(m) = s.world.messages.iter().find(|m| m.text.contains("script error")) {
            return Err(fail(m.tick, m.text.clone()));
        }
        let once = Snapshot::capture(&s).to_bytes();
        let back = Snapshot::from_bytes(&once)
            .and_then(|snap| snap.restore(mods, enabled))
            .map_err(|e| fail(s.world.tick, format!("its save didn't load: {e}")))?;
        if Snapshot::capture(&back).to_bytes() != once {
            return Err(fail(s.world.tick, "its save loaded as a different world".into()));
        }
        Ok(s.world.state_hash())
    }));
    match run {
        Ok(r) => r,
        Err(p) => {
            let why = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
            Err(Failure { tick: tick.get(), reason: format!("panicked: {}", why.unwrap_or_default()) })
        }
    }
}

/// The command that plays one seed again as the sweep did.
pub fn repro(seed: u64, days: u64) -> String {
    format!("rim seeds soak --seed {seed} --days {days}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_reads_three_ways_and_refuses_others() {
        assert_eq!(
            Where::parse("water near start").unwrap(),
            Where { no: false, feature: "water".into(), within: NEAR }
        );
        assert!(Where::parse("no granite near start").unwrap().no);
        assert_eq!(Where::parse("oak within 6 of start").unwrap().within, 6);
        assert!(Where::parse("water by the start").unwrap_err().contains("near start"));
        assert!(Where::parse("oak within six of start").is_err());
    }

    #[test]
    fn a_night_s_seeds_come_from_its_date() {
        let a = date_seeds(20260928, 200);
        assert_eq!(a.len(), 200);
        assert_eq!(a, date_seeds(20260928, 200));
        assert_ne!(a[0], date_seeds(20260929, 1)[0]);
        let mut unique = a.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 200);
    }
}
