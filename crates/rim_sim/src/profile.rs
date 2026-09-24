//! Per-system and per-mod timing. Wall-clock only; never feeds the sim.

use std::time::Instant;

#[derive(Default)]
pub struct Profile {
    /// (name, smoothed microseconds per call), for the live profiler.
    pub entries: Vec<(String, f64)>,
    /// (name, total microseconds, calls) since the last `reset_totals`, for
    /// benchmarks that need exact means.
    pub totals: Vec<(String, f64, u64)>,
}

impl Profile {
    pub fn add(&mut self, name: &str, micros: f64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.0 == name) {
            e.1 = e.1 * 0.97 + micros * 0.03;
        } else {
            self.entries.push((name.to_string(), micros));
        }
        if let Some(t) = self.totals.iter_mut().find(|t| t.0 == name) {
            t.1 += micros;
            t.2 += 1;
        } else {
            self.totals.push((name.to_string(), micros, 1));
        }
    }

    pub fn reset_totals(&mut self) {
        self.totals.clear();
    }

    pub fn time<R>(&mut self, name: &str, f: impl FnOnce() -> R) -> R {
        let t = Instant::now();
        let r = f();
        self.add(name, t.elapsed().as_secs_f64() * 1e6);
        r
    }
}
