//! The worked example in docs/modding/work.md loads and plays: the guide
//! can't drift from what the engine accepts. Blocks after
//! `<!-- example: PATH -->` are the example mod's files.

mod common;

use rim_sim::{Sim, TICKS_PER_DAY};

fn example_files() -> Vec<(String, String)> {
    let guide = std::fs::read_to_string(common::mods().join("../docs/modding/work.md")).unwrap().replace("\r\n", "\n");
    let mut out = Vec::new();
    for part in guide.split("<!-- example: ").skip(1) {
        let (path, rest) = part.split_once(" -->").unwrap();
        let body = rest.split("```").nth(1).unwrap();
        let body = body.split_once('\n').unwrap().1;
        out.push((path.to_string(), body.to_string()));
    }
    out
}

#[test]
fn the_tailoring_example_loads_and_plays() {
    let files = example_files();
    assert!(files.len() >= 2, "the guide has its example: {files:?}");
    let owned: Vec<(&str, &str)> = files.iter().map(|(p, b)| (p.as_str(), b.as_str())).collect();
    let dir = common::test_mods("work-guide", &["core"], &[("tailor", &owned)]);
    let mut s = Sim::new(&dir, 1).unwrap_or_else(|e| panic!("the example loads: {e}"));
    let d = s.world.defs.clone();
    assert!(d.lookup("work_type", "tailor:tailor").is_some());
    assert!(s.world.work_roles.iter().any(|r| r.def.as_deref() == Some("tailor:tailor")));
    // Day six of ten: clothes past half worn, the order holds.
    for _ in 0..6 * TICKS_PER_DAY + TICKS_PER_DAY / 12 {
        s.step();
    }
    let worn = s.world.standing.reading("tailor:worn").expect("the script publishes its reading");
    assert!(worn > 0.5, "{worn}");
    let rule = d.lookup("priority_rule", "tailor:worn").unwrap();
    assert!(s.world.rules.on.contains(&rule), "the order holds");
    let _ = std::fs::remove_dir_all(dir);
}
