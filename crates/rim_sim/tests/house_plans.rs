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
