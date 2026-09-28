//! Research as a plugin (mods/research): the engine knows only the stat
//! pipeline's modifiers. A project's modifiers hold its buildables back,
//! and the research script switches them off once colonists have studied
//! it at a desk.

mod common;

use rim_sim::data::Data;
use rim_sim::snapshot::Snapshot;
use rim_sim::{Command, IVec, Sim, TICKS_PER_DAY};
use std::collections::BTreeMap;

/// Joinery in two sessions at a desk, so a test sees it done.
const QUICK: &str = r##"
[[patch]]
target = "research:project/research:joinery"
set = { work = 400 }
"##;

fn mods(name: &str) -> std::path::PathBuf {
    let ship = ["core", "crafting", "primitive", "timber", "research"];
    common::test_mods(name, &ship, &[("quick", &[("defs/quick.toml", QUICK)])])
}

fn sim(dir: &std::path::Path) -> Sim {
    let mut s = Sim::new(dir, 3).expect("mods load");
    common::hands(&mut s);
    s
}

fn send(s: &mut Sim, name: &str, pairs: &[(&str, Data)]) {
    let t: BTreeMap<rim_sim::data::Key, Data> =
        pairs.iter().map(|(k, v)| (rim_sim::data::Key::Str(k.to_string()), v.clone())).collect();
    s.push(Command::ModEvent { name: name.into(), data: Some(Data::Table(t)) });
}

fn open_near(s: &Sim, at: IVec) -> IVec {
    common::loose_cells(s, at, 1)[0]
}

/// A desk the colony has, which the research script is told of.
fn desk(s: &mut Sim) -> rim_sim::hecs::Entity {
    let def = s.world.defs.thing_id("research:desk").unwrap();
    let at = open_near(s, s.world.colony_center().unwrap().offset(3, 0));
    let e = s.world.spawn_fixture(def, at, false).expect("a desk");
    send(s, "research:desk", &[("site", Data::Int(e.to_bits().get() as i64))]);
    s.step();
    e
}

fn shelves_at(s: &Sim, p: IVec) -> bool {
    let shelf = s.world.defs.thing_id("timber:shelf").unwrap();
    s.world.map.fixture_at(p).and_then(|e| s.world.thing(e)).is_some_and(|t| t.def == shelf)
}

fn build_shelf(s: &mut Sim, at: IVec) {
    let d = &s.world.defs;
    let (thing, stuff) = (d.thing_id("timber:shelf").unwrap(), d.thing_id("timber:planks"));
    s.push(Command::Build { thing, stuff, a: at, b: at, facing: 0 });
    s.step();
}

fn progress(s: &Sim, project: &str) -> Option<Data> {
    let Some(Data::Table(st)) = s.world.data.get("research:state") else { return None };
    match st.get(&rim_sim::data::Key::Str("progress".into())) {
        Some(Data::Table(p)) => p.get(&rim_sim::data::Key::Str(project.into())).cloned(),
        _ => None,
    }
}

#[test]
fn a_project_holds_its_buildables_back_until_it_is_studied() {
    let dir = mods("research-gate");
    let mut s = sim(&dir);
    let shelf = s.world.defs.thing_id("timber:shelf").unwrap();
    // Locked through the pipeline: its buildable stat is 0, with a reason.
    assert_eq!(s.world.def_stat(shelf, "buildable"), Some(0.0));
    assert_eq!(s.world.build_lock(shelf).as_deref(), Some("Needs research: Joinery"));
    let crate_ = s.world.defs.thing_id("timber:crate").unwrap();
    assert_eq!(s.world.build_lock(crate_), None, "what no project gates is free");
    let at = open_near(&s, s.world.colony_center().unwrap().offset(-4, 4));
    build_shelf(&mut s, at);
    assert!(!shelves_at(&s, at), "no plan for a locked shelf");

    // Study joinery at a desk: two sessions of 200.
    desk(&mut s);
    send(&mut s, "research:choose", &[("project", Data::Str("research:joinery".into()))]);
    let done = |s: &Sim| s.world.build_lock(shelf).is_none();
    let mut studied = false;
    for _ in 0..TICKS_PER_DAY {
        s.step();
        if done(&s) {
            studied = true;
            break;
        }
    }
    assert!(studied, "joinery studied: {:?}", s.world.messages.last());
    assert_eq!(s.world.def_stat(shelf, "buildable"), Some(1.0));
    build_shelf(&mut s, at);
    assert!(shelves_at(&s, at), "a shelf's plan goes down once joinery is done");
}

#[test]
fn research_progress_and_unlocks_survive_a_save() {
    let dir = mods("research-save");
    let mut s = sim(&dir);
    desk(&mut s);
    send(&mut s, "research:choose", &[("project", Data::Str("research:joinery".into()))]);
    // One session in: part way.
    let mut part = false;
    for _ in 0..TICKS_PER_DAY {
        s.step();
        if progress(&s, "research:joinery").is_some() {
            part = true;
            break;
        }
    }
    assert!(part, "some study done");
    let back = Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    assert_eq!(progress(&back, "research:joinery"), progress(&s, "research:joinery"));
    assert_eq!(back.world.state_hash(), s.world.state_hash());

    // Finish it, save again: the shelf stays unlocked after the load.
    let shelf = s.world.defs.thing_id("timber:shelf").unwrap();
    for _ in 0..TICKS_PER_DAY {
        s.step();
        if s.world.build_lock(shelf).is_none() {
            break;
        }
    }
    assert!(s.world.build_lock(shelf).is_none());
    let back = Snapshot::capture(&s).restore(&dir, &|_| true).expect("loads");
    assert!(back.world.build_lock(shelf).is_none(), "an unlock is kept in the save");
    assert_eq!(back.world.state_hash(), s.world.state_hash());
}

/// The engine's part, with no research at all: a data modifier on a stat,
/// switched by the mod that declares it, and one on a thing that isn't
/// loaded does nothing.
#[test]
fn modifiers_add_to_a_stat_and_their_mod_switches_them() {
    let data = r##"
[[modifier]]
id = "tough_walls"
group = "masonry"
stat = "hp"
thing = "core:wall"
value = 50

[[modifier]]
id = "no_campfire"
group = "masonry"
stat = "buildable"
thing = "core:campfire"
value = -1
reason = "Not yet"

[[modifier]]
id = "ghost"
stat = "buildable"
thing = "nowhere:ghost"
value = -1

[[modifier]]
id = "typo"
stat = "buildable"
thing = "core:wal"
value = -1
"##;
    let dir = common::test_mods("modifiers", &["core"], &[("walls", &[("defs/walls.toml", data)])]);
    let mut s = Sim::new(&dir, 1).expect("mods load");
    // A mod that isn't installed: silent, as a patch to it would be. A
    // loaded mod's thing that isn't there: a mistake, and said.
    assert!(!s.warnings.iter().any(|w| w.contains("modifier/walls:ghost")), "{:?}", s.warnings);
    assert!(s.warnings.iter().any(|w| w.contains("modifier/walls:typo")), "{:?}", s.warnings);
    let d = &s.world.defs;
    let (wall, fire) = (d.thing_id("core:wall").unwrap(), d.thing_id("core:campfire").unwrap());
    let base = d.thing(wall).hp as f64;
    assert_eq!(s.world.def_stat(wall, "hp"), Some(base + 50.0));
    assert_eq!(s.world.build_lock(fire).as_deref(), Some("Not yet"));
    let before = s.world.state_hash();
    assert_eq!(s.world.set_modifiers("walls", "masonry", false), 2);
    assert_eq!(s.world.def_stat(wall, "hp"), Some(base));
    assert_eq!(s.world.build_lock(fire), None);
    assert_ne!(s.world.state_hash(), before, "switches are world state");
    // Another mod's switch doesn't reach them.
    assert_eq!(s.world.set_modifiers("core", "masonry", true), 0);
    assert_eq!(s.world.build_lock(fire), None);
}

/// Every TOML sample in docs/modding/research.md loads, in one mod beside
/// core and research, and does what the guide says.
#[test]
fn guide_samples_load() {
    let guide =
        std::fs::read_to_string(common::mods().join("../docs/modding/research.md")).unwrap().replace("\r\n", "\n");
    let samples: Vec<&str> = guide.split("```toml\n").skip(1).map(|b| b.split("```").next().unwrap()).collect();
    assert_eq!(samples.len(), 2);
    let files: &[(&str, &str)] = &[("defs/guide.toml", &samples.join("\n"))];
    let dir = common::test_mods("research-guide", &["core", "research"], &[("guide", files)]);
    let s = Sim::new(&dir, 3).unwrap_or_else(|e| panic!("the guide's samples don't load: {e}"));
    let fire = s.world.defs.thing_id("core:campfire").unwrap();
    assert_eq!(s.world.build_lock(fire).as_deref(), Some("Needs research: Fire lore"));
}
