//! docs/engineering/benchmarks.md lists the budgets from budgets.toml, the
//! same file the benches check against, so the two can't drift.
//! `RIM_UPDATE_DOCS=1` rewrites the table.

use rim_sim::budgets::Budgets;
use std::path::Path;

const BEGIN: &str = "<!-- budgets.toml: begin -->";
const END: &str = "<!-- budgets.toml: end -->";

/// The table: every scenario and measure, with its target and cap.
fn table(b: &Budgets) -> String {
    let mut out = String::from("| Budget | Target | Cap |\n|---|---|---|\n");
    for (kind, table) in [("sim", &b.sim), ("render", &b.render)] {
        for (scenario, measures) in table {
            for (name, budget) in measures {
                out += &format!("| `{kind}.{scenario}.{name}` | {} | {} |\n", budget.target, budget.cap);
            }
        }
    }
    out += &format!("\nOn CI, a time (`_ms`) is held to its cap times the runner class's slack: {}", slacks(b));
    out.push('\n');
    out
}

fn slacks(b: &Budgets) -> String {
    let mut parts: Vec<String> = b.ci.class.iter().map(|(name, c)| format!("{} on {name}", c.slack)).collect();
    parts.push(format!("{} on any other", b.ci.slack));
    parts.join(", ") + "."
}

#[test]
fn the_benchmarks_guide_lists_the_budgets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("docs/engineering/benchmarks.md");
    let doc = std::fs::read_to_string(&path).unwrap();
    let (a, b) = (doc.find(BEGIN).expect("the begin marker"), doc.find(END).expect("the end marker"));
    let want = table(&Budgets::repo().unwrap());
    let have = &doc[a + BEGIN.len() + 1..b];
    if have != want {
        if std::env::var_os("RIM_UPDATE_DOCS").is_some() {
            let new = format!("{}{BEGIN}\n{want}{}", &doc[..a], &doc[b..]);
            std::fs::write(&path, new).unwrap();
            return;
        }
        panic!(
            "docs/engineering/benchmarks.md's budgets table differs from budgets.toml; \
             RIM_UPDATE_DOCS=1 cargo test -p rim_sim --test budgets_doc rewrites it"
        );
    }
}
