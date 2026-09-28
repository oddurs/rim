//! Invariants the example tests check only at a few points, over random edits
//! on small maps: saves are exact, regions and rooms after edits equal a
//! fresh build, paths only step where a pawn can, and patches that touch
//! different things give the same defs in any load order.
//!
//! The cases come from the test's seed (DESIGN.md §7b), so the nightly shift
//! gives each property new ones, and a failure prints the smallest case
//! proptest shrank it to, then the line that runs it again.

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use rim_sim::map::Map;
use rim_sim::path::{Goal, Pathfinder};
use rim_sim::snapshot::Snapshot;
use rim_sim::testseed::TestSeed;
use rim_sim::world::Faction;
use rim_sim::{IVec, Sim};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A runner whose cases come from the calling test's seed. Keep the seed
/// alive for the whole test: it prints the rerun line if the test fails.
fn runner(cases: u32) -> (TestRunner, TestSeed) {
    let seed = TestSeed::new();
    let config = Config {
        cases,
        rng_seed: RngSeed::Fixed(seed.seed),
        // The seed brings a failure back; no regression files in the tree.
        failure_persistence: None,
        ..Config::default()
    };
    (TestRunner::new(config), seed)
}

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

// ---------------------------------------------------------------------------
// 1. Save, load, save gives the same bytes.

#[derive(Clone, Debug)]
enum Play {
    /// Let the world run.
    Run(u32),
    /// Drop a stack near the colony.
    Stack { dx: i32, dy: i32, count: u32 },
    /// Put up a wall near the colony.
    Wall { dx: i32, dy: i32 },
}

fn play() -> impl Strategy<Value = Play> {
    prop_oneof![
        (1u32..400).prop_map(Play::Run),
        (-8i32..8, -8i32..8, 1u32..40).prop_map(|(dx, dy, count)| Play::Stack { dx, dy, count }),
        (-8i32..8, -8i32..8).prop_map(|(dx, dy)| Play::Wall { dx, dy }),
    ]
}

#[test]
fn save_load_save_gives_the_same_bytes() {
    let (mut runner, _seed) = runner(12);
    let base = Sim::build(&mods(), 1, &|m| m == "core", 64).unwrap();
    let base = Snapshot::capture(&base).to_bytes();
    let core = |m: &str| m == "core";
    runner
        .run(&prop::collection::vec(play(), 0..8), |plays| {
            let mut s = Snapshot::from_bytes(&base).unwrap().restore(&mods(), &core).unwrap();
            let c = s.world.colony_center().unwrap();
            let (wood, wall) = (s.world.defs.thing_id("wood").unwrap(), s.world.defs.thing_id("wall").unwrap());
            for p in &plays {
                match *p {
                    Play::Run(n) => (0..n).for_each(|_| s.step()),
                    Play::Stack { dx, dy, count } => {
                        let at = c.offset(dx, dy);
                        if s.world.map.passable(at) && s.world.map.item_at(at).is_none() {
                            s.world.place_item(wood, at, count);
                        }
                    }
                    Play::Wall { dx, dy } => {
                        let _ = s.world.spawn_fixture(wall, c.offset(dx, dy), false);
                    }
                }
            }
            let once = Snapshot::capture(&s).to_bytes();
            let back = Snapshot::from_bytes(&once).unwrap().restore(&mods(), &core).unwrap();
            let twice = Snapshot::capture(&back).to_bytes();
            prop_assert!(
                once == twice,
                "the second save differs from the first ({} and {} bytes)",
                once.len(),
                twice.len()
            );
            Ok(())
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// 2. Regions and rooms after edits equal a fresh build of the same cells.

const W: i32 = 20;

/// What a cell holds, as the map is told it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Floor,
    Water,
    Wall,
    Door,
    /// A door only the colony may use.
    OwnedDoor,
}

/// Terrain and fixtures change apart, as in a game: a wall goes up on
/// ground, water floods a cell whatever stands in it.
#[derive(Clone, Debug)]
enum Edit {
    Terrain(i32, i32, bool),
    Fixture(i32, i32, Cell),
    /// Rebuild now: the next edits land on a map that has rebuilt before.
    Rebuild,
}

fn cell() -> impl Strategy<Value = Cell> {
    prop_oneof![1 => Just(Cell::Floor), 1 => Just(Cell::Water), 3 => Just(Cell::Wall), 1 => Just(Cell::Door), 1 => Just(Cell::OwnedDoor)]
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        2 => (0..W, 0..W, any::<bool>()).prop_map(|(x, y, w)| Edit::Terrain(x, y, w)),
        6 => (0..W, 0..W, cell()).prop_map(|(x, y, c)| Edit::Fixture(x, y, c)),
        1 => Just(Edit::Rebuild),
    ]
}

fn put(m: &mut Map, p: IVec, c: Cell) {
    put_terrain(m, p, c == Cell::Water);
    put_fixture(m, p, c);
}

fn put_terrain(m: &mut Map, p: IVec, water: bool) {
    m.set_terrain(p, 0, if water { 0 } else { 100 });
}

/// Water isn't a fixture: as one, it means none.
fn put_fixture(m: &mut Map, p: IVec, c: Cell) {
    let (blocks, door) = match c {
        Cell::Floor | Cell::Water => (false, false),
        Cell::Wall => (true, false),
        Cell::Door | Cell::OwnedDoor => (false, true),
    };
    m.set_fixture(p, None, blocks, 100, door);
    m.set_owner(p, (c == Cell::OwnedDoor).then_some(Faction::Player));
}

/// Every cell's region, for each faction, and its room.
fn partition(m: &Map) -> Vec<(u32, u32, u32, u32)> {
    (0..m.plane())
        .map(|i| {
            let p = m.pos(i);
            let r = |f| m.region_at_for(p, f);
            (r(Faction::Player), r(Faction::Hostile), r(Faction::Wild), m.room_ids(i).0)
        })
        .collect()
}

#[test]
fn regions_and_rooms_after_edits_equal_a_fresh_build() {
    let (mut runner, _seed) = runner(64);
    runner
        .run(&prop::collection::vec(edit(), 1..60), |edits| {
            let mut live = Map::new(W, W);
            let mut cells: BTreeMap<(i32, i32), (bool, Cell)> = BTreeMap::new();
            live.ensure_regions();
            live.ensure_rooms();
            for e in &edits {
                match *e {
                    Edit::Terrain(x, y, water) => {
                        put_terrain(&mut live, IVec::new(x, y), water);
                        cells.entry((x, y)).or_insert((false, Cell::Floor)).0 = water;
                    }
                    Edit::Fixture(x, y, c) => {
                        put_fixture(&mut live, IVec::new(x, y), c);
                        cells.entry((x, y)).or_insert((false, Cell::Floor)).1 = c;
                    }
                    Edit::Rebuild => {
                        live.ensure_regions();
                        live.ensure_rooms();
                    }
                }
            }
            live.ensure_regions();
            live.ensure_rooms();
            let mut fresh = Map::new(W, W);
            for (&(x, y), &(water, c)) in &cells {
                put_terrain(&mut fresh, IVec::new(x, y), water);
                put_fixture(&mut fresh, IVec::new(x, y), c);
            }
            fresh.ensure_regions();
            fresh.ensure_rooms();
            let (a, b) = (partition(&live), partition(&fresh));
            let first = (0..a.len()).find(|&i| a[i] != b[i]);
            prop_assert!(
                first.is_none(),
                "cell {:?}: after edits {:?}, fresh {:?}",
                first.map(|i| live.pos(i)),
                first.map(|i| a[i]),
                first.map(|i| b[i])
            );
            Ok(())
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// 3. A path only steps where a pawn can, and is found when the goal is
//    reachable.

#[test]
fn a_path_only_steps_where_a_pawn_can() {
    let (mut runner, _seed) = runner(64);
    let case = (prop::collection::vec((0..W, 0..W, cell()), 0..120), (0..W, 0..W), (0..W, 0..W));
    runner
        .run(&case, |(cells, (sx, sy), (gx, gy))| {
            let mut m = Map::new(W, W);
            for &(x, y, c) in &cells {
                put(&mut m, IVec::new(x, y), c);
            }
            m.ensure_regions();
            let (start, goal) = (IVec::new(sx, sy), IVec::new(gx, gy));
            let who = Faction::Hostile;
            prop_assume!(start != goal && m.passable_for(start, who));
            let reach = m.region_at_for(start, who) == m.region_at_for(goal, who) && m.passable_for(goal, who);
            let path = Pathfinder::default().find(&m, start, Goal::Cell(goal), u32::MAX, who);
            prop_assert_eq!(path.is_some(), reach, "found a path exactly when the goal is reachable");
            let Some(path) = path else { return Ok(()) };
            prop_assert_eq!(path.first().copied(), Some(goal), "the path ends at the goal");
            let mut at = start;
            for &q in path.iter().rev() {
                let (dx, dy) = (q.x - at.x, q.y - at.y);
                prop_assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0), "{at:?} to {q:?} is one step");
                prop_assert!(m.passable_for(q, who), "{q:?} can be stood on");
                if dx != 0 && dy != 0 {
                    let (side, other) = (IVec::new(at.x + dx, at.y), IVec::new(at.x, at.y + dy));
                    prop_assert!(
                        m.passable_for(side, who) && m.passable_for(other, who),
                        "{at:?} to {q:?} cuts a corner"
                    );
                }
                at = q;
            }
            Ok(())
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// 4. Patches from mods that don't depend on each other give the same defs in
//    any load order, when no two set the same field. (Two that do conflict by
//    design: the later wins, with a warning.)

/// The fields a patch here sets.
const FIELDS: [&str; 2] = ["market_value", "hp"];

/// A base mod with things to patch, and patching mods loaded in `order`.
/// A patch is (thing, field, value).
fn load_in_order(dir: &Path, patches: &[Vec<(usize, usize, u32)>], order: &[usize]) -> Vec<(u32, u32)> {
    let _ = std::fs::remove_dir_all(dir);
    let write = |path: PathBuf, text: String| {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    let manifest = |id: &str, after: &[String]| {
        let deps = if id == "base" { String::new() } else { "depends = [\"base\"]\n".to_string() };
        let after = after.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(", ");
        format!("id = \"{id}\"\nname = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.6\"\n{deps}load_after = [{after}]\n")
    };
    write(dir.join("base/mod.toml"), manifest("base", &[]));
    // The least a game loads with (docs/modding/replacing-core.md), then the
    // things the patches aim at.
    let mut things = "[[terrain]]\nid = \"ground\"\nlabel = \"ground\"\ncolor = \"#6b8f4e\"\npath_cost = 100\n\
        gen = { elevation = [0.0, 1.0], priority = 1 }\n\n[[creature]]\nid = \"settler\"\nlabel = \"settler\"\n\
        color = \"#d8b890\"\nsize = 0.36\nspeed = 13\nmax_hp = 100\nmelee_damage = 5\nmelee_cooldown = 90\nneeds = []\n\n\
        [[start]]\nid = \"landing\"\ncreature = \"settler\"\ntitle = \"the settler\"\n\n"
        .to_string();
    for t in 0..THINGS {
        things += &format!("[[thing]]\nid = \"t{t}\"\nlabel = \"t{t}\"\ncolor = \"#808080\"\ncategory = \"item\"\n\n");
    }
    write(dir.join("base/defs/things.toml"), things);
    // Mod `order[k]` loads after `order[k - 1]`.
    for (k, &m) in order.iter().enumerate() {
        let after: Vec<String> = order[..k].last().map(|p| vec![format!("p{p}")]).unwrap_or_default();
        write(dir.join(format!("p{m}/mod.toml")), manifest(&format!("p{m}"), &after));
        let mut text = String::new();
        for &(thing, field, value) in &patches[m] {
            let field = FIELDS[field];
            text += &format!("[[patch]]\ntarget = \"thing/base:t{thing}\"\nset = {{ {field} = {value} }}\n\n");
        }
        write(dir.join(format!("p{m}/defs/patches.toml")), text);
    }
    let loaded = rim_sim::modloader::load_only(dir, &|_| true).unwrap();
    let defs = &loaded.defs;
    (0..THINGS)
        .map(|t| {
            let d = defs.thing(defs.thing_id(&format!("base:t{t}")).unwrap());
            (d.market_value as u32, d.hp)
        })
        .collect()
}

const THINGS: usize = 6;

#[test]
fn patches_give_the_same_defs_in_any_load_order() {
    let (mut runner, _seed) = runner(16);
    let dir = std::env::temp_dir().join(format!("rim-patch-orders-{}", std::process::id()));
    // Up to four mods; each (thing, field) is set by at most one of them.
    let case = prop::collection::btree_map((0..THINGS, 0..FIELDS.len()), (0..4usize, 1u32..500), 1..10).prop_flat_map(
        |owners| {
            let n = owners.values().map(|o| o.0).max().unwrap() + 1;
            (Just(owners), Just((0..n).collect::<Vec<_>>()).prop_shuffle())
        },
    );
    runner
        .run(&case, |(owners, order)| {
            let n = order.len();
            let mut patches = vec![Vec::new(); n];
            for (&(thing, field), &(m, value)) in &owners {
                patches[m].push((thing, field, value));
            }
            let natural: Vec<usize> = (0..n).collect();
            let a = load_in_order(&dir, &patches, &natural);
            let b = load_in_order(&dir, &patches, &order);
            prop_assert_eq!(a, b, "load order {:?} against {:?}", order, natural);
            Ok(())
        })
        .unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
