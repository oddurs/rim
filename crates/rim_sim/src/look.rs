//! How a thing is drawn, as data (DESIGN.md §6a). A look is layers of a
//! few primitives painted in order over the thing's cell. The renderer
//! knows the primitives and nothing about what the thing is, so a loom, a
//! fence or a hedge needs no renderer change.
//!
//! The sim never reads a look; it is here so it loads, patches and fails
//! validation with every other def.

use serde::Deserialize;

/// A def's `look` as written.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct LookDef {
    #[serde(default)]
    pub layers: Vec<LayerDef>,
    /// Drawn instead of `layers` while a harvested plant grows back.
    #[serde(default)]
    pub regrowing: Vec<LayerDef>,
    /// A free label. Built things with the same `join` join up: an
    /// `edges` layer leaves out the sides that face one.
    #[serde(default)]
    pub join: Option<String>,
}

/// One layer as written. Which fields apply depends on `draw`; the rest
/// are refused so a typo can't pass for a default.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LayerDef {
    pub draw: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub w: Option<f32>,
    pub h: Option<f32>,
    pub r: Option<f32>,
    /// Line width in screen points, for `outline` and `edges`.
    pub width: Option<f32>,
    /// Smallest radius (disc) or width and height (fill) in points, so a
    /// berry or a seam still shows zoomed out.
    pub min_px: Option<f32>,
    /// A fixed colour, "#rrggbb" or "#rrggbbaa". Unset: the thing's colour,
    /// or its material's when it is made of one.
    pub color: Option<String>,
    /// Multiplies the colour's brightness.
    pub shade: Option<f32>,
    /// Brightness varies per cell by up to this much, so a field of rock
    /// isn't one flat colour.
    pub vary: Option<f32>,
    /// A disc's radius flickers by up to this fraction.
    pub pulse: Option<f32>,
    /// For `sprite`: a PNG the mod ships under `sprites/`, as `name` (this
    /// mod's) or `mod:name`.
    pub sprite: Option<String>,
    /// For `sprite`: multiply it by the thing's colour (or its material's),
    /// so one grey plank serves every wood. Unset, it draws as painted.
    pub tint: Option<bool>,
    /// For `glyph`: one character, drawn in the layer's colour.
    pub glyph: Option<String>,
    /// For `glyph`: its height as a fraction of the cell.
    pub size: Option<f32>,
    /// While it is built, the stretch of the work this layer appears over:
    /// absent before `from`, rising from the bottom (a disc from its middle)
    /// until `to`, whole after (DESIGN.md §6b).
    pub grow: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prim {
    /// A rectangle in cell units, from the cell's top-left.
    Fill { rect: [f32; 4], min_px: f32 },
    /// A rectangle's outline, `width` points thick.
    Outline { rect: [f32; 4], width: f32 },
    /// A disc around a point in cell units.
    Disc { at: [f32; 2], r: f32, min_px: f32, pulse: f32 },
    /// The cell's border on the sides that don't face a joined neighbour.
    Edges { width: f32 },
    /// A mod's picture over a rectangle in cell units. `id` indexes
    /// `DefDb::sprites`.
    Sprite { rect: [f32; 4], id: u16 },
    /// A character centred on a point, `size` of the cell high. `id`
    /// indexes `DefDb::glyphs`.
    Glyph { at: [f32; 2], size: f32, id: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub prim: Prim,
    /// Fixed RGBA, or None for the thing's own colour.
    pub color: Option<[u8; 4]>,
    pub shade: f32,
    pub vary: f32,
    /// Its `grow` window, if the def gave one.
    pub grow: Option<[f32; 2]>,
}

#[derive(Clone, Debug, Default)]
pub struct Look {
    pub layers: Vec<Layer>,
    pub regrowing: Vec<Layer>,
    /// Index into `DefDb::join_groups`.
    pub join: Option<u16>,
}

impl Look {
    /// Does it change from frame to frame? Then it can't be cached.
    pub fn animated(&self) -> bool {
        self.layers.iter().chain(&self.regrowing).any(|l| matches!(l.prim, Prim::Disc { pulse, .. } if pulse > 0.0))
    }
}

/// What a def with no look draws: its cell filled in its colour, so
/// nothing is ever invisible.
pub fn plain() -> Vec<Layer> {
    vec![Layer {
        prim: Prim::Fill { rect: [0.0, 0.0, 1.0, 1.0], min_px: 0.0 },
        color: None,
        shade: 1.0,
        vary: 0.0,
        grow: None,
    }]
}

impl Layer {
    /// Layer `i` of `n`'s window: its own `grow`, or an even share of the
    /// work in paint order, so every look builds up with no edits.
    pub fn window(&self, i: usize, n: usize) -> [f32; 2] {
        self.grow.unwrap_or([i as f32 / n as f32, (i + 1) as f32 / n as f32])
    }
}

/// What the looks ask the world atlas for, in the order first used:
/// sprite keys, qualified (`mod:name`), which the modloader checks have a
/// file, and glyphs, which the client rasterises. The client packs both.
pub struct Art<'a> {
    pub sprites: &'a mut Vec<String>,
    pub glyphs: &'a mut Vec<String>,
    /// The mod whose def is compiling: a bare name is its sprite.
    pub home: &'a str,
}

/// One character as a player sees it: a base that draws something, and
/// only what extends it (variation selectors, joiners and what they join,
/// skin tones, keycaps, a flag's second letter). "ab" is two.
fn one_character(g: &str) -> bool {
    let mut cs = g.chars();
    let Some(base) = cs.next() else { return false };
    if base.is_whitespace() || base.is_control() || ('\u{200B}'..='\u{200F}').contains(&base) {
        return false;
    }
    let flag = |c: char| ('\u{1F1E6}'..='\u{1F1FF}').contains(&c);
    let mut joined = false;
    cs.all(|c| {
        let extends = matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{20E3}' | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0020}'..='\u{E007F}')
            || c == '\u{200D}'
            || joined
            || (flag(base) && flag(c));
        joined = c == '\u{200D}';
        extends
    })
}

fn intern(list: &mut Vec<String>, item: String) -> u16 {
    match list.iter().position(|k| *k == item) {
        Some(i) => i as u16,
        None => {
            list.push(item);
            (list.len() - 1) as u16
        }
    }
}

impl Art<'_> {
    fn sprite(&mut self, key: &str) -> u16 {
        let full = if key.contains(':') { key.to_string() } else { format!("{}:{key}", self.home) };
        intern(self.sprites, full)
    }
}

pub fn parse_rgba(s: &str) -> Result<[u8; 4], String> {
    let h = s.trim_start_matches('#');
    // Checked as hex digits first, so slicing by byte can't cut a character.
    if (h.len() != 6 && h.len() != 8) || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("bad color '{s}' (want #rrggbb or #rrggbbaa)"));
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| format!("bad color '{s}'"));
    Ok([p(0)?, p(2)?, p(4)?, if h.len() == 8 { p(6)? } else { 255 }])
}

impl LayerDef {
    fn compile(&self, art: &mut Art) -> Result<Layer, String> {
        let allowed: &[&str] = match self.draw.as_str() {
            "fill" => &["x", "y", "w", "h", "min_px"],
            "outline" => &["x", "y", "w", "h", "width"],
            "disc" => &["x", "y", "r", "min_px", "pulse"],
            "edges" => &["width"],
            "sprite" => &["x", "y", "w", "h", "sprite", "tint"],
            "glyph" => &["x", "y", "glyph", "size"],
            other => return Err(format!("unknown draw '{other}' (want fill, outline, disc, edges, sprite or glyph)")),
        };
        let given = [
            ("x", self.x.is_some()),
            ("y", self.y.is_some()),
            ("w", self.w.is_some()),
            ("h", self.h.is_some()),
            ("r", self.r.is_some()),
            ("width", self.width.is_some()),
            ("min_px", self.min_px.is_some()),
            ("pulse", self.pulse.is_some()),
            ("sprite", self.sprite.is_some()),
            ("tint", self.tint.is_some()),
            ("glyph", self.glyph.is_some()),
            ("size", self.size.is_some()),
        ];
        if let Some((name, _)) = given.iter().find(|(n, set)| *set && !allowed.contains(n)) {
            return Err(format!("`{name}` does not apply to draw = \"{}\"", self.draw));
        }
        // Numbers that would draw nothing, or something inside out, are
        // refused rather than drawn wrong.
        let checks: [(&str, Option<f32>, f32, f32); 11] = [
            ("x", self.x, f32::MIN, f32::MAX),
            ("y", self.y, f32::MIN, f32::MAX),
            ("w", self.w, 0.0, f32::MAX),
            ("h", self.h, 0.0, f32::MAX),
            ("r", self.r, 0.0, f32::MAX),
            ("width", self.width, f32::MIN_POSITIVE, f32::MAX),
            ("min_px", self.min_px, 0.0, f32::MAX),
            ("shade", self.shade, 0.0, f32::MAX),
            ("vary", self.vary, 0.0, 1.0),
            ("pulse", self.pulse, 0.0, 0.99),
            ("size", self.size, f32::MIN_POSITIVE, 4.0),
        ];
        for (name, v, lo, hi) in checks {
            if let Some(v) = v {
                if !(lo..=hi).contains(&v) {
                    let range = match (lo, hi) {
                        (_, f32::MAX) if lo > 0.0 => "above 0".to_string(),
                        (_, f32::MAX) => format!("at least {lo}"),
                        _ if lo > 0.0 && lo < 1e-6 => format!("above 0, at most {hi}"),
                        _ => format!("from {lo} to {hi}"),
                    };
                    return Err(format!("`{name}` = {v} is out of range (want {range})"));
                }
            }
        }
        let rect = [self.x.unwrap_or(0.0), self.y.unwrap_or(0.0), self.w.unwrap_or(1.0), self.h.unwrap_or(1.0)];
        let prim = match self.draw.as_str() {
            "fill" => Prim::Fill { rect, min_px: self.min_px.unwrap_or(0.0) },
            "outline" => Prim::Outline { rect, width: self.width.unwrap_or(1.0) },
            "disc" => Prim::Disc {
                at: [self.x.unwrap_or(0.5), self.y.unwrap_or(0.5)],
                r: self.r.unwrap_or(0.4),
                min_px: self.min_px.unwrap_or(0.0),
                pulse: self.pulse.unwrap_or(0.0),
            },
            "sprite" => {
                let key = self.sprite.as_deref().ok_or("draw = \"sprite\" needs `sprite`, the picture's name")?;
                Prim::Sprite { rect, id: art.sprite(key) }
            }
            "glyph" => {
                let g = self.glyph.as_deref().unwrap_or_default();
                if !one_character(g) {
                    return Err(format!("`glyph` is one character that draws something (got {g:?})"));
                }
                let at = [self.x.unwrap_or(0.5), self.y.unwrap_or(0.5)];
                Prim::Glyph { at, size: self.size.unwrap_or(0.8), id: intern(art.glyphs, g.to_string()) }
            }
            _ => Prim::Edges { width: self.width.unwrap_or(1.5) },
        };
        if let Some([from, to]) = self.grow {
            if !(0.0..1.0).contains(&from) || to <= from || to > 1.0 {
                return Err(format!("`grow` = [{from}, {to}] is not a window of the work (want 0 ≤ from < to ≤ 1)"));
            }
        }
        // A sprite draws as painted unless tinted or given a colour.
        let own = if matches!(prim, Prim::Sprite { .. }) && self.tint != Some(true) { Some([255; 4]) } else { None };
        Ok(Layer {
            prim,
            color: self.color.as_deref().map(parse_rgba).transpose()?.or(own),
            shade: self.shade.unwrap_or(1.0),
            vary: self.vary.unwrap_or(0.0),
            grow: self.grow,
        })
    }
}

impl LookDef {
    /// `groups` interns join labels across all defs, `art` sprite keys and
    /// glyphs.
    pub fn compile(&self, groups: &mut Vec<String>, art: &mut Art) -> Result<Look, String> {
        let mut layers = |v: &[LayerDef], what: &str| {
            v.iter()
                .enumerate()
                .map(|(i, l)| l.compile(art).map_err(|e| format!("look.{what}[{}]: {e}", i + 1)))
                .collect::<Result<Vec<_>, _>>()
        };
        let join = self.join.as_ref().map(|j| match groups.iter().position(|g| g == j) {
            Some(i) => i as u16,
            None => {
                groups.push(j.clone());
                (groups.len() - 1) as u16
            }
        });
        let mut look =
            Look { layers: layers(&self.layers, "layers")?, regrowing: layers(&self.regrowing, "regrowing")?, join };
        if look.layers.is_empty() {
            look.layers = plain();
        }
        Ok(look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(toml: &str) -> Result<Layer, String> {
        let (mut sprites, mut glyphs) = (Vec::new(), Vec::new());
        let mut art = Art { sprites: &mut sprites, glyphs: &mut glyphs, home: "m" };
        toml::from_str::<LayerDef>(toml).map_err(|e| e.to_string())?.compile(&mut art)
    }

    #[test]
    fn sprites_qualify_to_their_mod_and_draw_as_painted_unless_tinted() {
        let (mut keys, mut glyphs) = (Vec::new(), Vec::new());
        let mut sp = Art { sprites: &mut keys, glyphs: &mut glyphs, home: "loom" };
        let a = toml::from_str::<LayerDef>("draw = \"sprite\"\nsprite = \"frame\"").unwrap().compile(&mut sp).unwrap();
        let b = toml::from_str::<LayerDef>("draw = \"sprite\"\nsprite = \"loom:frame\"\ntint = true")
            .unwrap()
            .compile(&mut sp)
            .unwrap();
        let c =
            toml::from_str::<LayerDef>("draw = \"sprite\"\nsprite = \"core:plank\"").unwrap().compile(&mut sp).unwrap();
        assert_eq!((a.prim, a.color), (Prim::Sprite { rect: [0.0, 0.0, 1.0, 1.0], id: 0 }, Some([255; 4])));
        assert_eq!((b.prim, b.color), (Prim::Sprite { rect: [0.0, 0.0, 1.0, 1.0], id: 0 }, None));
        assert_eq!(c.prim, Prim::Sprite { rect: [0.0, 0.0, 1.0, 1.0], id: 1 });
        assert_eq!(keys, ["loom:frame", "core:plank"]);
        assert!(layer("draw = \"sprite\"").unwrap_err().contains("needs `sprite`"));
        assert!(layer("draw = \"fill\"\nsprite = \"x\"").unwrap_err().contains("does not apply"));
    }

    #[test]
    fn defaults_fill_the_cell_and_centre_the_disc() {
        assert_eq!(layer("draw = \"fill\"").unwrap().prim, Prim::Fill { rect: [0.0, 0.0, 1.0, 1.0], min_px: 0.0 });
        assert_eq!(
            layer("draw = \"disc\"\nr = 0.3").unwrap().prim,
            Prim::Disc { at: [0.5, 0.5], r: 0.3, min_px: 0.0, pulse: 0.0 }
        );
    }

    #[test]
    fn a_field_that_does_not_apply_is_an_error() {
        let e = layer("draw = \"fill\"\nr = 0.3").unwrap_err();
        assert!(e.contains("`r` does not apply"), "{e}");
        assert!(layer("draw = \"blob\"").unwrap_err().contains("unknown draw"));
        assert!(layer("draw = \"fill\"\nradius = 1").is_err(), "unknown keys are refused");
    }

    #[test]
    fn numbers_that_draw_nothing_or_inside_out_are_refused() {
        for bad in [
            "draw = \"fill\"\nw = -0.5",
            "draw = \"disc\"\nr = -0.3",
            "draw = \"edges\"\nwidth = 0",
            "draw = \"fill\"\nvary = 2.5",
            "draw = \"disc\"\npulse = 1.0",
            "draw = \"fill\"\nshade = nan",
        ] {
            let e = layer(bad).unwrap_err();
            assert!(e.contains("out of range"), "{bad}: {e}");
        }
        assert!(layer("draw = \"fill\"\nh = 0.0\nmin_px = 1.0").is_ok(), "a seam is zero high and min_px tall");
    }

    #[test]
    fn a_glyph_is_one_character_centred_by_default() {
        let l = layer("draw = \"glyph\"\nglyph = \"\u{25b2}\"").unwrap();
        assert_eq!(l.prim, Prim::Glyph { at: [0.5, 0.5], size: 0.8, id: 0 });
        assert!(layer("draw = \"glyph\"").unwrap_err().contains("one character"));
        assert!(layer("draw = \"glyph\"\nglyph = \"ab\"").unwrap_err().contains("one character"));
        assert!(layer("draw = \"glyph\"\nglyph = \" \"").unwrap_err().contains("draws something"));
        for emoji in [
            "\u{2764}\u{FE0F}",
            "\u{1F1EE}\u{1F1F8}",
            "\u{1F44D}\u{1F3FD}",
            "1\u{FE0F}\u{20E3}",
            "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        ] {
            assert!(layer(&format!("draw = \"glyph\"\nglyph = \"{emoji}\"")).is_ok(), "{emoji:?} is one character");
        }
        let e = layer("draw = \"glyph\"\nglyph = \"a\"\nsize = 0").unwrap_err();
        assert!(e.contains("above 0, at most 4"), "{e}");
    }

    #[test]
    fn colours_take_an_alpha() {
        assert_eq!(layer("draw = \"fill\"\ncolor = \"#00000040\"").unwrap().color, Some([0, 0, 0, 0x40]));
        assert_eq!(layer("draw = \"fill\"\ncolor = \"#ff8000\"").unwrap().color, Some([255, 128, 0, 255]));
        assert!(layer("draw = \"fill\"\ncolor = \"red\"").is_err());
        assert!(layer("draw = \"fill\"\ncolor = \"#ff0\u{e9}0\"").is_err(), "six bytes, not six digits");
    }

    #[test]
    fn no_layers_draws_a_plain_fill_and_joins_intern() {
        let mut groups = Vec::new();
        let (mut keys, mut glyphs) = (Vec::new(), Vec::new());
        let mut sp = Art { sprites: &mut keys, glyphs: &mut glyphs, home: "m" };
        let a = LookDef { join: Some("wall".into()), ..Default::default() }.compile(&mut groups, &mut sp).unwrap();
        let b = LookDef { join: Some("wall".into()), ..Default::default() }.compile(&mut groups, &mut sp).unwrap();
        assert_eq!(a.layers, plain());
        assert_eq!((a.join, b.join, groups.len()), (Some(0), Some(0), 1));
    }
}
