//! Chunk revisions: every change to what a cell draws bumps its chunk.
//!
//! Renderers cache per chunk and redraw only when the revision moves, so a
//! change that forgets to bump one is a stale picture. This plays a busy
//! colony and, every tick, compares a fingerprint of each chunk's drawn
//! state with the revision.

mod common;

use rim_sim::map::CHUNK;
use rim_sim::world::{Blueprint, Designated, Faction, MadeOf, Regrow, Thing, World};
use rim_sim::{Command, IVec, Sim};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// What a renderer reads from each cell of each chunk. A blueprint's
/// progress is left out on purpose: it changes every tick of work, and
/// renderers draw plans each frame rather than cache them (map.rs).
fn fingerprints(w: &World) -> Vec<u64> {
    let (cw, ch) = w.map.chunks();
    let mut out = vec![0; (cw * ch) as usize];
    for (c, fp) in out.iter_mut().enumerate() {
        let (ox, oy) = ((c as i32 % cw) * CHUNK, (c as i32 / cw) * CHUNK);
        let mut h = DefaultHasher::new();
        for y in oy..(oy + CHUNK).min(w.map.h) {
            for x in ox..(ox + CHUNK).min(w.map.w) {
                let p = IVec::new(x, y);
                w.map.terrain[w.map.idx(p)].hash(&mut h);
                for e in [w.map.floor_at(p), w.map.item_at(p), w.map.fixture_at(p)] {
                    let Some(e) = e else {
                        0u8.hash(&mut h);
                        continue;
                    };
                    e.to_bits().hash(&mut h);
                    if let Ok(t) = w.ecs.get::<&Thing>(e) {
                        (t.def, t.count).hash(&mut h);
                    }
                    w.ecs.get::<&Designated>(e).map(|d| d.0).ok().hash(&mut h);
                    w.ecs.get::<&MadeOf>(e).map(|m| m.0).ok().hash(&mut h);
                    (w.ecs.get::<&Regrow>(e).is_ok(), w.ecs.get::<&Blueprint>(e).is_ok()).hash(&mut h);
                }
            }
        }
        *fp = h.finish();
    }
    out
}

fn revs(w: &World) -> Vec<(u64, u64)> {
    let (cw, ch) = w.map.chunks();
    (0..(cw * ch) as usize).map(|c| (w.map.things_rev(c), w.map.terrain_rev(c))).collect()
}

#[test]
fn every_drawn_change_bumps_its_chunk() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let defs = s.world.defs.clone();
    let c = s.world.colony_center().unwrap();
    let human = defs.start.as_ref().unwrap().creature_r;
    for k in 0..10 {
        let p = (0..20).map(|r| c.offset(k - 5, r - 10)).find(|&p| s.world.map.passable(p)).unwrap_or(c);
        s.world.spawn_pawn(human, Faction::Player, p, None);
    }
    let (a, b) = (c.offset(-30, -30), c.offset(30, 30));
    for d in 0..defs.designations.len() {
        s.push(Command::Designate { designation: d as _, a, b });
    }
    let wood = defs.thing_id("wood");
    for (id, dx) in [("wall", 3), ("floor", 5), ("bed", 7), ("campfire", 9)] {
        let thing = defs.thing_id(id).unwrap();
        s.push(Command::Build { thing, stuff: wood, a: c.offset(dx, -4), b: c.offset(dx, 4) });
    }
    let (mut fp, mut rv) = (fingerprints(&s.world), revs(&s.world));
    let mut changes = 0;
    for tick in 0..8000 {
        s.step();
        let (fp2, rv2) = (fingerprints(&s.world), revs(&s.world));
        for i in 0..fp.len() {
            if fp[i] != fp2[i] {
                changes += 1;
                assert_ne!(rv[i], rv2[i], "tick {tick}: chunk {i} changed but its revision didn't");
            }
        }
        (fp, rv) = (fp2, rv2);
    }
    assert!(changes > 50, "the colony should have been busy ({changes} chunk changes)");
}

#[test]
fn a_touch_at_a_chunk_border_bumps_the_neighbour() {
    let mut s = Sim::new(&common::mods(), 3).unwrap();
    let m = &mut s.world.map;
    let (left, right) = (m.chunk_of(IVec::new(CHUNK - 1, 5)), m.chunk_of(IVec::new(CHUNK, 5)));
    let (l0, r0) = (m.things_rev(left), m.things_rev(right));
    m.touch(IVec::new(CHUNK - 1, 5));
    assert_eq!((m.things_rev(left), m.things_rev(right)), (l0 + 1, r0 + 1));
    let far = m.chunk_of(IVec::new(CHUNK * 2 + 5, 5));
    let f0 = m.things_rev(far);
    m.touch(IVec::new(CHUNK + 5, 5));
    assert_eq!(m.things_rev(far), f0, "an inner cell doesn't reach past its own chunk");
}
