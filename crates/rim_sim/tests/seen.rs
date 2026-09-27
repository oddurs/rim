//! What pawns have seen (DESIGN.md §6d): one bit per cell, set where a
//! pawn stands and around it, kept by a save. The surface starts seen.

mod common;

use rim_sim::map::Map;
use rim_sim::snapshot::Snapshot;
use rim_sim::world::Faction;
use rim_sim::{IVec, Sim};

#[test]
fn the_surface_starts_seen_and_the_levels_below_do_not() {
    let m = Map::with_levels(8, 8, 2, 1);
    let seen = |z: i32| m.seen(m.idx(IVec::at(4, 4, z)));
    assert!(seen(0) && seen(1), "the surface and what is over it");
    assert!(!seen(-1) && !seen(-2), "the levels below");
}

#[test]
fn a_pawn_sees_the_cells_around_it_and_touches_what_it_saw() {
    let mut m = Map::with_levels(40, 40, 1, 0);
    let p = IVec::at(33, 5, -1);
    let chunk = m.chunk_of(p);
    let rev = m.things_rev(chunk);
    m.see_around(p);
    for q in [p, p.offset(-1, -1), p.offset(1, 1)] {
        assert!(m.seen(m.idx(q)), "{q:?} is seen");
    }
    assert!(!m.seen(m.idx(p.offset(2, 0))), "two cells away is not");
    assert!(m.things_rev(chunk) > rev, "rock seen for the first time draws anew");
    let rev = m.things_rev(chunk);
    m.see_around(p);
    assert_eq!(m.things_rev(chunk), rev, "seeing it again changes nothing");
}

#[test]
fn a_colonist_below_sees_where_they_go_and_a_save_keeps_it() {
    let mut s = Sim::build(&common::mods(), 3, &|m| m == "core", 64).expect("core loads");
    let c = s.world.colony_center().unwrap();
    // A corridor dug out of the level below, and a colonist at one end.
    let start = IVec::at(c.x - 10, c.y, -1);
    for x in 0..20 {
        let p = start.offset(x, 0);
        let leaves = s.world.solid_at(p).and_then(|r| r.leaves_r).expect("workable rock below");
        s.world.map.set_terrain(p, leaves, s.world.defs.terrain[leaves as usize].path_cost);
    }
    let human = s.world.defs.start.as_ref().unwrap().creature_r;
    let e = s.world.spawn_pawn(human, Faction::Player, start, None);
    let far = start.offset(19, 0);
    assert!(!s.world.map.seen(s.world.map.idx(far)), "the far end is unseen");
    let mut went = 0;
    for _ in 0..20_000 {
        s.step();
        went = went.max(s.world.pawn_pos(e).map_or(0, |p| p.x - start.x));
        if went >= 10 {
            break;
        }
    }
    assert!(went >= 10, "the colonist moved along the corridor");
    let beyond = start.offset(went, 1);
    assert!(s.world.map.seen(s.world.map.idx(beyond)), "the rock beside the way they went is seen");
    let back = Snapshot::capture(&s).restore(&common::mods(), &|m| m == "core").unwrap();
    assert_eq!(back.world.map.seen_bits(), s.world.map.seen_bits(), "a save keeps what was seen");
}
