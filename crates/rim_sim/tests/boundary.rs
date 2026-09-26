//! Rooms made of something: a room's leak and daylight come from the
//! pieces that enclose it, scaled by what those pieces are made of.

use rim_sim::defs::DefId;
use rim_sim::field::ROOM_INTERVAL;
use rim_sim::{IVec, Sim, TICKS_PER_DAY};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn sim() -> Sim {
    Sim::new(&mods(), 21).expect("mods load")
}

fn field(s: &Sim, id: &str) -> usize {
    s.world.defs.lookup("field", id).unwrap_or_else(|| panic!("field {id}")) as usize
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

/// Top-left corner of a 5x5 block of passable ground near the colony that
/// does not overlap `taken`.
fn site(s: &Sim, taken: &[IVec]) -> IVec {
    let c = s.world.colony_center().expect("a colony");
    (2..40)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .find(|&o| {
            (0..5).all(|y| (0..5).all(|x| s.world.map.passable(o.offset(x, y))))
                && taken.iter().all(|t| (t.x - o.x).abs() >= 7 || (t.y - o.y).abs() >= 7)
        })
        .expect("open ground for a hut")
}

/// A 5x5 hut with its corner at `o`, walls of `material`, interior cleared.
/// `door` and `window` name cells on the ring to use instead of wall.
fn hut(s: &mut Sim, o: IVec, material: DefId, door: Option<IVec>, pane: Option<(IVec, DefId)>) -> IVec {
    let (wall, door_def) = (thing(s, "wall"), thing(s, "door"));
    for y in 0..5 {
        for x in 0..5 {
            let p = o.offset(x, y);
            if let Some(f) = s.world.map.fixture_at(p) {
                s.world.despawn_thing(f);
            }
            if let Some(i) = s.world.map.item_at(p) {
                s.world.despawn_thing(i);
            }
            if !(x == 0 || y == 0 || x == 4 || y == 4) {
                continue;
            }
            let def = if Some(p) == door {
                door_def
            } else if let Some((_, pd)) = pane.filter(|(pp, _)| *pp == p) {
                pd
            } else {
                wall
            };
            s.world.spawn_fixture_of(def, p, false, Some(material)).expect("hut piece placed");
        }
    }
    s.world.map.ensure_rooms();
    s.world.refresh_boundaries();
    o.offset(2, 2)
}

fn room_id(s: &Sim, p: IVec) -> u32 {
    let r = s.world.map.room_at(p).expect("a room");
    assert!(r.enclosed(), "the hut should be enclosed");
    r.id
}

fn run_at(s: &mut Sim, temp: f64, ticks: u64) {
    let t = field(s, "temperature");
    for _ in 0..ticks {
        s.world.fields.set_ambient(t, Some(temp));
        s.step();
    }
}

#[test]
fn a_wooden_hut_leaks_exactly_as_it_did_before_there_were_boundaries() {
    let mut s = sim();
    let wood = thing(&s, "wood");
    let o = site(&s, &[]);
    let inside = hut(&mut s, o, wood, None, None);
    let t = field(&s, "temperature");
    let (leak, pass) = s.world.fields.boundary(t, room_id(&s, inside));
    assert!((leak - 1.0).abs() < 1e-9, "wood is the baseline: leak multiplier {leak}");
    assert_eq!(pass, 0.0, "a wall lets no daylight in");
}

#[test]
fn stone_holds_heat_better_than_wood() {
    let mut s = sim();
    let (wood, stone, fire) = (thing(&s, "wood"), thing(&s, "stone"), thing(&s, "campfire"));
    let a = site(&s, &[]);
    let b = site(&s, &[a]);
    let in_wood = hut(&mut s, a, wood, None, None);
    let in_stone = hut(&mut s, b, stone, None, None);
    let t = field(&s, "temperature");
    let (lw, _) = s.world.fields.boundary(t, room_id(&s, in_wood));
    let (ls, _) = s.world.fields.boundary(t, room_id(&s, in_stone));
    assert!(ls < lw, "stone leaks less: {ls} vs {lw}");

    // The same fire in each. A campfire is capped at its room value, so
    // while it burns both huts sit near the cap and the walls barely show;
    // the material shows in what the room keeps once the fire is out.
    let fires: Vec<_> =
        [in_wood, in_stone].iter().map(|&c| s.world.spawn_fixture(fire, c, false).expect("a fire")).collect();
    run_at(&mut s, 2.0, TICKS_PER_DAY / 12);
    let temp = |s: &Sim, p| s.world.fields.value(&s.world.defs, &s.world.map, t, p);
    let (tw, ts) = (temp(&s, in_wood), temp(&s, in_stone));
    assert!(tw > 15.0 && ts > 15.0, "both huts warmed on the same fire: wood {tw}° stone {ts}°");
    assert!(ts >= tw, "stone is never the colder one: stone {ts}° vs wood {tw}°");

    for f in fires {
        s.world.despawn_thing(f);
    }
    run_at(&mut s, 2.0, TICKS_PER_DAY / 8);
    let (tw, ts) = (temp(&s, in_wood), temp(&s, in_stone));
    assert!(tw > 2.0 && ts > 2.0, "neither has fallen to outdoors yet: wood {tw}° stone {ts}°");
    assert!(ts > tw + 0.8, "three cold hours later the stone hut is warmer: stone {ts}° vs wood {tw}°");
}

#[test]
fn a_door_makes_a_room_leakier() {
    let mut s = sim();
    let wood = thing(&s, "wood");
    let a = site(&s, &[]);
    let b = site(&s, &[a]);
    let sealed = hut(&mut s, a, wood, None, None);
    let doored = hut(&mut s, b, wood, Some(b.offset(2, 4)), None);
    let t = field(&s, "temperature");
    let (l0, _) = s.world.fields.boundary(t, room_id(&s, sealed));
    let (l1, _) = s.world.fields.boundary(t, room_id(&s, doored));
    assert!(l1 > l0, "a door leaks more than the wall it replaced: {l1} vs {l0}");
    // One piece of sixteen at 1.5 instead of 1.0.
    assert!((l1 - (15.0 + 1.5) / 16.0).abs() < 1e-9, "{l1}");
}

#[test]
fn the_boundary_is_the_whole_ring_corners_included() {
    let mut s = sim();
    let wood = thing(&s, "wood");
    let o = site(&s, &[]);
    let inside = hut(&mut s, o, wood, None, None);
    let id = room_id(&s, inside);
    assert_eq!(s.world.map.room_boundary(id).len(), 16, "a 5x5 ring is sixteen pieces");
}

#[test]
fn rebuilding_a_wall_elsewhere_keeps_the_numbers() {
    let mut s = sim();
    let (wood, stone) = (thing(&s, "wood"), thing(&s, "stone"));
    let a = site(&s, &[]);
    let inside = hut(&mut s, a, stone, None, None);
    let t = field(&s, "temperature");
    let before = s.world.fields.boundary(t, room_id(&s, inside));
    // A wall far away forces a room rebuild.
    let far = site(&s, &[a]);
    s.world.spawn_fixture_of(thing(&s, "wall"), far, false, Some(wood)).expect("a wall");
    s.step();
    let after = s.world.fields.boundary(t, room_id(&s, inside));
    assert_eq!(before, after, "the stone hut is still the stone hut");
}

// ---------------------------------------------------------- daylight

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

/// The mechanism a window will be pure content on top of: a piece that
/// passes half the daylight, declared by a mod, lights the room.
#[test]
fn a_pane_lets_daylight_in() {
    let dir = std::env::temp_dir().join(format!("rim-pane-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let m = dir.join("panes");
    std::fs::create_dir_all(m.join("defs")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        "id = \"panes\"\nname = \"Panes\"\nversion = \"0.0.0\"\napi = \"0.6\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        m.join("defs/pane.toml"),
        r##"
[[thing]]
id = "pane"
label = "pane"
color = "#cfe8ff"
category = "building"
blocks = true
hp = 60
boundary = [{ field = "core:light", pass = 0.5 }, { field = "core:temperature", leak = 3.0 }]
build = { menu = "structure", work = 100, stuff = { category = "structural", count = 2 } }
"##,
    )
    .unwrap();

    let mut s = Sim::new(&dir, 21).expect("core + panes load");
    let (wood, pane) = (thing(&s, "wood"), thing(&s, "pane"));
    let a = site(&s, &[]);
    let b = site(&s, &[a]);
    let dark = hut(&mut s, a, wood, None, None);
    let lit = hut(&mut s, b, wood, None, Some((b.offset(2, 0), pane)));
    let (light, temp) = (field(&s, "light"), field(&s, "temperature"));
    let (_, p0) = s.world.fields.boundary(light, room_id(&s, dark));
    let (_, p1) = s.world.fields.boundary(light, room_id(&s, lit));
    assert_eq!(p0, 0.0, "no pane, no daylight");
    assert!((p1 - 0.5).abs() < 1e-9, "one pane passes half: {p1}");

    // Noon outside; the field's own ambient update runs every 20 ticks.
    s.world.fields.set_ambient(light, Some(100.0));
    for _ in 0..25 {
        s.step();
    }
    let inside_dark = s.world.fields.value(&s.world.defs, &s.world.map, light, dark);
    let inside_lit = s.world.fields.value(&s.world.defs, &s.world.map, light, lit);
    assert!(inside_dark < 1.0, "sealed hut is dark: {inside_dark}");
    assert!((inside_lit - 50.0).abs() < 1.0, "the pane lights the room to half: {inside_lit}");
    let (lt, _) = s.world.fields.boundary(temp, room_id(&s, lit));
    assert!(lt > 1.0, "and it leaks heat: {lt}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Criterion 3: the boundary recompute rides the room rebuild. Measured,
/// and bounded generously so a pathological regression is caught in CI.
#[test]
fn boundary_refresh_is_cheap() {
    let mut s = sim();
    let wood = thing(&s, "wood");
    let mut taken = Vec::new();
    for _ in 0..12 {
        let o = site(&s, &taken);
        hut(&mut s, o, wood, None, None);
        taken.push(o);
    }
    // Force a rebuild and time only the refresh.
    let probe = site(&s, &taken);
    s.world.spawn_fixture_of(thing(&s, "wall"), probe, false, Some(wood)).expect("a wall");
    s.world.map.ensure_rooms();
    let t0 = std::time::Instant::now();
    s.world.refresh_boundaries();
    let ms = t0.elapsed().as_secs_f64() * 1e3;
    println!("boundary refresh over {} rooms: {ms:.3} ms", s.world.map.room_count());
    assert!(ms < 5.0, "boundary refresh took {ms} ms");
    let _ = ROOM_INTERVAL;
}

/// Two identical warm huts, one in a gale: the one in the wind loses its
/// heat faster (8376a04f). The wind field is the same everywhere, so each
/// run has one hut and one wind.
fn warm_hut_after(wind: f64) -> f64 {
    let mut s = sim();
    let wood = thing(&s, "wood");
    let o = site(&s, &[]);
    let inside = hut(&mut s, o, wood, None, None);
    let (t, w) = (field(&s, "temperature"), field(&s, "wind"));
    let fire = s.world.spawn_fixture(thing(&s, "campfire"), inside, false).expect("a fire");
    run_at(&mut s, 2.0, TICKS_PER_DAY / 12);
    s.world.despawn_thing(fire);
    s.world.fields.set_ambient(w, Some(wind));
    run_at(&mut s, 2.0, TICKS_PER_DAY / 8);
    s.world.fields.value(&s.world.defs, &s.world.map, t, inside)
}

#[test]
fn a_room_cools_faster_in_a_gale() {
    let (calm, breeze, gale) = (warm_hut_after(0.0), warm_hut_after(3.0), warm_hut_after(16.0));
    assert_eq!(calm, breeze, "a breeze doesn't find the gaps");
    assert!(gale < calm - 1.0, "a storm wind draws the heat out: {gale}° vs {calm}° in calm");
}

/// A field that still says `leak_per_hour` loads and leaks as it did.
#[test]
fn a_constant_leak_still_loads() {
    let dir = std::env::temp_dir().join(format!("rim-test-leak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let f = dir.join("core/defs/fields.toml");
    let text = std::fs::read_to_string(&f).unwrap();
    let start = text.find("[field.leak.sealed]").unwrap();
    let end = text.find("# Core's climate").unwrap();
    let text = format!("{}{}", &text[..start], &text[end..])
        .replace("boundary_factor", "leak_per_hour = 0.06\nboundary_factor");
    std::fs::write(&f, text).unwrap();
    let s = Sim::new(&dir, 21).expect("leak_per_hour loads");
    let t = field(&s, "temperature");
    let fd = &s.world.defs.fields[t];
    assert!(fd.leak_terms.terms.is_empty() && fd.leak_per_hour == 0.06);
    let _ = std::fs::remove_dir_all(dir);
}
