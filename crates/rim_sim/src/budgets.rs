//! Performance budgets, from `budgets.toml` at the repo root (DESIGN.md §8):
//! what each bench's `--check` fails over, and what the docs list. Each
//! budget has a `target`, the goal on the reference machine, and a `cap`,
//! what CI fails over today. A cap only goes down as work lands. On CI a
//! runner class's slack multiplies every cap: shared cores, and software GL.

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub target: f64,
    pub cap: f64,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    pub slack: f64,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ci {
    /// For a runner class not listed.
    pub slack: f64,
    /// By the start of `machine()`'s CPU name.
    #[serde(default)]
    pub class: BTreeMap<String, Class>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Scenario, then measure: `sim.winter.p99_ms`.
    pub sim: BTreeMap<String, BTreeMap<String, Budget>>,
    #[serde(default)]
    pub render: BTreeMap<String, BTreeMap<String, Budget>>,
    pub ci: Ci,
}

impl Budgets {
    pub fn load(path: &Path) -> Result<Budgets, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The repo's: `budgets.toml` two levels above this crate.
    pub fn repo() -> Result<Budgets, String> {
        Budgets::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../budgets.toml"))
    }

    /// What multiplies every cap here: 1 off CI; on CI the runner class's
    /// slack, or the default for a class not listed.
    pub fn slack(&self, machine: &str, ci: bool) -> f64 {
        if !ci {
            return 1.0;
        }
        self.ci
            .class
            .iter()
            .find(|(name, _)| machine.starts_with(name.as_str()))
            .map_or(self.ci.slack, |(_, c)| c.slack)
    }

    /// Each measure of `scenario` over its cap, as lines to print: empty
    /// when everything is within. A time (`_ms`) is held to its cap times
    /// `slack`; a count of work is the same on every machine and isn't.
    pub fn over(
        table: &BTreeMap<String, BTreeMap<String, Budget>>,
        scenario: &str,
        measured: &[(&str, f64)],
        slack: f64,
    ) -> Result<Vec<String>, String> {
        let budgets = table.get(scenario).ok_or_else(|| format!("budgets.toml has no scenario {scenario}"))?;
        let mut over = Vec::new();
        for (name, b) in budgets {
            let &(_, v) = measured
                .iter()
                .find(|(m, _)| m == name)
                .ok_or_else(|| format!("budgets.toml gates {scenario}.{name}, which this bench doesn't measure"))?;
            let slack = if name.ends_with("_ms") { slack } else { 1.0 };
            if v > b.cap * slack {
                over.push(format!(
                    "{scenario}.{name}: {v:.3} over its cap of {:.3} ({} x {slack})",
                    b.cap * slack,
                    b.cap
                ));
            }
        }
        Ok(over)
    }
}

/// The CPU this runs on, and how many threads it has: CI's runner classes
/// differ up to 3x, so a bench names its class and `budgets.toml` scales by
/// it.
pub fn machine() -> String {
    let brand = if cfg!(target_os = "macos") {
        std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("model name").map(|r| r.trim_start_matches([' ', '\t', ':']).to_string()))
        })
    } else {
        std::env::var("PROCESSOR_IDENTIFIER").ok()
    };
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    format!("{}, {threads} threads", brand.filter(|b| !b.is_empty()).unwrap_or_else(|| "unknown CPU".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
[sim.winter]
mean_ms = { target = 0.1, cap = 2.0 }
p99_ms = { target = 1.0, cap = 20.0 }
max_nodes = { target = 5000.0, cap = 40000.0 }

[ci]
slack = 3.0
class."AMD EPYC 9V74" = { slack = 2.0, note = "the fast class" }
"#;

    #[test]
    fn a_cap_times_the_class_s_slack_is_the_line() {
        let b: Budgets = toml::from_str(FILE).unwrap();
        assert_eq!(b.slack("AMD EPYC 9V74 80-Core Processor, 4 threads", true), 2.0);
        assert_eq!(b.slack("Intel(R) Xeon(R) Platinum 8370C, 4 threads", true), 3.0);
        assert_eq!(b.slack("AMD EPYC 9V74 80-Core Processor, 4 threads", false), 1.0);
        let all = |mean, p99, nodes| [("mean_ms", mean), ("p99_ms", p99), ("max_nodes", nodes)];
        let within = Budgets::over(&b.sim, "winter", &all(3.9, 39.0, 40000.0), 2.0).unwrap();
        assert!(within.is_empty(), "{within:?}");
        let over = Budgets::over(&b.sim, "winter", &all(4.1, 1.0, 1.0), 2.0).unwrap();
        assert_eq!(over, ["winter.mean_ms: 4.100 over its cap of 4.000 (2 x 2)"]);
        // A count isn't scaled: 40001 nodes is over 40000 on any machine.
        let over = Budgets::over(&b.sim, "winter", &all(1.0, 1.0, 40001.0), 2.0).unwrap();
        assert_eq!(over, ["winter.max_nodes: 40001.000 over its cap of 40000.000 (40000 x 1)"]);
        assert!(Budgets::over(&b.sim, "winter", &[("mean_ms", 1.0)], 1.0).unwrap_err().contains("max_nodes"));
        assert!(Budgets::over(&b.sim, "spring", &[], 1.0).is_err());
    }

    #[test]
    fn the_repo_s_budgets_load() {
        let b = Budgets::repo().unwrap();
        assert!(b.sim.contains_key("base") && b.sim.contains_key("winter"));
        for (scenario, measures) in b.sim.iter().chain(&b.render) {
            for (name, budget) in measures {
                assert!(budget.target <= budget.cap, "{scenario}.{name}: the target is above the cap");
            }
        }
    }
}
