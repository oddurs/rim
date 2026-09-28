//! The live frame budget, first in the F3 profiler: the last second's
//! median and worst frame, the draw calls the client counts itself, and
//! the biggest render pass. A frame is the wall time from one render to the
//! next, so it includes the present and, with vsync, the wait for it.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How far back the numbers look.
const WINDOW: Duration = Duration::from_secs(1);

/// How often the line is written anew: a number changing every frame would
/// reflow the panel every frame.
const REFRESH: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct FrameStats {
    last: Option<Instant>,
    /// The line, and when it was written.
    line: (String, Option<Instant>),
    /// When each frame ended, and how long it took, ms.
    frames: VecDeque<(Instant, f64)>,
    calls: usize,
    biggest: (&'static str, f64),
}

impl FrameStats {
    /// At the end of a frame's render: its draw calls and its passes' CPU
    /// time (µs), by name.
    pub fn record(&mut self, calls: usize, passes: &[(&'static str, f64)]) {
        self.record_at(Instant::now(), calls, passes);
    }

    fn record_at(&mut self, now: Instant, calls: usize, passes: &[(&'static str, f64)]) {
        if let Some(last) = self.last {
            self.frames.push_back((now, (now - last).as_secs_f64() * 1e3));
        }
        self.last = Some(now);
        while self.frames.front().is_some_and(|(t, _)| now - *t > WINDOW) {
            self.frames.pop_front();
        }
        self.calls = calls;
        self.biggest = passes.iter().copied().fold(("-", 0.0), |a, b| if b.1 > a.1 { b } else { a });
        if self.line.1.is_none_or(|t| now - t >= REFRESH) {
            self.line = (self.write(), Some(now));
        }
    }

    /// The line as last written: "frame 8.4 ms, worst 21.0 · 96 calls ·
    /// biggest light 2.25 ms".
    pub fn line(&self) -> &str {
        &self.line.0
    }

    fn write(&self) -> String {
        let mut ms: Vec<f64> = self.frames.iter().map(|f| f.1).collect();
        ms.sort_by(f64::total_cmp);
        let median = ms.get(ms.len() / 2).copied().unwrap_or(0.0);
        let worst = ms.last().copied().unwrap_or(0.0);
        format!(
            "frame {median:.1} ms, worst {worst:.1} · {} mesh and figure calls · biggest {} {:.2} ms",
            self.calls,
            self.biggest.0,
            self.biggest.1 / 1e3
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_holds_the_last_second_s_median_and_worst_frame() {
        let mut f = FrameStats::default();
        let t0 = Instant::now();
        let passes = [("ground", 300.0), ("light", 2250.0), ("ui", 900.0)];
        // A frame of 8 ms, then one of 40, then ten of 10.
        let mut t = t0;
        for ms in [0, 8, 40, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10] {
            t += Duration::from_millis(ms);
            f.record_at(t, 96, &passes);
        }
        assert_eq!(f.write(), "frame 10.0 ms, worst 40.0 · 96 mesh and figure calls · biggest light 2.25 ms");
        // The line was last written 250 ms or less ago, not every frame.
        assert_ne!(f.line(), f.write());
        // Two seconds on, only the frame that ends it is left, and the line
        // is written anew.
        f.record_at(t + Duration::from_secs(2), 90, &passes);
        assert_eq!(f.line(), "frame 2000.0 ms, worst 2000.0 · 90 mesh and figure calls · biggest light 2.25 ms");
    }
}
