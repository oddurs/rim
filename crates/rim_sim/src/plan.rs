//! A stretch of the colony written back out as a house plan (DESIGN.md
//! §6c): the `[[plan]]` text a mod ships, so what was built can be placed
//! again, and read, diffed and shared as text.

use crate::defs::DefId;
use crate::world::World;
use crate::IVec;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The pieces in the rectangle `a`..`b` as a `[[plan]]` def with this id
/// and label: every built or planned fixture whose footprint lies inside
/// it. Natural things and floors are left out. Each (thing, material,
/// facing) gets a character, the first letter of its name where that's
/// free, and a piece bigger than a cell is written over its whole
/// footprint so the grid reads as the house does.
pub fn plan_text(w: &World, a: IVec, b: IVec, id: &str, label: &str) -> String {
    // One level: `a`'s. A plan is a single storey.
    let (lo, hi) = (IVec::at(a.x.min(b.x), a.y.min(b.y), a.z), IVec::at(a.x.max(b.x), a.y.max(b.y), a.z));
    let (wd, ht) = ((hi.x - lo.x + 1) as usize, (hi.y - lo.y + 1) as usize);
    let mut grid = vec![vec!['.'; wd]; ht];
    let mut keys: BTreeMap<(DefId, Option<DefId>, u8), char> = BTreeMap::new();
    let mut taken: Vec<char> = vec!['.', ' '];
    let inside = |p: IVec| p.z == lo.z && p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let p = IVec::at(x, y, lo.z);
            let Some(e) = w.map.fixture_at(p) else { continue };
            let Some(t) = w.thing(e).filter(|t| t.pos == p) else { continue };
            let td = w.defs.thing(t.def);
            if td.natural || td.build.is_none() {
                continue;
            }
            let cells: Vec<IVec> = td.footprint(t.pos, t.facing).collect();
            if !cells.iter().all(|&c| inside(c)) {
                continue;
            }
            let key = (t.def, w.made_of(e), t.facing);
            let c = *keys.entry(key).or_insert_with(|| {
                let first = td.label.chars().next().map(|c| c.to_ascii_lowercase());
                let pool = first.into_iter().chain("#abcdefghijklmnopqrstuvwxyz0123456789".chars());
                let c = pool.filter(|c| c.is_ascii_graphic()).find(|c| !taken.contains(c)).unwrap_or('?');
                taken.push(c);
                c
            });
            for cell in cells {
                grid[(cell.y - lo.y) as usize][(cell.x - lo.x) as usize] = c;
            }
        }
    }
    let mut out = String::new();
    let _ = writeln!(out, "[[plan]]\nid = {id:?}\nlabel = {label:?}\ngrid = \"\"\"");
    for row in &grid {
        let _ = writeln!(out, "{}", row.iter().collect::<String>());
    }
    let _ = writeln!(out, "\"\"\"");
    let mut legend: Vec<(char, String)> = keys
        .iter()
        .map(|(&(def, stuff, facing), &c)| {
            let mut e = format!("thing = {:?}", w.defs.thing(def).id);
            if let Some(m) = stuff {
                let _ = write!(e, ", stuff = {:?}", w.defs.thing(m).id);
            }
            if facing != 0 {
                let _ = write!(e, ", facing = {facing}");
            }
            (c, e)
        })
        .collect();
    legend.sort();
    let _ = writeln!(out, "[plan.legend]");
    for (c, e) in legend {
        let _ = writeln!(out, "{:?} = {{ {e} }}", c.to_string());
    }
    out
}
