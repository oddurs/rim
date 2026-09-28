//! House plans as data (DESIGN.md §6c): a grid of characters placed whole
//! with one command, turned with the placement.

mod common;

use rim_sim::world::MadeOf;
use rim_sim::{Command, IVec, Sim};

const DEFS: &str = r##"
[[thing]]
id = "bench"
label = "bench"
color = "#8a6a45"
category = "building"
size = [2, 1]
look.layers = [{ draw = "fill" }]
build = { menu = "furniture", work = 30, cost = [{ thing = "core:wood", count = 2 }] }

[[plan]]
id = "shed"
label = "shed"
grid = """
##+
#.bb
"""
legend = { "#" = { thing = "core:wall", stuff = "core:wood" }, "+" = { thing = "core:door", stuff = "core:wood" }, "b" = { thing = "bench" } }
"##;

fn shed(name: &str, defs: &str) -> Result<Sim, String> {
    let dir = common::test_mods(name, &["core"], &[("house", &[("defs/house.toml", defs)])]);
    let sim = Sim::with_mods(&dir, 5, &|_| true);
    let _ = std::fs::remove_dir_all(dir);
    sim
}

/// A square of open ground near the colony, `n` on a side.
fn open_square(s: &Sim, n: i32) -> IVec {
    let c = s.world.colony_center().unwrap();
    let open =
        |p: IVec| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none();
    (3..40)
        .flat_map(|r| [c.offset(r, 0), c.offset(-r, 0), c.offset(0, r), c.offset(0, -r)])
        .find(|&o| (0..n).all(|y| (0..n).all(|x| open(o.offset(x, y)))))
        .expect("open ground")
}

#[test]
fn a_plan_turned_east_matches_its_grid_turned_east() {
    let mut s = shed("plans-east", DEFS).unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let (plan, wall, door, bench, stone) = (
        d.lookup("plan", "house:shed").unwrap(),
        d.thing_id("wall").unwrap(),
        d.thing_id("door").unwrap(),
        d.thing_id("house:bench").unwrap(),
        d.thing_id("stone").unwrap(),
    );
    let o = open_square(&s, 5);
    s.push(Command::PlacePlan { plan, at: o, facing: 1, stuff: Some(stone) });
    s.step();
    // The shed's grid, a quarter turn clockwise.
    let east = ["##", ".#", "b+", "b."];
    for (y, row) in east.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let p = o.offset(x as i32, y as i32);
            let got = s.world.map.fixture_at(p).and_then(|f| s.world.thing(f).map(|t| (f, t.def)));
            let want = match c {
                '#' => Some(wall),
                '+' => Some(door),
                'b' => Some(bench),
                _ => None,
            };
            assert_eq!(got.map(|g| g.1), want, "'{c}' at {x},{y}");
            if let Some((f, def)) = got {
                let made = s.world.ecs.get::<&MadeOf>(f).ok().map(|m| m.0);
                // The placement's stone replaces the plan's wood; the bench takes no material.
                assert_eq!(made, (def != bench).then_some(stone), "'{c}' at {x},{y} is made of the placement's stone");
            }
        }
    }
    // One bench, turned with the house and covering both of its cells.
    let b = s.world.map.fixture_at(o.offset(0, 2)).unwrap();
    assert_eq!(s.world.map.fixture_at(o.offset(0, 3)), Some(b));
    assert_eq!(s.world.thing(b).unwrap().facing, 1);
}

#[test]
fn a_character_not_in_the_legend_fails_the_load() {
    let bad = DEFS.replace("##+", "##x");
    let err = shed("plans-bad", &bad).err().expect("refused");
    assert!(err.contains("'x' at row 1, column 3 isn't in the legend"), "{err}");
}

/// A plan keeps the level it's placed on: at z -1 every piece is planned
/// there (the rock in the way marked to be dug), and none on the surface.
#[test]
fn a_plan_placed_below_the_surface_stays_there() {
    let mut s = shed("plans-below", DEFS).unwrap_or_else(|e| panic!("loads: {e}"));
    assert!(*s.world.map.levels().start() < 0, "the map has a level below ({:?})", s.world.map.levels());
    let plan = s.world.defs.lookup("plan", "house:shed").unwrap();
    let o = open_square(&s, 5);
    let below = IVec::at(o.x, o.y, -1);
    let surface: Vec<_> =
        (0..4).flat_map(|y| (0..4).map(move |x| o.offset(x, y))).map(|p| s.world.map.fixture_at(p)).collect();
    s.push(Command::PlacePlan { plan, at: below, facing: 0, stuff: None });
    s.step();
    let defs = s.world.defs.clone();
    for piece in defs.plans[plan as usize].placed(&defs, below, 0) {
        let at = IVec::at(piece.at.0, piece.at.1, -1);
        let f = s.world.map.fixture_at(at).unwrap_or_else(|| panic!("something planned at {at:?}"));
        let planned = s.world.ecs.get::<&rim_sim::world::Planned>(f).is_ok_and(|p| p.thing == piece.thing)
            || s.world.thing(f).is_some_and(|t| t.def == piece.thing);
        assert!(planned, "the piece at {at:?} is planned on its level");
    }
    let after: Vec<_> =
        (0..4).flat_map(|y| (0..4).map(move |x| o.offset(x, y))).map(|p| s.world.map.fixture_at(p)).collect();
    assert_eq!(after, surface, "nothing new on the surface");
}

/// What stands can be written out as a plan and placed again: the shed
/// saved as text loads as a mod's plan and puts up the same pieces.
#[test]
fn a_saved_selection_loads_back_and_places_the_same_pieces() {
    let mut s = shed("plans-save", DEFS).unwrap_or_else(|e| panic!("loads: {e}"));
    let plan = s.world.defs.lookup("plan", "house:shed").unwrap();
    let o = open_square(&s, 5);
    s.push(Command::PlacePlan { plan, at: o, facing: 1, stuff: None });
    s.step();
    let text = rim_sim::plan::plan_text(&s.world, o, o.offset(3, 3), "saved", "saved shed");
    let kind = |s: &Sim, p: IVec| {
        s.world.map.fixture_at(p).and_then(|e| {
            let t = s.world.thing(e)?;
            Some((
                s.world.defs.thing(t.def).id.clone(),
                s.world.made_of(e).map(|m| s.world.defs.thing(m).id.clone()),
                t.facing,
            ))
        })
    };
    let before: Vec<_> =
        (0..4).flat_map(|y| (0..4).map(move |x| (x, y))).map(|(x, y)| kind(&s, o.offset(x, y))).collect();

    let mut back = shed("plans-save-back", &format!("{DEFS}\n{text}"))
        .unwrap_or_else(|e| panic!("the saved plan loads: {e}\n{text}"));
    let saved = back.world.defs.lookup("plan", "house:saved").expect("the saved plan");
    let q = open_square(&back, 5);
    back.push(Command::PlacePlan { plan: saved, at: q, facing: 0, stuff: None });
    back.step();
    let after: Vec<_> =
        (0..4).flat_map(|y| (0..4).map(move |x| (x, y))).map(|(x, y)| kind(&back, q.offset(x, y))).collect();
    assert_eq!(after, before, "the same pieces, materials and facings\n{text}");
}

/// A plan is one storey: it's read from the level its corner is on.
#[test]
fn a_plan_is_saved_from_its_own_level() {
    let mut s = shed("plans-save-level", DEFS).unwrap_or_else(|e| panic!("loads: {e}"));
    let d = &s.world.defs;
    let (wall, wood) = (d.thing_id("wall").unwrap(), d.thing_id("wood").unwrap());
    let o = open_square(&s, 5);
    let below = IVec::at(o.x, o.y, -1);
    for x in 0..3 {
        let p = IVec::at(o.x + x, o.y, -1);
        if let Some(e) = s.world.map.fixture_at(p) {
            s.world.despawn_thing(e);
        }
        s.world.spawn_fixture_of(wall, p, false, Some(wood)).expect("a wall below");
    }
    let deep = rim_sim::plan::plan_text(&s.world, below, below.offset(2, 0), "deep", "deep");
    let top = rim_sim::plan::plan_text(&s.world, o, o.offset(2, 0), "top", "top");
    assert!(deep.contains("core:wall") && deep.contains("\nwww\n"), "the walls below are saved:\n{deep}");
    assert!(!top.contains("core:wall"), "and not read from the surface:\n{top}");
}
