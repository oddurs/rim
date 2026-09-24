//! Rooms: enclosed areas bounded by walls, doors, rock and water, cut off from
//! the map edge and no bigger than MAX_ROOM_CELLS.

use rim_sim::map::MAX_ROOM_CELLS;
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
            put(s, if Some(p) == door { "door_wood" } else { "wall_wood" }, p);
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

#[test]
fn oversized_enclosures_are_outdoors() {
    let mut s = sim();
    let o = s.world.colony_center().unwrap().offset(-12, -12);
    let size = 25; // interior 23x23 = 529 cells
    assert!(((size - 2) * (size - 2)) as u32 > MAX_ROOM_CELLS);
    clear(&mut s, o.offset(-1, -1), size + 2);
    ring(&mut s, o, size, None, None);
    s.world.map.ensure_rooms();
    let r = s.world.map.room_at(o.offset(5, 5)).unwrap();
    assert!(!r.touches_edge, "sealed off");
    assert!(!r.enclosed(), "but far too big to count as shelter");
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

    put(&mut s, "wall_wood", o);
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
        "id = \"probe\"\nname = \"Probe\"\nversion = \"0.0.0\"\napi = \"0.1\"\ndepends = [\"core\"]\n",
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
        put(&mut s, "wall_wood", c.offset(dx, dy));
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
