//! Interaction spots: furniture a pawn uses from a cell its def lays out.
//! A bed is the first user; a chair at a table is the second.

use rim_sim::defs::DefId;
use rim_sim::hecs::Entity;
use rim_sim::world::{Faction, Job, Pawn, NEED_MAX};
use rim_sim::{IVec, Sim};
use std::path::{Path, PathBuf};

fn mods() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods")
}

fn alone(dir: &Path) -> (Sim, Entity) {
    let mut s = Sim::new(dir, 21).expect("mods load");
    let founder = s.world.colonists().next().expect("a founder");
    for e in s.world.pawns.clone() {
        if e != founder {
            let _ = s.world.ecs.despawn(e);
        }
    }
    s.world.pawns.retain(|&e| e == founder);
    (s, founder)
}

fn thing(s: &Sim, id: &str) -> DefId {
    s.world.defs.thing_id(id).unwrap_or_else(|| panic!("thing {id}"))
}

fn need(s: &Sim, id: &str) -> DefId {
    s.world.defs.lookup("need", id).unwrap_or_else(|| panic!("need {id}"))
}

fn set_need(s: &mut Sim, e: Entity, id: &str, v: i32) {
    let n = need(s, id);
    s.world.ecs.get::<&mut Pawn>(e).unwrap().needs.iter_mut().for_each(|x| {
        if x.0 == n {
            x.1 = v;
        }
    });
}

fn open_cells(s: &Sim, n: usize) -> Vec<IVec> {
    let c = s.world.colony_center().expect("a colony");
    (1..30)
        .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| c.offset(dx, dy))))
        .filter(|&p| s.world.map.passable(p) && s.world.map.fixture_at(p).is_none() && s.world.map.item_at(p).is_none())
        .take(n)
        .collect()
}

fn put(s: &mut Sim, id: &str, p: IVec) -> Entity {
    let def = thing(s, id);
    s.world.spawn_fixture(def, p, false).unwrap_or_else(|| panic!("could not place {id}"))
}

fn job(s: &Sim, e: Entity) -> Job {
    s.world.ecs.get::<&Pawn>(e).unwrap().job.clone()
}

fn pos(s: &Sim, e: Entity) -> IVec {
    s.world.pawn_pos(e).expect("alive")
}

#[test]
fn a_bed_is_a_thing_with_a_spot_and_sleeping_on_it_is_unchanged() {
    let (mut s, e) = alone(&mods());
    let at = open_cells(&s, 1)[0];
    let bed = put(&mut s, "bed", at);
    assert_eq!(s.world.defs.thing(thing(&s, "bed")).spots.len(), 1, "core's bed declares its spot");
    set_need(&mut s, e, "rest", NEED_MAX / 10);
    let mut slept_in_bed = false;
    for _ in 0..3000 {
        s.step();
        let p = s.world.ecs.get::<&Pawn>(e).unwrap();
        if p.asleep && matches!(p.job, Job::Sleep { bed: Some(b), .. } if b == bed) {
            assert_eq!(p.pos, at, "sleeps on the bed itself, as before");
            assert_eq!(p.sleep_rate, 180, "at the bed's rate, as before");
            slept_in_bed = true;
            break;
        }
    }
    assert!(slept_in_bed, "a tired colonist uses the bed");
}

#[test]
fn two_pawns_cannot_use_one_spot() {
    let (mut s, a) = alone(&mods());
    let at = open_cells(&s, 1)[0];
    let bed = put(&mut s, "bed", at);
    let human = s.world.defs.creature_id("human").expect("human");
    let b = s.world.spawn_pawn(human, Faction::Player, pos(&s, a), Some("Second".into()));
    for e in [a, b] {
        set_need(&mut s, e, "rest", NEED_MAX / 10);
    }
    let mut in_bed = Vec::new();
    for _ in 0..3000 {
        s.step();
        for e in [a, b] {
            if matches!(job(&s, e), Job::Sleep { bed: Some(x), .. } if x == bed) && !in_bed.contains(&e) {
                in_bed.push(e);
            }
        }
        let both_asleep = [a, b].iter().all(|&e| s.world.ecs.get::<&Pawn>(e).unwrap().asleep);
        if both_asleep {
            break;
        }
    }
    assert_eq!(in_bed.len(), 1, "exactly one of them got the bed: {in_bed:?}");
    let other = if in_bed[0] == a { b } else { a };
    assert!(
        matches!(job(&s, other), Job::Sleep { bed: None, .. }),
        "the other sleeps on the ground: {:?}",
        job(&s, other)
    );
}

// ---------------------------------------------------------- at a table

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

/// Core has a table and a chair (0219); this mod declares its own pair under
/// other names, so the mechanism is proven without leaning on core's content. `who` keeps two tests in one process
/// from building, and then deleting, the same directory under each other.
fn with_furniture(who: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rim-seats-{}-{who}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&mods().join("core"), &dir.join("core"));
    let m = dir.join("seats");
    std::fs::create_dir_all(m.join("defs")).unwrap();
    std::fs::write(
        m.join("mod.toml"),
        "id = \"seats\"\nname = \"Seats\"\nversion = \"0.0.0\"\napi = \"0.6\"\ndepends = [\"core\"]\n",
    )
    .unwrap();
    std::fs::write(
        m.join("defs/seats.toml"),
        r##"
[[thing]]
id = "bench"
label = "bench"
color = "#8c6a48"
category = "building"
tags = ["bench"]
build = { menu = "furniture", work = 200, stuff = { category = "structural", count = 10 } }

[[thing]]
id = "stool"
label = "stool"
color = "#a07a50"
category = "building"
spots = [{ dx = 0, dy = 0, beside = "bench" }]
build = { menu = "furniture", work = 100, stuff = { category = "structural", count = 4 } }
"##,
    )
    .unwrap();
    dir
}

#[test]
fn a_pawn_eats_at_a_table_when_there_is_one() {
    let dir = with_furniture("table");
    let (mut s, e) = alone(&dir);
    let cells = open_cells(&s, 3);
    let table_at = cells[0];
    let chair_at = (1..3)
        .flat_map(|r| [table_at.offset(r, 0), table_at.offset(-r, 0), table_at.offset(0, r), table_at.offset(0, -r)])
        .find(|&p| p.chebyshev(table_at) == 1 && s.world.map.passable(p) && s.world.map.fixture_at(p).is_none())
        .expect("room for a chair beside the table");
    put(&mut s, "bench", table_at);
    let chair = put(&mut s, "stool", chair_at);
    let berries = thing(&s, "berries");
    let food_at = cells[2];
    s.world.place_item(berries, food_at, 10);
    set_need(&mut s, e, "food", NEED_MAX / 10);

    let mut sat = false;
    for _ in 0..4000 {
        s.step();
        if let Job::Eat { seat: Some((c, _)), stage: 1, t, .. } = job(&s, e) {
            if c == chair && t > 0 {
                assert_eq!(pos(&s, e), chair_at, "eats sitting on the chair");
                sat = true;
                break;
            }
        }
    }
    assert!(sat, "the pawn carried its food to the chair");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_chair_alone_in_a_field_is_not_a_seat() {
    let dir = with_furniture("stool");
    let (mut s, e) = alone(&dir);
    let cells = open_cells(&s, 2);
    put(&mut s, "stool", cells[0]);
    let berries = thing(&s, "berries");
    s.world.place_item(berries, cells[1], 10);
    set_need(&mut s, e, "food", NEED_MAX / 10);
    let mut ate = false;
    for _ in 0..4000 {
        s.step();
        if let Job::Eat { seat, t, .. } = job(&s, e) {
            assert!(seat.is_none(), "no table, no seat: {seat:?}");
            if t > 0 {
                ate = true;
                break;
            }
        }
    }
    assert!(ate, "ate where the food lay");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_a_seat_eating_is_exactly_as_before() {
    let (mut s, e) = alone(&mods());
    let at = open_cells(&s, 1)[0];
    let berries = thing(&s, "berries");
    s.world.place_item(berries, at, 10);
    set_need(&mut s, e, "food", NEED_MAX / 10);
    let mut ate_here = false;
    for _ in 0..4000 {
        s.step();
        if let Job::Eat { seat: None, stage: 0, t, .. } = job(&s, e) {
            if t > 0 {
                assert_eq!(pos(&s, e), at, "eats standing where the food is");
                ate_here = true;
                break;
            }
        }
    }
    assert!(ate_here);
}
