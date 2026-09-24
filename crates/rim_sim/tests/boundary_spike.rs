//! 0211: what does deriving room properties from the boundary cost?
//!
//! Run with: cargo test -p rim_sim --test boundary_spike -- --nocapture

use rim_sim::{IVec, Sim};
use std::path::Path;
use std::time::Instant;

fn sim() -> Sim {
    Sim::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods"), 21).expect("mods load")
}

/// Fill a square of the map with `n` x `n` huts, each 5x5 with a door.
/// This is the worst case the spike cares about: many small rooms, so the
/// boundary is a large fraction of the cells.
fn build_huts(s: &mut Sim, origin: IVec, across: i32) -> usize {
    let wall = s.world.defs.thing_id("wall_wood").unwrap();
    let mut rooms = 0;
    for hy in 0..across {
        for hx in 0..across {
            let o = origin.offset(hx * 6, hy * 6);
            let mut ok = true;
            for y in 0..5 {
                for x in 0..5 {
                    let p = o.offset(x, y);
                    if !s.world.map.inb(p) {
                        ok = false;
                    }
                }
            }
            if !ok {
                continue;
            }
            for y in 0..5 {
                for x in 0..5 {
                    let p = o.offset(x, y);
                    if !(x == 0 || y == 0 || x == 4 || y == 4) {
                        continue;
                    }
                    if let Some(f) = s.world.map.fixture_at(p) {
                        s.world.despawn_thing(f);
                    }
                    s.world.spawn_fixture(wall, p, false);
                }
            }
            rooms += 1;
        }
    }
    rooms
}

/// One pass over the map's wall cells, adding each wall's contribution to
/// every distinct room it touches. This is the shape the real thing would
/// take: walls are sparse, and each is visited once.
fn boundary_pass(s: &Sim) -> (usize, f64) {
    let m = &s.world.map;
    let mut touched = 0usize;
    let mut total = 0.0f64;
    let mut acc = vec![0.0f64; 4096];
    for i in 0..(m.w * m.h) as usize {
        if !m.blocks_fields(i) {
            continue;
        }
        touched += 1;
        let p = m.pos(i);
        // Each distinct adjacent room gets this wall's contribution once.
        let mut seen = [0u32; 4];
        let mut n = 0;
        for (dx, dy) in &rim_sim::map::NEIGHBORS8[..4] {
            let q = p.offset(*dx, *dy);
            let Some(r) = m.room_at(q) else { continue };
            if seen[..n].contains(&r.id) {
                continue;
            }
            seen[n] = r.id;
            n += 1;
            let slot = (r.id as usize) % acc.len();
            acc[slot] += 0.06;
            total += 0.06;
        }
    }
    (touched, total)
}

#[test]
fn boundary_cost() {
    let c = sim().world.colony_center().unwrap_or(IVec::new(100, 100));
    let origin = IVec::new((c.x - 60).max(1), (c.y - 60).max(1));

    for across in [4, 10, 20] {
        let mut s2 = sim();
        let rooms = build_huts(&mut s2, origin, across);
        s2.world.map.ensure_rooms();

        // Rebuild cost as it stands: force dirty by toggling one wall.
        // Alternate placing and removing a wall so rooms are genuinely
        // dirty on every iteration.
        let wall = s2.world.defs.thing_id("wall_wood").unwrap();
        let probe = origin.offset(1, 1);
        let mut rebuild = f64::MAX;
        for k in 0..6 {
            if k % 2 == 0 {
                s2.world.spawn_fixture(wall, probe, false);
            } else if let Some(f) = s2.world.map.fixture_at(probe) {
                s2.world.despawn_thing(f);
            }
            let t = Instant::now();
            s2.world.map.ensure_rooms();
            rebuild = rebuild.min(t.elapsed().as_secs_f64() * 1e3);
        }

        let mut bpass = f64::MAX;
        let mut walls = 0;
        for _ in 0..5 {
            let t = Instant::now();
            let (w, _) = boundary_pass(&s2);
            bpass = bpass.min(t.elapsed().as_secs_f64() * 1e3);
            walls = w;
        }

        println!(
            "huts={rooms:<4} walls={walls:<6} room rebuild {rebuild:>6.3} ms   boundary pass {bpass:>6.3} ms   \
             (+{:.0}%)",
            bpass / rebuild * 100.0
        );
    }
}
