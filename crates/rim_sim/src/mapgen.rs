//! Data-driven map generation: terrain comes from `[terrain.gen]` bands,
//! the levels below from `[[stratum]]`, fixtures from `[thing.spawn]`,
//! wildlife from `[creature.spawn]`.

use crate::defs::{DefDb, DefId};
use crate::rng::{hash2_f, hash_str, mix};
use crate::world::{Faction, World};
use crate::IVec;

/// Generates terrain, plants, rocks and wildlife. Returns the start cell.
pub fn generate(w: &mut World) -> IVec {
    let defs = w.defs.clone();
    let seed = w.seed;
    let (mw, mh) = (w.map.w, w.map.h);

    let mut bands: Vec<(usize, &crate::defs::TerrainGen)> =
        defs.terrain.iter().enumerate().filter_map(|(i, t)| t.gen.as_ref().map(|g| (i, g))).collect();
    bands.sort_by_key(|(i, g)| (-g.priority, *i));

    for y in 0..mh {
        for x in 0..mw {
            let e = fbm(x as f64 / 48.0, y as f64 / 48.0, seed ^ 0xE1E7, 5);
            let m = fbm(x as f64 / 36.0, y as f64 / 36.0, seed ^ 0x3015, 4);
            let t = bands
                .iter()
                .find(|(_, g)| {
                    (g.elevation[0]..=g.elevation[1]).contains(&e) && (g.moisture[0]..=g.moisture[1]).contains(&m)
                })
                .map_or(0, |(i, _)| *i);
            w.map.set_terrain(IVec::new(x, y), t as DefId, defs.terrain[t].path_cost);
        }
    }
    strata(w, &defs, seed);

    // Each spawning def's pattern is keyed by its id, not its place in the
    // list: a mod adding a def elsewhere must not move every tree.
    let salts: Vec<u64> = defs.things.iter().map(|d| mix(hash_str(&d.id))).collect();
    for y in 0..mh {
        for x in 0..mw {
            let p = IVec::new(x, y);
            let terrain = w.map.terrain[w.map.idx(p)];
            for (di, td) in defs.things.iter().enumerate() {
                let Some(s) = &td.spawn else { continue };
                if s.terrain_r.contains(&terrain) && hash2_f(x as i64, y as i64, seed ^ salts[di]) < s.density {
                    w.spawn_fixture(di as DefId, p, false);
                    break;
                }
            }
        }
    }

    w.map.ensure_regions();
    let start = find_start(w);

    for (ci, cd) in defs.creatures.iter().enumerate() {
        let Some(s) = &cd.spawn else { continue };
        for _ in 0..s.groups {
            for _ in 0..200 {
                let p = IVec::new(w.rng.below(mw as u32) as i32, w.rng.below(mh as u32) as i32);
                let i = w.map.idx(p);
                if !w.map.passable(p) || !s.terrain_r.contains(&w.map.terrain[i]) || p.chebyshev(start) < 30 {
                    continue;
                }
                let n = w.rng.range(s.group[0] as i32, s.group[1] as i32);
                for _ in 0..n {
                    let q = p.offset(w.rng.range(-2, 2), w.rng.range(-2, 2));
                    if w.map.passable(q) {
                        w.spawn_pawn(ci as DefId, Faction::Wild, q, None);
                    }
                }
                break;
            }
        }
    }
    start
}

/// Open ground near the middle, in the map's largest connected area.
fn find_start(w: &World) -> IVec {
    let c = IVec::new(w.map.w / 2, w.map.h / 2);
    let need = (w.map.w * w.map.h) as usize / 4;
    for r in 0..w.map.w.max(w.map.h) / 2 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let p = c.offset(dx, dy);
                if w.map.passable(p)
                    && w.map.cost(p) <= 100
                    && w.map.fixture_at(p).is_none()
                    && w.map.region_size(p) >= need
                {
                    return p;
                }
            }
        }
    }
    c
}

/// Fill each level below the surface from its stratum (DESIGN.md §6d):
/// patches of its fills, each fill only under the surface terrains it names,
/// and a ring of its edge rock. A level the map has without a stratum stays
/// impassable.
fn strata(w: &mut World, defs: &DefDb, seed: u64) {
    let (mw, mh) = (w.map.w, w.map.h);
    let mut fits: Vec<(DefId, f64)> = Vec::new();
    for z in w.map.levels().filter(|&z| z < 0) {
        let Some(st) = defs.stratum(z) else { continue };
        let salt = seed ^ mix(0x5747_A000 + z.unsigned_abs() as u64);
        for y in 0..mh {
            for x in 0..mw {
                let t = if x == 0 || y == 0 || x == mw - 1 || y == mh - 1 {
                    st.edge_r
                } else {
                    let above = w.map.terrain[w.map.idx(IVec::new(x, y))];
                    fits.clear();
                    fits.extend(
                        st.fill
                            .iter()
                            .filter(|f| f.under_r.is_empty() || f.under_r.contains(&above))
                            .map(|f| (f.terrain_r, f.weight)),
                    );
                    let total: f64 = fits.iter().map(|f| f.1).sum();
                    let mut left = fbm(x as f64 / st.patch, y as f64 / st.patch, salt, 3) * total;
                    let fallback = fits.last().map_or(st.fill[0].terrain_r, |f| f.0);
                    fits.iter()
                        .find(|f| {
                            left -= f.1;
                            left < 0.0
                        })
                        .map_or(fallback, |f| f.0)
                };
                w.map.set_terrain(IVec::at(x, y, z), t, defs.terrain[t as usize].path_cost);
            }
        }
    }
}

/// Lay each `[[vein]]` into its stock field (DESIGN.md §6d): from about
/// `chance` of the cells of its terrains, a wandering blob of `size` cells,
/// each holding an `amount`. Every choice hashes the seed, the vein's id and
/// the cell, so the same seed lays the same veins on every machine and the
/// world's RNG is left alone.
pub fn veins(w: &mut World) {
    let defs = w.defs.clone();
    let unit = |h: u64| (h >> 11) as f64 / (1u64 << 53) as f64;
    for v in &defs.veins {
        let salt = mix(w.seed ^ hash_str(&v.id));
        for start in 0..w.map.cells() {
            let h = mix(salt ^ start as u64);
            if !v.within_r.contains(&w.map.terrain[start]) || unit(h) >= v.chance {
                continue;
            }
            let size = v.size[0] + (mix(h ^ 1) % (v.size[1] - v.size[0] + 1) as u64) as u32;
            let mut at = w.map.pos(start);
            for k in 0..size as u64 {
                let i = w.map.idx(at);
                let step = mix(h ^ (k << 8) ^ 2);
                if v.within_r.contains(&w.map.terrain[i]) {
                    // Whole units, so a vein is spent exactly.
                    let amount = (v.amount[0] + (v.amount[1] - v.amount[0]) * unit(step)).round();
                    w.fields.set_stock(&defs, &w.map, v.field_r, at, amount, false);
                }
                let (dx, dy) = crate::map::NEIGHBORS8[(step % 8) as usize];
                let next = at.offset(dx, dy);
                if w.map.inb(next) {
                    at = next;
                }
            }
        }
    }
}

/// Fractal noise in [0, 1] at a cell, for scripts that generate a level
/// (`rim.noise`): the same numbers on every machine.
pub fn noise(x: f64, y: f64, seed: u64) -> f64 {
    fbm(x, y, seed, 3)
}

fn value_noise(x: f64, y: f64, seed: u64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let (xi, yi) = (xi as i64, yi as i64);
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(xf), s(yf));
    let a = hash2_f(xi, yi, seed);
    let b = hash2_f(xi + 1, yi, seed);
    let c = hash2_f(xi, yi + 1, seed);
    let d = hash2_f(xi + 1, yi + 1, seed);
    (a + (b - a) * u) + ((c + (d - c) * u) - (a + (b - a) * u)) * v
}

/// Fractal noise normalized to roughly [0, 1].
fn fbm(x: f64, y: f64, seed: u64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += value_noise(x * freq, y * freq, mix(seed + o as u64)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    // Stretch contrast: sums of octaves cluster around 0.5.
    ((sum / norm - 0.5) * 1.8 + 0.5).clamp(0.0, 1.0)
}
