//! How much the lighting does, and so what it costs (DESIGN.md §6e).
//!
//! Four presets and per-setting overrides, read from `[lighting]` in the
//! player's settings file. Whatever the preset, a light texel never gets
//! smaller than `MIN_TEXEL_PX` screen pixels, so zooming out doesn't raise
//! the cost.

/// What the lighting does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    /// Light texels per cell, at most: fewer when zoomed out.
    pub texels: u32,
    /// Steps the sun's shadows are traced, 0.4 cells each.
    pub sun_steps: u32,
    /// Sun shadows soften with distance from what casts them.
    pub soft: bool,
    /// How far the sun moves, in degrees, before its shadows are worked out
    /// again.
    pub sun_rebuild: f64,
}

/// The presets, cheapest first.
pub const PRESETS: [(&str, Quality); 4] = [
    ("low", Quality { texels: 1, sun_steps: 16, soft: false, sun_rebuild: 1.0 }),
    ("medium", Quality { texels: 2, sun_steps: 28, soft: true, sun_rebuild: 0.25 }),
    ("high", Quality { texels: 2, sun_steps: 40, soft: true, sun_rebuild: 0.1 }),
    ("ultra", Quality { texels: 4, sun_steps: 56, soft: true, sun_rebuild: 0.02 }),
];

/// The default.
pub const MEDIUM: usize = 1;

/// The fewest screen pixels a light texel may cover.
pub const MIN_TEXEL_PX: f32 = 4.0;

impl Default for Quality {
    fn default() -> Self {
        PRESETS[MEDIUM].1
    }
}

/// The player's lighting setting: a preset and overrides.
#[derive(Clone, Debug, PartialEq)]
pub struct Setting {
    /// Index into `PRESETS`.
    pub preset: usize,
    pub quality: Quality,
}

impl Default for Setting {
    fn default() -> Self {
        Setting { preset: MEDIUM, quality: PRESETS[MEDIUM].1 }
    }
}

impl Setting {
    /// A preset by name: `low`, `medium`, `high` or `ultra`.
    pub fn named(name: &str) -> Option<Setting> {
        let preset = PRESETS.iter().position(|(n, _)| *n == name)?;
        Some(Setting { preset, quality: PRESETS[preset].1 })
    }

    /// The `[lighting]` table of a settings file, if it has one. A table
    /// that doesn't parse is an error to report, not a default.
    pub fn from_settings(text: &str) -> Result<Option<Setting>, String> {
        let t: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let Some(v) = t.get("lighting") else { return Ok(None) };
        let l = v.as_table().ok_or("lighting should be a table: [lighting]")?;
        let name = match l.get("quality") {
            None => "medium",
            Some(q) => q.as_str().ok_or("lighting.quality should be a string")?,
        };
        let mut s = Setting::named(name)
            .ok_or_else(|| format!("lighting.quality should be low, medium, high or ultra, not {name}"))?;
        let q = &mut s.quality;
        for (key, v) in l {
            let int = || v.as_integer().ok_or_else(|| format!("lighting.{key} should be a whole number"));
            let flag = || v.as_bool().ok_or_else(|| format!("lighting.{key} should be true or false"));
            match key.as_str() {
                "quality" => {}
                "texels_per_cell" => {
                    q.texels = match int()? {
                        n @ (1 | 2 | 4) => n as u32,
                        n => return Err(format!("lighting.texels_per_cell should be 1, 2 or 4, not {n}")),
                    }
                }
                "sun_steps" => q.sun_steps = int()?.clamp(4, 64) as u32,
                "soft_shadows" => q.soft = flag()?,
                "sun_rebuild_degrees" => {
                    let d = v.as_float().or(v.as_integer().map(|i| i as f64)).filter(|d| d.is_finite());
                    q.sun_rebuild = d.ok_or("lighting.sun_rebuild_degrees should be a number")?.clamp(0.01, 5.0);
                }
                _ => return Err(format!("lighting has no setting {key}")),
            }
        }
        Ok(Some(s))
    }

    /// The preset's name.
    pub fn name(&self) -> &'static str {
        PRESETS[self.preset].0
    }
}

/// Light texels per cell for a view at `px_per_cell` screen pixels a cell:
/// as many as `max` allows while each covers at least `MIN_TEXEL_PX`, and
/// not changed back and forth at the edge: growing needs a fifth to spare.
pub fn texels_for(max: u32, px_per_cell: f32, current: u32) -> u32 {
    let fits = |t: u32| px_per_cell / t as f32 >= MIN_TEXEL_PX;
    let mut t = current.clamp(1, max);
    while t > 1 && !fits(t) {
        t /= 2;
    }
    while t * 2 <= max && px_per_cell / (t * 2) as f32 >= MIN_TEXEL_PX * 1.2 {
        t *= 2;
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_overrides_are_read_from_the_settings_file() {
        for (name, quality) in PRESETS {
            let text = format!("render_scale = 0.5\n\n[lighting]\nquality = \"{name}\"\n");
            let s = Setting::from_settings(&text).unwrap().unwrap();
            assert_eq!((s.name(), s.quality), (name, quality), "{text}");
        }
        let text = "[lighting]\nquality = \"high\"\ntexels_per_cell = 4\nsun_steps = 90\nsoft_shadows = false\nsun_rebuild_degrees = 1\n";
        let s = Setting::from_settings(text).unwrap().unwrap();
        assert_eq!(s.name(), "high", "overrides keep the preset's name");
        assert_eq!(s.quality, Quality { texels: 4, sun_steps: 64, soft: false, sun_rebuild: 1.0 }, "steps are capped");
        let s = Setting::from_settings("[lighting]\nsun_rebuild_degrees = 0.5").unwrap().unwrap();
        assert_eq!((s.name(), s.quality.sun_rebuild), ("medium", 0.5), "no preset is medium");
        assert_eq!(Setting::from_settings("render_scale = 0.5"), Ok(None), "unset is the default");
        assert!(Setting::from_settings("[lighting]\nquality = \"auto\"").is_err(), "auto is not a preset yet");
        assert!(Setting::from_settings("[lighting]\nquality = \"shiny\"").is_err());
        assert!(Setting::from_settings("[lighting]\ntexels_per_cell = 3").is_err());
        assert!(Setting::from_settings("[lighting]\nbicubic = true").is_err(), "and bicubic is no longer a setting");
        assert!(Setting::from_settings("lighting = 2").is_err());
    }

    #[test]
    fn a_light_texel_never_gets_smaller_than_four_pixels() {
        // rim's minimum zoom is 4 points a cell.
        assert_eq!(texels_for(2, 4.0, 2), 1, "the minimum zoom is 1 texel a cell");
        assert_eq!(texels_for(4, 8.0, 4), 2);
        assert_eq!(texels_for(2, 28.0, 1), 2, "close up, as many as the preset allows");
        assert_eq!(texels_for(1, 80.0, 1), 1, "never more than the preset");
        // At the edge it holds what it has rather than flicking back and forth.
        assert_eq!(texels_for(2, 8.5, 1), 1, "growing needs room to spare");
        assert_eq!(texels_for(2, 8.5, 2), 2, "shrinking waits until it must");
    }
}
