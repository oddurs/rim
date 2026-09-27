//! Mining that rewards looking (DESIGN.md §6d): each rock gives its own
//! blocks, and flint lies in chalk as veins that deplete as they're worked.

mod common;

use rim_sim::Sim;

fn sim(seed: u64) -> Sim {
    Sim::new(&common::mods(), seed).expect("mods load")
}

fn vein(s: &Sim) -> usize {
    s.world.defs.lookup("field", "primitive:flint_vein").unwrap() as usize
}

/// Every veined cell below the surface, in map order: (index, amount).
fn veined(s: &Sim) -> Vec<(usize, i32)> {
    let f = vein(s);
    s.world.fields.layers[f].stock.iter().enumerate().filter(|(_, &v)| v > 0).map(|(i, &v)| (i, v)).collect()
}

#[test]
fn each_rock_gives_its_own_blocks_with_their_own_factors() {
    let s = sim(1);
    let d = &s.world.defs;
    let yields = |rock: &str| {
        let t = d.thing(d.thing_id(rock).unwrap());
        d.thing(t.harvest[0].yields_r[0].0).id.clone()
    };
    let blocks = [yields("core:granite"), yields("core:limestone"), yields("core:chalk")];
    assert_eq!(blocks, ["core:stone", "core:limestone_blocks", "core:chalk_blocks"]);
    let hp = |id: &str| d.factor(d.thing_id(id), "hp");
    assert!(hp("core:stone") > hp("core:limestone_blocks") && hp("core:limestone_blocks") > hp("core:chalk_blocks"));
}

#[test]
fn veins_are_laid_in_chalk_the_same_way_for_the_same_seed() {
    let (a, b, c) = (sim(4), sim(4), sim(5));
    let va = veined(&a);
    assert!(!va.is_empty(), "flint veins below");
    let chalk = a.world.defs.lookup("terrain", "chalk").unwrap();
    assert!(va.iter().all(|&(i, _)| a.world.map.terrain[i] == chalk), "only in chalk");
    assert!(va.iter().all(|&(i, _)| i >= a.world.map.plane()), "only below the surface");
    assert_eq!(va, veined(&b), "the same seed, the same veins");
    assert_ne!(va, veined(&c), "another seed, other veins");
}

#[test]
fn a_vein_yields_flint_until_it_is_spent_and_the_chalk_stays() {
    let mut s = sim(4);
    let (i, amount) = veined(&s)[0];
    let p = s.world.map.pos(i);
    let rock = s.world.wake_rock(p).expect("chalk stands up to be worked");
    let d = s.world.defs.clone();
    let extract = d.thing(s.world.thing(rock).unwrap().def).harvest.iter().find(|h| h.draw.is_some()).unwrap().clone();
    let draw = extract.draw.as_ref().unwrap();
    // Up to three a time, while the cell holds some.
    let mut taken = 0;
    while s.world.harvest_ready(rock, extract.key()) {
        let n = s.world.drawable(p, draw);
        assert!((1..=3).contains(&n));
        s.world.fields.set_stock(&d, &s.world.map, draw.field_r, p, -(n as f64), true);
        taken += n;
    }
    assert_eq!(taken as i32, amount / 10_000, "all the vein held, and no more");
    assert!(s.world.thing(rock).is_some(), "extracting leaves the rock");
    assert!(!s.world.harvest_ready(rock, extract.key()), "a spent vein offers nothing");
}

/// Prospecting is looking: the flint overlay shows a veined cell only once
/// a colonist has seen it, and the level below starts unseen.
#[test]
fn a_vein_shows_once_it_has_been_seen() {
    let mut s = sim(4);
    let f = vein(&s);
    assert!(s.world.defs.fields[f].until_seen, "the flint field is shown only where seen");
    let (i, _) = veined(&s)[0];
    assert!(!s.world.map.seen(i), "rock below starts unseen");
    let p = s.world.map.pos(i);
    s.world.map.see_around(p);
    assert!(s.world.map.seen(i), "seen once a colonist stands by it");
}
