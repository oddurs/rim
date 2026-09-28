//! Rooms: enclosed areas bounded by walls, doors, rock and water, cut off from
//! the map edge and roofed everywhere by the span of their supports.

mod common;

use rim_sim::path::Goal;
use rim_sim::{IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim() -> Sim {
    Sim::new(&mods(), 21).expect("mods load")
}

/// Place a finished fixture, clearing whatever grows there first.
fn put(s: &mut Sim, id: &str, p: IVec) {
    if let Some(f) = s.world.map.fixture_at(p) {
        s.world.despawn_thing(f);
    }
    let def = s.world.defs.thing_id(id).unwrap();
    assert!(s.world.spawn_fixture(def, p, false).is_some(), "could not place {id} at {p:?}");
}

/// A ring of walls `size` cells across with its corner at `o`, and an
/// optional door and gap on the bottom edge.
fn ring(s: &mut Sim, o: IVec, size: i32, door: Option<IVec>, gap: Option<IVec>) {
    for y in 0..size {
        for x in 0..size {
            let p = o.offset(x, y);
            let edge = x == 0 || y == 0 || x == size - 1 || y == size - 1;
            if !edge || Some(p) == gap {
                continue;
            }
            put(s, if Some(p) == door { "door" } else { "wall" }, p);
        }
    }
}

/// Clear a square of plants and rock so interiors are plain floor.
fn clear(s: &mut Sim, o: IVec, size: i32) {
    let grass = s.world.defs.lookup("terrain", "grass").unwrap();
    for y in 0..size {
        for x in 0..size {
            let p = o.offset(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            s.world.map.set_terrain(p, grass, 100);
        }
    }
}

fn site(s: &Sim) -> IVec {
    s.world.colony_center().unwrap().offset(10, 10)
}

#[test]
fn hut_with_a_door_is_enclosed() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 7);
    let door = o.offset(2, 4);
    ring(&mut s, o, 5, Some(door), None);
    s.world.map.ensure_rooms();
    let m = &s.world.map;

    let inside = m.room_at(o.offset(2, 2)).expect("interior has a room");
    assert_eq!(inside.cells, 9, "3x3 interior");
    assert!(inside.enclosed());
    for y in 1..4 {
        for x in 1..4 {
            assert!(m.indoors(o.offset(x, y)), "({x},{y}) should be indoors");
        }
    }
    assert!(m.room_at(door).is_none(), "a doorway belongs to no room");
    assert!(m.room_at(o).is_none(), "a wall belongs to no room");
    assert!(!m.indoors(o.offset(2, 6)), "outside the door is outdoors");
}

#[test]
fn a_gap_in_the_wall_means_outdoors() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 7);
    ring(&mut s, o, 5, None, Some(o.offset(2, 4)));
    s.world.map.ensure_rooms();
    let r = s.world.map.room_at(o.offset(2, 2)).unwrap();
    assert!(r.touches_edge, "interior leaks out through the gap to the map edge");
    assert!(!s.world.map.indoors(o.offset(2, 2)));
}

#[test]
fn doors_split_rooms_but_not_paths() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 7);
    ring(&mut s, o, 5, Some(o.offset(2, 4)), None);
    let (inside, outside) = (o.offset(2, 2), o.offset(2, 6));
    s.world.map.ensure_rooms();
    s.world.map.ensure_regions();
    let m = &s.world.map;
    assert_ne!(m.room_at(inside).unwrap().id, m.room_at(outside).unwrap().id);
    assert_eq!(m.region_at(inside), m.region_at(outside), "same pathing region through the door");
    let w = &mut s.world;
    // Nobody built this door, so it opens for anyone.
    let f = rim_sim::world::Faction::Player;
    assert!(w.pf.find(&w.map, outside, Goal::Cell(inside), 10_000, f).is_some(), "can walk in through the door");
}

/// Place a finished fixture made of `stuff`, clearing whatever grows there.
fn put_of(s: &mut Sim, id: &str, stuff: &str, p: IVec) {
    if let Some(f) = s.world.map.fixture_at(p) {
        s.world.despawn_thing(f);
    }
    let (def, m) = (s.world.defs.thing_id(id).unwrap(), s.world.defs.thing_id(stuff).unwrap());
    assert!(s.world.spawn_fixture_of(def, p, false, Some(m)).is_some(), "could not place {id} at {p:?}");
}

#[test]
fn oversized_enclosures_are_outdoors() {
    let mut s = sim();
    let o = s.world.colony_center().unwrap().offset(-12, -12);
    // Interior 23 across: its middle is 12 cells from any wall, three times
    // what a wall with no material holds up.
    let size = 25;
    clear(&mut s, o.offset(-1, -1), size + 2);
    ring(&mut s, o, size, None, None);
    s.world.map.ensure_rooms();
    let r = s.world.map.room_at(o.offset(5, 5)).unwrap();
    assert!(!r.touches_edge, "sealed off");
    assert!(r.uncovered > 0 && !r.enclosed(), "but its middle is open to the sky");
}

/// A room is roofed within its supports' span (DESIGN.md §6c): a log
/// hall twelve wide and ten deep is outdoors until a pillar holds up its
/// middle, and a pillar taken away opens it again.
#[test]
fn a_hall_too_wide_for_its_walls_needs_a_pillar() {
    let mut s = sim();
    let o = site(&s);
    let (w, h) = (14, 12);
    clear(&mut s, o.offset(-1, -1), w + 2);
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                put_of(&mut s, "wall", "wood", o.offset(x, y));
            }
        }
    }
    s.world.map.ensure_rooms();
    let middle = o.offset(w / 2, h / 2);
    let i = s.world.map.idx(middle);
    let r = s.world.map.room_at(middle).unwrap();
    assert!(!r.touches_edge && !r.enclosed(), "wood walls hold 4 cells, and the middle is 5 away");
    assert!(!s.world.map.roofed(i));
    assert!(s.world.map.roofed(s.world.map.idx(o.offset(4, 4))), "near the walls it is roofed");
    put_of(&mut s, "pillar", "wood", o.offset(w / 2, h / 2));
    s.world.map.ensure_rooms();
    let r = s.world.map.room_at(o.offset(3, 3)).unwrap();
    assert_eq!(r.uncovered, 0);
    assert!(r.enclosed(), "a pillar in the middle roofs the rest");
    let pillar = s.world.map.fixture_at(o.offset(w / 2, h / 2)).unwrap();
    s.world.despawn_thing(pillar);
    s.world.map.ensure_rooms();
    assert!(!s.world.map.room_at(middle).unwrap().enclosed(), "and taking it away opens the roof again");
}

/// The material sets the span: a branch hut holds 2 cells, so a 3×3
/// interior is roofed and a 5×5 one isn't.
#[test]
fn the_material_sets_the_span() {
    let mut s = Sim::new(&mods(), 21).expect("mods load");
    for (size, roofed) in [(5, true), (7, false)] {
        let o = site(&s).offset(size * 3, 0);
        clear(&mut s, o.offset(-1, -1), size + 2);
        for y in 0..size {
            for x in 0..size {
                if x == 0 || y == 0 || x == size - 1 || y == size - 1 {
                    put_of(&mut s, "wall", "branches", o.offset(x, y));
                }
            }
        }
        s.world.map.ensure_rooms();
        let r = s.world.map.room_at(o.offset(size / 2, size / 2)).unwrap();
        assert_eq!(r.enclosed(), roofed, "a branch hut {size} across");
    }
}

/// Rock holds a roof: a room dug into granite is sheltered however it is
/// shaped, as long as rock is within reach.
#[test]
fn rock_holds_a_roof() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 9);
    let granite = s.world.defs.thing_id("granite").unwrap();
    for y in 0..7 {
        for x in 0..7 {
            if x == 0 || y == 0 || x == 6 || y == 6 {
                assert!(s.world.spawn_fixture(granite, o.offset(x, y), false).is_some());
            }
        }
    }
    s.world.map.ensure_rooms();
    assert!(s.world.map.room_at(o.offset(3, 3)).unwrap().enclosed());
}

#[test]
fn rooms_rebuild_only_when_walls_change() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 7);
    s.world.map.ensure_rooms();
    let base = s.world.map.room_rebuilds;

    // Items, pawns walking about and time passing leave rooms alone.
    let wood = s.world.defs.thing_id("wood").unwrap();
    s.world.place_item(wood, o.offset(2, 2), 10);
    for _ in 0..500 {
        s.step();
    }
    assert_eq!(s.world.map.room_rebuilds, base, "rebuilt without a wall changing");

    put(&mut s, "wall", o);
    s.step();
    s.step();
    assert_eq!(s.world.map.room_rebuilds, base + 1, "one wall, one rebuild");
}

/// Scripts can ask about shelter. A probe mod reports the room under the
/// founder, whom we wall in on the spot.
#[test]
fn scripts_see_rooms() {
    let dir = std::env::temp_dir().join(format!("rim-rooms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let probe = dir.join("probe");
    std::fs::create_dir_all(probe.join("scripts")).unwrap();
    std::fs::write(
        probe.join("mod.toml"),
        "id = \"probe\"\nname = \"Probe\"\nversion = \"0.0.0\"\napi = \"0.7\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        probe.join("scripts/probe.luau"),
        r#"
local done = false
rim.every(1, function()
	if done then return end
	done = true
	local x, y = rim.colony_center()
	local r = rim.room_at(x, y)
	rim.message(`probe indoors={rim.indoors(x, y)} cells={r.cells} enclosed={r.enclosed}`)
end)
"#,
    )
    .unwrap();

    let mut s = Sim::new(&dir, 21).expect("mods load");
    let c = s.world.colony_center().unwrap();
    for (dx, dy) in rim_sim::map::NEIGHBORS8 {
        put(&mut s, "wall", c.offset(dx, dy));
    }
    s.step();
    let _ = std::fs::remove_dir_all(&dir);
    let texts: Vec<_> = s.world.messages.iter().map(|m| m.text.clone()).collect();
    assert!(texts.iter().any(|t| t == "probe indoors=true cells=1 enclosed=true"), "messages: {texts:?}");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// The role of the room at `p`, by qualified id.
fn role(s: &mut Sim, p: IVec) -> Option<String> {
    s.world.map.ensure_rooms();
    s.world.ensure_roles();
    s.world.room_role(p).map(|d| s.world.defs.room_roles[d as usize].id.clone())
}

/// A room takes the first `[[room_role]]` it meets, from what's inside it
/// (DESIGN.md §6c): a bed and a fire make a home, a second bed a dormitory.
#[test]
fn a_room_takes_the_first_role_it_meets() {
    let mut s = sim();
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 9);
    ring(&mut s, o, 7, Some(o.offset(3, 6)), None);
    let inside = o.offset(3, 3);
    assert_eq!(role(&mut s, inside), None, "an empty room has no role");
    put(&mut s, "bed", o.offset(1, 1));
    assert_eq!(role(&mut s, inside).as_deref(), Some("core:bedroom"));
    put(&mut s, "campfire", o.offset(5, 5));
    assert_eq!(role(&mut s, inside).as_deref(), Some("core:home"));
    put(&mut s, "bed", o.offset(3, 1));
    assert_eq!(role(&mut s, inside).as_deref(), Some("core:dormitory"));
    // Taking a bed away changes the role without any wall changing.
    let bed = s.world.map.fixture_at(o.offset(3, 1)).unwrap();
    s.world.despawn_thing(bed);
    assert_eq!(role(&mut s, inside).as_deref(), Some("core:home"));
    // Knock a hole in the wall and it's outdoors: no role needs an open room.
    let wall = s.world.map.fixture_at(o.offset(0, 3)).unwrap();
    s.world.despawn_thing(wall);
    assert_eq!(role(&mut s, inside), None);
}

/// A mod adds a role with data alone, and a script reads it.
#[test]
fn a_mod_adds_a_role_and_a_script_reads_it() {
    let defs = r#"
[[room_role]]
id = "reading_room"
label = "Reading room"
needs = { seat = 2 }
min_cells = 9
"#;
    let script = r#"
        rim.every(1, function()
            local x, y = rim.get_data("probe:at_x"), rim.get_data("probe:at_y")
            if x == nil then return end
            local r = rim.room_at(x, y)
            rim.set_data("seen", { role = r and r.role or "none", label = r and r.role_label or "none" })
        end)
    "#;
    let dir = common::test_mods(
        "room-role",
        &["core"],
        &[("probe", &[("defs/rooms.toml", defs), ("scripts/probe.luau", script)])],
    );
    let mut s = Sim::new(&dir, 21).unwrap_or_else(|e| panic!("loads: {e}"));
    let _ = std::fs::remove_dir_all(dir);
    let o = site(&s);
    clear(&mut s, o.offset(-1, -1), 9);
    ring(&mut s, o, 7, Some(o.offset(3, 6)), None);
    put(&mut s, "chair", o.offset(1, 1));
    put(&mut s, "chair", o.offset(2, 1));
    let inside = o.offset(3, 3);
    assert_eq!(role(&mut s, inside).as_deref(), Some("probe:reading_room"));
    s.world.data.insert("probe:at_x".into(), rim_sim::data::Data::Int(inside.x as i64));
    s.world.data.insert("probe:at_y".into(), rim_sim::data::Data::Int(inside.y as i64));
    s.step();
    let seen = s.world.data.get("probe:seen").expect("the script ran");
    assert_eq!(seen.get("role"), Some(&rim_sim::data::Data::Str("probe:reading_room".into())));
    assert_eq!(seen.get("label"), Some(&rim_sim::data::Data::Str("Reading room".into())));
}

#[test]
fn a_role_that_needs_nothing_is_refused() {
    let dir = common::test_mods(
        "room-role-empty",
        &["core"],
        &[("probe", &[("defs/rooms.toml", "[[room_role]]\nid = \"any\"\nlabel = \"Any\"\nneeds = {}\n")])],
    );
    let e = Sim::new(&dir, 1).err().expect("an empty role doesn't load");
    let _ = std::fs::remove_dir_all(dir);
    assert!(e.contains("room_role/probe:any") && e.contains("`needs` is empty"), "{e}");
}

/// Untouched rock is terrain, not a thing (DESIGN.md §6d), and holds a
/// roof all the same: a pocket dug into it is sheltered.
#[test]
fn a_pocket_dug_into_solid_rock_is_roofed() {
    let mut s = sim();
    let m = &s.world.map;
    let solid = |p: IVec| s.world.solid_at(p).is_some();
    let o = (0..m.h - 9)
        .flat_map(|y| (0..m.w - 9).map(move |x| IVec::new(x, y)))
        .find(|&o| (0..9).all(|y| (0..9).all(|x| solid(o.offset(x, y)))))
        .expect("a stretch of solid rock");
    let leaves = s.world.solid_at(o).and_then(|s| s.leaves_r).expect("rock that can be dug");
    let cost = s.world.defs.terrain[leaves as usize].path_cost;
    for y in 3..6 {
        for x in 3..6 {
            s.world.map.set_terrain(o.offset(x, y), leaves, cost);
        }
    }
    s.world.map.ensure_rooms();
    let r = s.world.map.room_at(o.offset(4, 4)).expect("the pocket is a room");
    assert_eq!(r.cells, 9);
    assert!(r.enclosed(), "rock all round holds its roof");
}
