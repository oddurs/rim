//! Wind shelter: the lee of walls, rock and trees. A wall on the windward
//! side keeps the wind off even without a roof, so where you build answers
//! the weather. Worked out only when the wind turns into another octant or
//! something that blocks it changes, never per tick.

use crate::defs::FieldKind;
use crate::map::Map;
use crate::world::{Blueprint, World};
use crate::IVec;

/// The step downwind for each octant of a direction the wind blows toward:
/// 0 east, then clockwise on screen (y grows down), 45° apart.
const DOWNWIND: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// The octant a direction in degrees falls in.
pub fn octant(degrees: f64) -> u8 {
    ((degrees / 45.0).round() as i64).rem_euclid(8) as u8
}

/// Each cell's exposure in percent, from each cell's blocking in percent:
/// a blocker shelters `lee` cells downwind of it, fading with distance, and
/// itself. Where lees overlap the stronger one counts.
pub fn exposure(map: &Map, blocks: &[u8], octant: u8, lee: u32) -> Vec<u8> {
    let (dx, dy) = DOWNWIND[octant as usize % 8];
    let mut shelter = vec![0u8; blocks.len()];
    let reach = lee as i32 + 1;
    for (i, &b) in blocks.iter().enumerate().filter(|(_, &b)| b > 0) {
        shelter[i] = shelter[i].max(b);
        let p = map.pos(i);
        // Inside a wall or a wood, the next blocker downwind casts a lee at
        // least as long and strong: this one's is covered.
        let next = p.offset(dx, dy);
        if map.inb(next) && blocks[map.idx(next)] >= b {
            continue;
        }
        for k in 1..reach {
            let q = IVec::new(p.x + dx * k, p.y + dy * k);
            if !map.inb(q) {
                break;
            }
            let s = (b as i32 * (reach - k) / reach) as u8;
            let j = map.idx(q);
            shelter[j] = shelter[j].max(s);
        }
    }
    shelter.into_iter().map(|s| 100 - s.min(100)).collect()
}

impl World {
    /// Bring every shelter field up to date with the wind and the map.
    pub fn update_shelter(&mut self) {
        let defs = self.defs.clone();
        for (f, fd) in defs.fields.iter().enumerate() {
            if fd.kind != FieldKind::Shelter {
                continue;
            }
            let key = (octant(self.fields.ambient(fd.from_r)), self.map.revision);
            if self.fields.layers[f].exposure_for == Some(key) {
                continue;
            }
            let mut blocks = vec![0u8; self.map.fixture.len()];
            for (i, fx) in self.map.fixture.iter().enumerate() {
                let Some(e) = fx else { continue };
                if self.ecs.get::<&Blueprint>(*e).is_ok() {
                    continue;
                }
                if let Some(t) = self.thing(*e) {
                    blocks[i] = (defs.thing(t.def).blocks_wind.clamp(0.0, 1.0) * 100.0).round() as u8;
                }
            }
            let layer = &mut self.fields.layers[f];
            layer.exposure = exposure(&self.map, &blocks, key.0, fd.lee);
            layer.exposure_for = Some(key);
            self.shelter_recomputes += 1;
        }
    }
}
