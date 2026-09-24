//! Text: the system UI font, shaping with cosmic-text, and a glyph atlas.
//!
//! The primary font is the operating system's own UI font, found at runtime
//! and never shipped: San Francisco on macOS, Segoe UI on Windows, fontconfig's
//! `sans-serif` on Linux. Every other installed font is loaded as a fallback,
//! so accents, CJK, Arabic and emoji render even when the primary lacks them.
//!
//! Shaped text is cached by (string, size, weight, wrap width). Glyphs are
//! rasterised once into an RGBA atlas that the renderer uploads when it
//! changes; a steady frame uploads nothing.

use cosmic_text::{
    fontdb, Attrs, Buffer, CacheKey, Family, FontSystem, Hinting, Metrics, Shaping, SwashCache, SwashContent, Weight,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Candidate files for the system UI font, most specific first.
fn system_font_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(target_os = "macos")]
    {
        v.push("/System/Library/Fonts/SFNS.ttf".into());
        v.push("/System/Library/Fonts/SFNSText.ttf".into());
        v.push("/System/Library/Fonts/Helvetica.ttc".into());
    }
    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        v.push(PathBuf::from(&windir).join("Fonts").join("segoeui.ttf"));
        v.push(PathBuf::from(&windir).join("Fonts").join("arial.ttf"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // fontconfig knows what the desktop calls its UI font.
        if let Ok(out) = std::process::Command::new("fc-match").args(["-f", "%{file}", "sans-serif"]).output() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                v.push(path.into());
            }
        }
        for p in [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ] {
            v.push(p.into());
        }
    }
    v
}

/// Which font the UI ended up with, for the profiler and bug reports.
#[derive(Clone, Debug)]
pub struct FontInfo {
    pub family: String,
    pub source: String,
    pub fallback_faces: usize,
    /// A family the theme asked for that isn't installed (the system UI font
    /// was used instead).
    pub missing: Option<String>,
    /// The font list came from the disk cache rather than a scan.
    pub cached: bool,
}

/// Where a glyph sits in the atlas, and where to draw it relative to the pen.
#[derive(Clone, Copy, Debug)]
struct GlyphSlot {
    /// Atlas rectangle in pixels.
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    /// Offset from the glyph's pen position to the bitmap's top-left.
    left: i16,
    top: i16,
    /// Colour glyphs (emoji) ignore the text colour.
    color: bool,
}

/// A laid-out string: its size and the glyphs to draw, relative to its top-left.
#[derive(Clone, Debug, Default)]
pub struct Shaped {
    pub width: f32,
    pub height: f32,
    glyphs: Vec<(CacheKey, i32, i32)>,
}

/// One glyph to draw: destination and source rectangles in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    pub dst: [f32; 4],
    pub uv: [f32; 4],
    /// Draw with its own colours (emoji) instead of the text colour.
    pub color: bool,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct ShapeKey {
    text: String,
    size_q: u32,
    weight: u16,
    width_q: Option<u32>,
}

pub struct Atlas {
    pub size: u32,
    /// RGBA8, `size * size * 4` bytes.
    pub pixels: Vec<u8>,
    /// Set when pixels changed since the renderer last uploaded them.
    pub dirty: bool,
    /// Bumped whenever the atlas was cleared and every glyph re-placed.
    pub generation: u64,
    shelf_x: u32,
    shelf_y: u32,
    shelf_h: u32,
}

/// A solid white block at the atlas origin. The renderer draws shapes
/// (panels, outlines) with this texel so they batch with glyphs in one draw
/// call instead of switching between textured and untextured geometry.
const WHITE: u32 = 4;
/// Glyph shelves start below the white block.
const FIRST_SHELF: u32 = WHITE + 1;

impl Atlas {
    fn new(size: u32) -> Self {
        let mut a = Atlas {
            size,
            pixels: vec![0; (size * size * 4) as usize],
            dirty: true,
            generation: 0,
            shelf_x: 1,
            shelf_y: FIRST_SHELF,
            shelf_h: 0,
        };
        a.fill_white();
        a
    }

    fn fill_white(&mut self) {
        for y in 0..WHITE {
            for x in 0..WHITE {
                let i = ((y * self.size + x) * 4) as usize;
                self.pixels[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }

    /// The centre of the white block, in pixels: sample here for solid
    /// colour. The centre, so linear filtering never reaches a glyph.
    pub fn white_texel(&self) -> (f32, f32) {
        (WHITE as f32 / 2.0, WHITE as f32 / 2.0)
    }

    /// Shelf packing: fill a row left to right, then start a new row.
    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w + 2 > self.size || h + 2 > self.size {
            return None;
        }
        if self.shelf_x + w + 1 > self.size {
            self.shelf_y += self.shelf_h + 1;
            self.shelf_x = 1;
            self.shelf_h = 0;
        }
        if self.shelf_y + h + 1 > self.size {
            return None;
        }
        let at = (self.shelf_x, self.shelf_y);
        self.shelf_x += w + 1;
        self.shelf_h = self.shelf_h.max(h);
        Some(at)
    }

    fn clear(&mut self) {
        self.pixels.iter_mut().for_each(|p| *p = 0);
        self.fill_white();
        self.shelf_x = 1;
        self.shelf_y = FIRST_SHELF;
        self.shelf_h = 0;
        self.generation += 1;
        self.dirty = true;
    }
}

pub struct Text {
    pub fonts: FontSystem,
    pub info: FontInfo,
    family: String,
    swash: SwashCache,
    /// Shaped strings with the frame they were last used.
    shaped: HashMap<ShapeKey, (Shaped, u64)>,
    frame: u64,
    slots: HashMap<CacheKey, Option<GlyphSlot>>,
    pub atlas: Atlas,
    /// Shaping cache misses, for tests and the profiler.
    pub shapes: u64,
    /// Snap glyph advances to whole pixels. Crisper small text on 1x
    /// displays; at 2x and above, subpixel positions read smoother.
    hinting: bool,
}

impl Text {
    /// Load the system UI font (or `family`, if the theme names one) and
    /// every installed font as a fallback.
    pub fn new(family: Option<&str>) -> Result<Text, String> {
        // From the disk cache, or scanned (maybe already, on the preload thread).
        let (mut db, from) = crate::fontcache::take();
        let fallback_faces = db.len();

        let mut chosen: Option<(String, String)> = None;
        let mut missing = None;
        if let Some(want) = family {
            let found = db.faces().find(|f| f.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(want)));
            match found {
                Some(f) => chosen = Some((f.families[0].0.clone(), format!("family '{want}'"))),
                None => missing = Some(want.to_string()),
            }
        }
        if chosen.is_none() {
            for path in system_font_candidates() {
                if !path.is_file() {
                    continue;
                }
                let before: Vec<fontdb::ID> = db.faces().map(|f| f.id).collect();
                if db.load_font_file(&path).is_err() {
                    continue;
                }
                // The face we just added, or the one already loaded from that file.
                let face = db
                    .faces()
                    .find(|f| !before.contains(&f.id))
                    .or_else(|| db.faces().find(|f| matches!(&f.source, fontdb::Source::File(p) if p == &path)));
                if let Some(f) = face {
                    chosen = Some((f.families[0].0.clone(), path.display().to_string()));
                    break;
                }
            }
        }
        let (family, source) = match chosen {
            Some(c) => c,
            None => {
                // Any sans-serif face at all.
                let f = db.faces().next().ok_or("no fonts installed on this system")?;
                (f.families[0].0.clone(), "first installed font".to_string())
            }
        };
        let fonts = FontSystem::new_with_locale_and_db("en-US".into(), db);
        Ok(Text {
            fonts,
            info: FontInfo {
                family: family.clone(),
                source,
                fallback_faces,
                missing,
                cached: from == crate::fontcache::FontsFrom::Cache,
            },
            family,
            swash: SwashCache::new(),
            shaped: HashMap::new(),
            frame: 0,
            slots: HashMap::new(),
            atlas: Atlas::new(1024),
            shapes: 0,
            hinting: false,
        })
    }

    /// Call once per frame. Shaped strings unused for ~10 seconds (the clock
    /// as it was, old profiler numbers) are dropped so the cache stays small.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        // cosmic-text's own shaping cache (text + attributes, any wrap
        // width): keep runs used in the last few seconds.
        self.fonts.shape_run_cache.trim(300);
        if self.frame.is_multiple_of(600) {
            let now = self.frame;
            self.shaped.retain(|_, (_, used)| now - *used < 600);
        }
    }

    /// Turn width hinting on (1x displays) or off. Changing it drops shaped
    /// text, since glyph positions change.
    pub fn set_hinting(&mut self, on: bool) {
        if on != self.hinting {
            self.hinting = on;
            self.shaped.clear();
        }
    }

    pub fn cached_strings(&self) -> usize {
        self.shaped.len()
    }

    /// Shape `text` at `size` pixels and `weight` (400 regular, 600 semibold),
    /// wrapping at `width` if given. Cached.
    pub fn shape(&mut self, text: &str, size: f32, weight: u16, width: Option<f32>) -> &Shaped {
        let key = ShapeKey {
            text: text.to_string(),
            size_q: (size * 4.0).round() as u32,
            weight,
            width_q: width.map(|w| (w * 4.0).round() as u32),
        };
        let frame = self.frame;
        if let Some(entry) = self.shaped.get_mut(&key) {
            entry.1 = frame;
        } else {
            self.shapes += 1;
            let line_height = (size * 1.3).ceil();
            let mut buf = Buffer::new(&mut self.fonts, Metrics::new(size, line_height));
            buf.set_size(width, None);
            buf.set_hinting(if self.hinting { Hinting::Enabled } else { Hinting::Disabled });
            let attrs = Attrs::new().family(Family::Name(&self.family)).weight(Weight(weight));
            buf.set_text(text, &attrs, Shaping::Advanced, None);
            buf.shape_until_scroll(&mut self.fonts, false);
            let mut out = Shaped::default();
            for run in buf.layout_runs() {
                out.width = out.width.max(run.line_w);
                out.height = out.height.max(run.line_top + run.line_height);
                for g in run.glyphs {
                    let p = g.physical((0.0, run.line_y), 1.0);
                    out.glyphs.push((p.cache_key, p.x, p.y));
                }
            }
            if out.height == 0.0 {
                out.height = line_height;
            }
            out.width = out.width.ceil();
            self.shaped.insert(key.clone(), (out, frame));
        }
        &self.shaped[&key].0
    }

    /// Glyph quads for text shaped earlier, with its top-left at (x, y).
    pub fn quads(&mut self, text: &str, size: f32, weight: u16, width: Option<f32>, x: f32, y: f32) -> Vec<GlyphQuad> {
        let glyphs = self.shape(text, size, weight, width).glyphs.clone();
        let mut out = Vec::with_capacity(glyphs.len());
        for (key, gx, gy) in glyphs {
            let Some(slot) = self.slot(key) else { continue };
            out.push(GlyphQuad {
                dst: [
                    (x.round() as i32 + gx + slot.left as i32) as f32,
                    (y.round() as i32 + gy - slot.top as i32) as f32,
                    slot.w as f32,
                    slot.h as f32,
                ],
                uv: [slot.x as f32, slot.y as f32, slot.w as f32, slot.h as f32],
                color: slot.color,
            });
        }
        out
    }

    /// Rasterise a glyph into the atlas the first time it's needed.
    fn slot(&mut self, key: CacheKey) -> Option<GlyphSlot> {
        if let Some(s) = self.slots.get(&key) {
            return *s;
        }
        let img = self.swash.get_image_uncached(&mut self.fonts, key);
        let slot = img.and_then(|img| {
            let (w, h) = (img.placement.width, img.placement.height);
            if w == 0 || h == 0 {
                return None;
            }
            let (ax, ay) = match self.atlas.alloc(w, h) {
                Some(a) => a,
                None => {
                    // Full: start over. Everything re-rasterises on demand.
                    self.atlas.clear();
                    self.slots.clear();
                    self.atlas.alloc(w, h)?
                }
            };
            let size = self.atlas.size;
            for row in 0..h {
                for col in 0..w {
                    let i = ((ay + row) * size + ax + col) as usize * 4;
                    let (r, g, b, a) = match img.content {
                        SwashContent::Mask => (255, 255, 255, img.data[(row * w + col) as usize]),
                        SwashContent::Color => {
                            let j = (row * w + col) as usize * 4;
                            (img.data[j], img.data[j + 1], img.data[j + 2], img.data[j + 3])
                        }
                        SwashContent::SubpixelMask => {
                            let j = (row * w + col) as usize * 4;
                            let a = img.data[j].max(img.data[j + 1]).max(img.data[j + 2]);
                            (255, 255, 255, a)
                        }
                    };
                    self.atlas.pixels[i..i + 4].copy_from_slice(&[r, g, b, a]);
                }
            }
            self.atlas.dirty = true;
            Some(GlyphSlot {
                x: ax as u16,
                y: ay as u16,
                w: w as u16,
                h: h as u16,
                left: img.placement.left as i16,
                top: img.placement.top as i16,
                color: matches!(img.content, SwashContent::Color),
            })
        });
        self.slots.insert(key, slot);
        slot
    }
}
