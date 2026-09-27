//! What stops light, one texel per cell (DESIGN.md §6e).
//!
//! Every lighting pass marches through this texture, so each texel says:
//!
//! - R: how tall the cell stands, in the high six bits (`MAX_HEIGHT` / 63
//!   a step), and in the low two what kind of mass it is: a window or a
//!   door. Kinds can't be blended, so passes that read R sample cell centres.
//! - G: under a roof (255) or not (0). One bit, so the linear filter blends it
//!   into a soft edge at a wall and never into a kind that isn't there.
//! - B: how much of the sky it stops.
//! - A: how much firelight it stops.
//!
//! A chunk is repacked when a fixture or the terrain in it changes, or when
//! a room rebuild roofed or unroofed one of its cells, and uploaded only if
//! a texel changed. Hauling and designations don't wake it. "Roofed" is the
//! sim's indoors: an enclosed room, every cell within a support's span
//! (DESIGN.md §6c), so the light drawn agrees with the light the sim keeps.

use macroquad::prelude::*;
use rim_sim::defs::{Category, DefId};
use rim_sim::map::CHUNK;
use rim_sim::world::{Blueprint, Thing, World};

/// The tallest height a texel holds, in cells.
pub const MAX_HEIGHT: f64 = 4.0;
/// How tall a blocking thing stands when its def doesn't say, and how high
/// a roof sits: one storey, one cell.
pub const STOREY: f64 = 1.0;
/// How much of the sky a canopy stops: the rest comes through the leaves.
const CANOPY_SKY: f64 = 0.6;
/// How much firelight a shut door stops: a little leaks round its edges.
const DOOR_FIRE: f64 = 0.7;

/// R's low bits: a wall with a pane in it.
pub const WINDOW: u8 = 1;
/// R's low bits: a door.
pub const DOOR: u8 = 2;

/// What stands in a cell, as light sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Occluder {
    Open,
    /// A wall, rock, or anything else that blocks: solid to the top.
    Solid {
        height: f64,
    },
    /// A wall that lets `pass` of the outdoor light through.
    Window {
        height: f64,
        pass: f64,
    },
    Door {
        height: f64,
    },
    /// Tall but not blocking: a tree's canopy, which the sky partly gets through.
    Canopy {
        height: f64,
    },
}

/// R: a height and a kind of mass.
fn height_and(height: f64, kind: u8) -> u8 {
    ((height.clamp(0.0, MAX_HEIGHT) / MAX_HEIGHT * 63.0).round() as u8) << 2 | kind
}

/// One cell's texel.
pub fn texel(o: Occluder, roofed: bool) -> [u8; 4] {
    if roofed {
        // The roof is what the sky meets; what stands under it is indoors,
        // lit by fires, and stops none of their light.
        return [height_and(STOREY, 0), 255, 255, 0];
    }
    let u = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    match o {
        Occluder::Open => [0, 0, 0, 0],
        Occluder::Solid { height } => [height_and(height, 0), 0, 255, 255],
        Occluder::Window { height, pass } => [height_and(height, WINDOW), 0, 255, u(1.0 - pass)],
        Occluder::Door { height } => [height_and(height, DOOR), 0, 255, u(DOOR_FIRE)],
        Occluder::Canopy { height } => [height_and(height, 0), 0, u(CANOPY_SKY), 0],
    }
}

/// What stands in cell `i`: its fixture, or the rock it is. `light` is the
/// light field, whose boundary `pass` makes a wall a window.
pub fn occluder_at(w: &World, i: usize, light: Option<DefId>) -> Occluder {
    let p = w.map.pos(i);
    // The fixture's def, or the rock's thing's; read in place, since this
    // runs for every cell of a repacked chunk.
    let def = match w.map.fixture_at(p) {
        // A plan casts no shadow until it's built.
        Some(e) if w.ecs.get::<&Blueprint>(e).is_ok() => return Occluder::Open,
        Some(e) => w.ecs.get::<&Thing>(e).map(|t| t.def).ok(),
        None => match w.solid_at(p) {
            // Bedrock names no thing (it can't be worked), and it still
            // fills its cell.
            Some(s) if s.thing_r.is_none() => return Occluder::Solid { height: STOREY },
            Some(s) => s.thing_r,
            None => None,
        },
    };
    let Some(def) = def else { return Occluder::Open };
    let d = &w.defs.things[def as usize];
    if d.door {
        return Occluder::Door { height: d.height.unwrap_or(STOREY) };
    }
    // Rock blocks whatever its thing says: it fills its cell.
    if d.blocks || w.solid_at(p).is_some() {
        let height = d.height.unwrap_or(STOREY);
        let pass = d.boundary.iter().filter(|b| Some(b.field_r) == light).map(|b| b.pass).fold(0.0, f64::max);
        return if pass > 0.0 { Occluder::Window { height, pass } } else { Occluder::Solid { height } };
    }
    match d.height {
        Some(height) if height > 0.0 && d.category == Category::Plant => Occluder::Canopy { height },
        _ => Occluder::Open,
    }
}

/// The occluder texture, kept in step with the map.
#[derive(Default)]
pub struct Occluders {
    bytes: Vec<u8>,
    /// Per chunk, the (terrain, fixture) revisions it was packed at.
    seen: Vec<(u64, u64)>,
    /// The room rebuild it was last packed for.
    rooms: u64,
    size: (i32, i32),
    /// Linear, so a pass sampling between cells gets a soft edge.
    pub texture: Option<Texture2D>,
    /// Bumped whenever the texture changes, so passes that read it can cache.
    pub version: u64,
}

/// A chunk that must be packed again whatever its revisions say.
const STALE: (u64, u64) = (u64::MAX, u64::MAX);

impl Occluders {
    /// Bring the texture up to date with `w`. Whether it changed.
    pub fn update(&mut self, w: &World) -> bool {
        let (whole, dirty) = self.pack(w);
        if dirty.is_empty() && self.texture.is_some() {
            return false;
        }
        self.version += 1;
        let m = &w.map;
        match &self.texture {
            Some(t) if !whole => {
                for (x, y, cw, ch) in dirty {
                    let mut part = Vec::with_capacity((cw * ch * 4) as usize);
                    for row in y..y + ch {
                        let a = ((row * m.w + x) * 4) as usize;
                        part.extend_from_slice(&self.bytes[a..a + (cw * 4) as usize]);
                    }
                    t.update_part(&Image { bytes: part, width: cw as u16, height: ch as u16 }, x, y, cw, ch);
                }
            }
            Some(t) if t.width() as i32 == m.w && t.height() as i32 == m.h => {
                t.update(&Image { bytes: self.bytes.clone(), width: m.w as u16, height: m.h as u16 });
            }
            _ => {
                let img = Image { bytes: self.bytes.clone(), width: m.w as u16, height: m.h as u16 };
                let t = Texture2D::from_image(&img);
                t.set_filter(FilterMode::Linear);
                self.texture = Some(t);
            }
        }
        true
    }

    /// Repack everything next update.
    pub fn invalidate(&mut self) {
        self.bytes.clear();
    }

    /// Repack what changed: every chunk whose fixtures or terrain changed or
    /// whose roof did, or the whole map when nothing is packed yet. Whether
    /// it was whole, and the rectangles whose texels changed, as (x, y, w, h)
    /// in cells.
    fn pack(&mut self, w: &World) -> (bool, Vec<(i32, i32, i32, i32)>) {
        let m = &w.map;
        let (cw, ch) = m.chunks();
        let chunks = (cw * ch) as usize;
        let whole = self.bytes.is_empty() || self.size != (m.w, m.h);
        if whole {
            self.bytes = vec![0; (m.w * m.h * 4) as usize];
            self.seen = vec![STALE; chunks];
            self.size = (m.w, m.h);
        } else if self.rooms != m.room_rebuilds {
            // A room rebuild can roof or unroof any cell: repack the chunks
            // where one did, found by the first cell that disagrees.
            for c in 0..chunks {
                if self.seen[c] == STALE {
                    continue;
                }
                let o = m.chunk_origin(c);
                let roof_moved = (o.y..(o.y + CHUNK).min(m.h)).any(|y| {
                    (o.x..(o.x + CHUNK).min(m.w)).any(|x| {
                        let i = (y * m.w + x) as usize;
                        (self.bytes[i * 4 + 1] == 255) != m.indoors(rim_sim::IVec::new(x, y))
                    })
                });
                if roof_moved {
                    self.seen[c] = STALE;
                }
            }
        }
        self.rooms = m.room_rebuilds;
        let light = w.defs.lookup("field", "light");
        let mut dirty = Vec::new();
        for c in 0..chunks {
            let rev = (m.terrain_rev(c), m.fixture_rev(c));
            if self.seen[c] == rev {
                continue;
            }
            self.seen[c] = rev;
            let o = m.chunk_origin(c);
            let (x1, y1) = ((o.x + CHUNK).min(m.w), (o.y + CHUNK).min(m.h));
            let mut changed = whole;
            for y in o.y..y1 {
                for x in o.x..x1 {
                    let i = (y * m.w + x) as usize;
                    let t = texel(occluder_at(w, i, light), m.indoors(rim_sim::IVec::new(x, y)));
                    if self.bytes[i * 4..i * 4 + 4] != t {
                        self.bytes[i * 4..i * 4 + 4].copy_from_slice(&t);
                        changed = true;
                    }
                }
            }
            if changed {
                dirty.push((o.x, o.y, x1 - o.x, y1 - o.y));
            }
        }
        (whole, dirty)
    }

    /// The packed texel of cell `i`, as uploaded.
    #[cfg(test)]
    fn at(&self, i: usize) -> [u8; 4] {
        self.bytes[i * 4..i * 4 + 4].try_into().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_sim::IVec;

    #[test]
    fn each_kind_of_cell_packs_as_the_passes_expect() {
        let wall = texel(Occluder::Solid { height: 1.0 }, false);
        assert_eq!(wall, [16 << 2, 0, 255, 255], "a storey high, stops sky and fire");
        let window = texel(Occluder::Window { height: 1.0, pass: 0.35 }, false);
        assert_eq!(window, [16 << 2 | WINDOW, 0, 255, 166], "a window; firelight gets through the pane");
        let door = texel(Occluder::Door { height: 1.0 }, false);
        assert_eq!(door, [16 << 2 | DOOR, 0, 255, 179]);
        let tree = texel(Occluder::Canopy { height: 2.0 }, false);
        assert_eq!(tree, [32 << 2, 0, 153, 0], "the sky partly gets through; fire isn't stopped");
        assert_eq!(texel(Occluder::Open, false), [0; 4]);
        assert_eq!(texel(Occluder::Canopy { height: 2.0 }, true), [16 << 2, 255, 255, 0], "a roof wins");
        assert_eq!(texel(Occluder::Solid { height: 9.0 }, false)[0], 63 << 2, "clamped to what a texel holds");
    }

    fn sim() -> rim_sim::Sim {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        rim_sim::Sim::new(&mods, 3).expect("mods load")
    }

    /// The first `n` by `rows` open cells, away from the map's edge.
    fn open_block(w: &World, n: i32, rows: i32) -> IVec {
        let m = &w.map;
        let free =
            |p: IVec| m.passable(p) && m.fixture_at(p).is_none() && m.item_at(p).is_none() && w.solid_at(p).is_none();
        for y in 4..m.h - rows - 4 {
            for x in 4..m.w - n - 4 {
                let p = IVec::new(x, y);
                if (0..rows).all(|dy| (0..n).all(|dx| free(p.offset(dx, dy)))) {
                    return p;
                }
            }
        }
        panic!("no open {n}×{rows}");
    }

    /// Put `thing` at `p`, built of whatever it can be made of.
    fn place(w: &mut World, thing: &str, p: IVec, planned: bool) {
        let d = w.defs.lookup("thing", thing).unwrap_or_else(|| panic!("no {thing}"));
        let stuff = w.defs.things[d as usize]
            .build
            .as_ref()
            .and_then(|b| b.stuff.as_ref())
            .and_then(|sc| w.defs.materials(&sc.category).first().copied());
        w.spawn_fixture_of(d, p, planned, stuff).unwrap_or_else(|| panic!("{thing} at {p:?}"));
    }

    #[test]
    fn walls_windows_doors_and_trees_pack_from_their_defs() {
        let mut s = sim();
        let w = &mut s.world;
        let p = open_block(w, 5, 1);
        for (k, thing) in ["wall", "window", "door", "tree_oak"].into_iter().enumerate() {
            place(w, thing, p.offset(k as i32, 0), false);
        }
        place(w, "wall", p.offset(4, 0), true);
        let mut o = Occluders::default();
        o.pack(w);
        let at = |k: i32| o.at(w.map.idx(p.offset(k, 0)));
        assert_eq!(at(0), texel(Occluder::Solid { height: STOREY }, false), "a wall");
        let pass = w.defs.things[w.defs.lookup("thing", "window").unwrap() as usize].boundary[0].pass;
        assert_eq!(at(1), texel(Occluder::Window { height: STOREY, pass }, false), "a window lets its pass through");
        assert_eq!(at(2), texel(Occluder::Door { height: STOREY }, false), "a door");
        assert_eq!(at(3), texel(Occluder::Canopy { height: 2.0 }, false), "an oak, as tall as core says");
        assert_eq!(at(4), [0; 4], "a planned wall casts nothing yet");
    }

    #[test]
    fn only_what_stands_in_a_cell_wakes_it_not_hauling() {
        let mut s = sim();
        let mut o = Occluders::default();
        let (whole, first) = o.pack(&s.world);
        assert!(whole && !first.is_empty());
        let (_, again) = o.pack(&s.world);
        assert!(again.is_empty(), "nothing changed, nothing repacked: {again:?}");
        let p = open_block(&s.world, 1, 1);
        place(&mut s.world, "tree_oak", p, false);
        let (whole, dirty) = o.pack(&s.world);
        assert!(!whole, "a tree doesn't rebuild rooms");
        assert_eq!(dirty.len(), 1, "only the tree's chunk changed");
        assert_eq!(o.at(s.world.map.idx(p)), texel(Occluder::Canopy { height: 2.0 }, false));
        // An item set down stops no light, and doesn't even wake its chunk.
        let before = o.seen.clone();
        let q = open_block(&s.world, 1, 1);
        let stone = s.world.defs.things.iter().position(|d| d.category == Category::Item).unwrap();
        s.world.place_item(stone as DefId, q, 1);
        let (_, dirty) = o.pack(&s.world);
        assert!(dirty.is_empty() && o.seen == before, "hauling repacks nothing");
    }

    #[test]
    fn closing_a_room_roofs_its_floor_and_repacks_only_where_it_is() {
        let mut s = sim();
        let mut o = Occluders::default();
        s.world.map.ensure_rooms();
        o.pack(&s.world);
        // A 3×3 hut: a ring of walls round one cell.
        let p = open_block(&s.world, 3, 3);
        for (dx, dy) in [(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)] {
            place(&mut s.world, "wall", p.offset(dx, dy), false);
        }
        s.world.map.ensure_rooms();
        assert!(s.world.map.indoors(p.offset(1, 1)), "the hut is a room");
        let (whole, dirty) = o.pack(&s.world);
        let (cw, ch) = s.world.map.chunks();
        assert!(!whole && (1..=4).contains(&dirty.len()) && dirty.len() < (cw * ch) as usize, "{} chunks", dirty.len());
        assert_eq!(o.at(s.world.map.idx(p.offset(1, 1)))[1], 255, "the floor is under a roof");
    }
}
