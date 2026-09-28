//! A game with no `core` mod (DESIGN.md §5): core is a plugin like any other,
//! so a total conversion can replace it. The fixture declares the least a
//! game needs (docs/modding/replacing-core.md) and a second mod adds a thing
//! and a script on top.

use rim_sim::snapshot::Snapshot;
use rim_sim::{Sim, TICKS_PER_DAY};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

#[test]
fn a_game_with_no_core_loads_runs_a_day_and_round_trips_a_save() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/no_core");
    let run = catch_unwind(AssertUnwindSafe(|| {
        let mut s = Sim::build(&dir, 1, &|_| true, 32).map_err(|e| format!("it didn't load: {e}"))?;
        if !s.warnings.is_empty() {
            return Err(format!("it loaded with warnings: {:?}", s.warnings));
        }
        for _ in 0..TICKS_PER_DAY {
            s.step();
        }
        let back = Snapshot::capture(&s).restore(&dir, &|_| true).map_err(|e| format!("its save didn't load: {e}"))?;
        Ok((s, back))
    }));
    let (s, back) = match run {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            panic!("a game with no core mod: {e}. Engine code may need a `core:` id, which DESIGN.md §5 rules out")
        }
        Err(p) => {
            let why = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
            panic!(
                "a game with no core mod panicked: {}. Engine code may need a `core:` id, which DESIGN.md §5 rules out",
                why.unwrap_or_default()
            )
        }
    };
    let ids: Vec<&str> = s.world.defs.things.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.iter().all(|id| !id.starts_with("core:")), "no core: ids anywhere: {ids:?}");
    assert_eq!(s.world.colonists().count(), 1, "the start's one settler");
    assert_eq!(s.world.data.get("content:hours"), Some(&rim_sim::data::Data::Int(25)), "the script ran every hour");
    assert_eq!(s.world.stock.on_map(s.world.defs.thing_id("content:pebble").unwrap()), 25, "and laid its pebbles");
    assert_eq!(back.world.state_hash(), s.world.state_hash(), "the save carries on as if never saved");
}

/// docs/modding/replacing-core.md lists every kind of def the fixture's
/// vocabulary mod declares: it is what a replacement must declare.
#[test]
fn the_replacing_core_guide_lists_what_the_fixture_declares() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = std::fs::read_to_string(root.join("../../docs/modding/replacing-core.md")).unwrap();
    let mut kinds = std::collections::BTreeSet::new();
    for e in std::fs::read_dir(root.join("tests/fixtures/no_core/vocab/defs")).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap();
        kinds.extend(text.lines().filter(|l| l.starts_with("[[")).map(|l| l.trim().to_string()));
    }
    assert!(!kinds.is_empty());
    for k in &kinds {
        assert!(guide.contains(&format!("| `{k}` |")), "the guide's table lacks {k}");
    }
}
