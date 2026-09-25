//! Every loaded mod's sprites and glyphs, packed at load into one texture (DESIGN.md
//! §8). A switch between two textures ends a draw call, so sprites from
//! twenty mods side by side on screen would cost a call each; from one
//! atlas they cost none. Each page keeps a white block, so primitives
//! (fills, discs) sample it and batch with the sprites.
//!
//! A page is the smallest power of two that fits, up to `MAX_PAGE`, the
//! largest texture the reference class of integrated GPU takes. What
//! doesn't fit spills to another page, and the load log says so.

use macroquad::prelude::*;
use std::path::PathBuf;

pub const MAX_PAGE: u32 = 4096;
/// The white block at each page's origin, and the gutter between sprites.
const WHITE: u32 = 4;
const GUTTER: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub page: usize,
    /// Normalised u0, v0, u1, v1.
    pub uv: [f32; 4],
}

pub struct WorldAtlas {
    pub pages: Vec<Texture2D>,
    /// Per page, the centre of the white block, normalised.
    white: Vec<[f32; 2]>,
    /// By sprite id (`DefDb::sprites`).
    slots: Vec<Slot>,
    /// By glyph id (`DefDb::glyphs`); None where no font has it.
    glyphs: Vec<Option<Glyph>>,
}

/// Glyphs are rasterised this many pixels high and scaled to the cell.
const GLYPH_PX: f32 = 48.0;

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub slot: Slot,
    /// Width over height.
    pub aspect: f32,
    /// A colour glyph: draw as is rather than in the layer's colour.
    pub painted: bool,
}

/// Where each rectangle goes: (page, x, y), and each page's side.
fn pack(sizes: &[(u32, u32)], max: u32) -> (Vec<(usize, u32, u32)>, Vec<u32>) {
    // Tallest first: shelves waste least when rows are even.
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(sizes[i].1), i));
    let try_side = |side: u32, items: &[usize]| -> (Vec<(usize, u32, u32)>, Vec<usize>) {
        let (mut x, mut y, mut row) = (WHITE + GUTTER, 0, WHITE);
        let (mut placed, mut rest) = (Vec::new(), Vec::new());
        for &i in items {
            let (w, h) = sizes[i];
            if x + w + GUTTER > side {
                (x, y, row) = (GUTTER, y + row + GUTTER, 0);
            }
            if w + 2 * GUTTER > side || y + h + GUTTER > side {
                rest.push(i);
                continue;
            }
            placed.push((i, x, y));
            x += w + GUTTER;
            row = row.max(h);
        }
        (placed, rest)
    };
    let mut at = vec![(0, 0, 0); sizes.len()];
    let mut sides = Vec::new();
    let mut left = order;
    while !left.is_empty() || sides.is_empty() {
        let mut side = 64;
        let (placed, rest) = loop {
            let (placed, rest) = try_side(side, &left);
            if rest.is_empty() || side >= max {
                break (placed, rest);
            }
            side *= 2;
        };
        if placed.is_empty() && !rest.is_empty() {
            // Bigger than a page: nothing will ever place it.
            for i in rest {
                at[i] = (usize::MAX, 0, 0);
            }
            break;
        }
        for (i, x, y) in placed {
            at[i] = (sides.len(), x, y);
        }
        sides.push(side);
        left = rest;
    }
    (at, sides)
}

/// Can a `w`×`h` sprite go on a page at all?
pub fn fits(w: u32, h: u32) -> bool {
    pack(&[(w, h)], MAX_PAGE).0[0].0 != usize::MAX
}

impl WorldAtlas {
    /// Decode and pack `files` (by sprite id), and rasterise `glyphs` with
    /// the UI's font. Errors name the file. A glyph no font has is a
    /// warning and draws nothing: a mod shouldn't stop the game on a
    /// machine with fewer fonts.
    pub fn load(files: &[PathBuf], glyphs: &[String], text: &mut rim_ui::text::Text) -> Result<WorldAtlas, String> {
        let mut images = Vec::with_capacity(files.len() + glyphs.len());
        for f in files {
            let (w, h, rgba) = rim_ui::image::decode(f).map_err(|e| format!("sprite {}: {e}", f.display()))?;
            images.push((w, h, rgba));
        }
        let mut glyph_meta = Vec::with_capacity(glyphs.len());
        for g in glyphs {
            match text.rasterize(g, GLYPH_PX) {
                Some(r) => {
                    glyph_meta.push(Some((images.len(), r.w as f32 / r.h as f32, r.painted)));
                    images.push((r.w, r.h, r.rgba));
                }
                None => {
                    eprintln!("  warning: glyph {g:?} is in no font here; it draws nothing");
                    glyph_meta.push(None);
                }
            }
        }
        let sizes: Vec<(u32, u32)> = images.iter().map(|i| (i.0, i.1)).collect();
        let (at, sides) = pack(&sizes, MAX_PAGE);
        if let Some(i) = at.iter().position(|a| a.0 == usize::MAX) {
            return Err(format!("sprite {}: larger than {MAX_PAGE}×{MAX_PAGE}", files[i].display()));
        }
        if sides.len() > 1 {
            eprintln!("  world atlas: {} sprites need {} pages of up to {MAX_PAGE}²", files.len(), sides.len());
        }
        let mut pixels: Vec<Vec<u8>> = sides.iter().map(|&s| vec![0; (s * s * 4) as usize]).collect();
        for (p, &side) in pixels.iter_mut().zip(&sides) {
            for y in 0..WHITE {
                for x in 0..WHITE {
                    let i = ((y * side + x) * 4) as usize;
                    p[i..i + 4].copy_from_slice(&[255; 4]);
                }
            }
        }
        let mut slots = Vec::with_capacity(images.len());
        for ((w, h, rgba), &(page, x, y)) in images.iter().zip(&at) {
            let side = sides[page];
            for row in 0..*h {
                let src = (row * w * 4) as usize;
                let dst = (((y + row) * side + x) * 4) as usize;
                pixels[page][dst..dst + (w * 4) as usize].copy_from_slice(&rgba[src..src + (w * 4) as usize]);
            }
            let s = side as f32;
            slots.push(Slot { page, uv: [x as f32 / s, y as f32 / s, (x + w) as f32 / s, (y + h) as f32 / s] });
        }
        let pages = pixels
            .into_iter()
            .zip(&sides)
            .map(|(p, &side)| {
                let t = Texture2D::from_rgba8(side as u16, side as u16, &p);
                // Crisp, like the ground: sprites are pixel art until a mod
                // says otherwise.
                t.set_filter(FilterMode::Nearest);
                t
            })
            .collect();
        let white = sides.iter().map(|&s| [WHITE as f32 / 2.0 / s as f32; 2]).collect();
        let glyphs = glyph_meta
            .into_iter()
            .map(|m| m.map(|(i, aspect, painted)| Glyph { slot: slots[i], aspect, painted }))
            .collect();
        slots.truncate(files.len());
        Ok(WorldAtlas { pages, white, slots, glyphs })
    }

    pub fn slot(&self, id: u16) -> Slot {
        self.slots[id as usize]
    }

    pub fn glyph(&self, id: u16) -> Option<Glyph> {
        self.glyphs[id as usize]
    }

    /// Where a primitive samples solid colour on `page`.
    pub fn white(&self, page: usize) -> [f32; 2] {
        self.white[page]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_mods_of_sprites_share_one_small_page() {
        // Twenty mods, each shipping a few 32×32 sprites and a big one.
        let mut sizes = Vec::new();
        for _ in 0..20 {
            sizes.extend([(32, 32), (32, 32), (16, 16), (64, 48)]);
        }
        let (at, sides) = pack(&sizes, MAX_PAGE);
        assert_eq!(sides.len(), 1, "one page: one texture, one draw call per chunk layer");
        assert!(sides[0] <= 512, "and no bigger than it needs ({})", sides[0]);
        assert!(at.iter().all(|a| a.0 == 0));
    }

    #[test]
    fn nothing_overlaps_the_white_block_or_each_other() {
        let sizes: Vec<(u32, u32)> = (0..60).map(|i| (8 + i % 7 * 5, 8 + i % 5 * 9)).collect();
        let (at, sides) = pack(&sizes, 256);
        let rects: Vec<(usize, u32, u32, u32, u32)> =
            at.iter().zip(&sizes).map(|(&(p, x, y), &(w, h))| (p, x, y, w, h)).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a.1 + a.3 <= sides[a.0] && a.2 + a.4 <= sides[a.0], "inside its page");
            assert!(a.1 >= WHITE + GUTTER || a.2 >= WHITE + GUTTER, "clear of the white block");
            for b in &rects[i + 1..] {
                let apart = a.0 != b.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1 || a.2 + a.4 <= b.2 || b.2 + b.4 <= a.2;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn what_does_not_fit_spills_to_another_page() {
        let sizes = vec![(200, 200); 5];
        let (at, sides) = pack(&sizes, 256);
        assert_eq!(sides.len(), 5, "one 200² sprite per 256² page");
        assert_eq!(at.iter().map(|a| a.0).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        let (at, _) = pack(&[(300, 10)], 256);
        assert_eq!(at[0].0, usize::MAX, "bigger than a page is refused, not looped on");
        assert!(fits(4000, 4000) && !fits(4094, 4094), "the white block and gutters count");
    }
}
