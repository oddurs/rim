//! Per-system and per-mod timing. Wall-clock only; never feeds the sim.

use std::time::Instant;

#[derive(Default)]
pub struct Profile {
    /// (name, smoothed microseconds per call)
    pub entries: Vec<(String, f64)>,
}

impl Profile {
    pub fn add(&mut self, name: &str, micros: f64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.0 == name) {
            e.1 = e.1 * 0.97 + micros * 0.03;
        } else {
            self.entries.push((name.to_string(), micros));
        }
    }

    pub fn time<R>(&mut self, name: &str, f: impl FnOnce() -> R) -> R {
        let t = Instant::now();
        let r = f();
        self.add(name, t.elapsed().as_secs_f64() * 1e6);
        r
    }
}
