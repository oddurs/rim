//! How much the lighting does, and so what it costs (DESIGN.md §6e).
//!
//! Two settings, from `[lighting] quality` in the player's settings file.
//! `flat`, the default, lights the world by the sim's light field in one
//! multiply. `shadows`, the one richer look a player may choose, adds sun,
//! moon and firelight shadows at a GPU cost (08a5d182). Either way a light
//! texel never gets smaller than `MIN_TEXEL_PX` screen pixels, so zooming
//! out doesn't raise the cost.

/// What the lighting does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    /// The sim's light field as it is: no marches, bakes or fill.
    #[default]
    Flat,
    /// Shadows from the brightest sky body, firelight baked with soft
    /// shadows, moving lights, and rooms filled by their lights.
    Shadows,
}

/// Under `shadows`: light texels per cell, at most; fewer when zoomed out.
pub const TEXELS: u32 = 2;
/// Steps the sky's shadows are traced, 0.4 cells each.
pub const SUN_STEPS: u32 = 28;
/// How far a sky body moves, in degrees, before its shadows are worked out
/// again.
pub const SUN_REBUILD: f64 = 0.25;
/// Moving lights that cast shadows, nearest the view first; the rest glow
/// without.
pub const MOVING_SHADOWS: usize = 8;

/// The fewest screen pixels a light texel may cover.
pub const MIN_TEXEL_PX: f32 = 4.0;

impl Setting {
    /// A setting by name: `flat` or `shadows`.
    pub fn named(name: &str) -> Option<Setting> {
        match name {
            "flat" => Some(Setting::Flat),
            "shadows" => Some(Setting::Shadows),
            _ => None,
        }
    }

    /// The `[lighting]` table of a settings file, if it has one. A table
    /// that doesn't parse is an error to report, not a default. Tuning an
    /// older file carried (`sun_steps` and the like, gone with 08a5d182)
    /// is ignored, so an upgrade keeps the player's choice, and the next
    /// save drops it.
    pub fn from_settings(text: &str) -> Result<Option<Setting>, String> {
        let t: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let Some(v) = t.get("lighting") else { return Ok(None) };
        let l = v.as_table().ok_or("lighting should be a table: [lighting]")?;
        let Some(q) = l.get("quality") else { return Ok(None) };
        let name = q.as_str().ok_or("lighting.quality should be a string")?;
        Setting::named(name).map(Some).ok_or_else(|| format!("lighting.quality should be flat or shadows, not {name}"))
    }

    pub fn name(&self) -> &'static str {
        match self {
            Setting::Flat => "flat",
            Setting::Shadows => "shadows",
        }
    }

    pub fn flat(&self) -> bool {
        *self == Setting::Flat
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
    fn flat_is_the_default_and_shadows_the_one_choice() {
        assert_eq!(Setting::default(), Setting::Flat);
        assert_eq!(Setting::from_settings("render_scale = 0.5"), Ok(None), "unset is the default");
        assert_eq!(Setting::from_settings("[lighting]"), Ok(None));
        for s in [Setting::Flat, Setting::Shadows] {
            let text = format!("render_scale = 0.5\n\n[lighting]\nquality = \"{}\"\n", s.name());
            assert_eq!(Setting::from_settings(&text), Ok(Some(s)), "{text}");
        }
        // An older file: the tiers that went with 08a5d182 are errors, so
        // it says so and runs flat; its tuning is ignored, and a quality
        // beside it still holds.
        for gone in ["medium", "auto", "ultra"] {
            assert!(Setting::from_settings(&format!("[lighting]\nquality = \"{gone}\"")).is_err(), "{gone}");
        }
        let old = "[lighting]\nquality = \"shadows\"\nsun_steps = 40\ntexels_per_cell = 4";
        assert_eq!(Setting::from_settings(old), Ok(Some(Setting::Shadows)));
        assert_eq!(Setting::from_settings("[lighting]\nsun_steps = 40"), Ok(None), "tuning alone is the default");
        assert!(Setting::from_settings("lighting = 2").is_err());
    }

    #[test]
    fn a_light_texel_never_gets_smaller_than_four_pixels() {
        // rim's minimum zoom is 4 points a cell.
        assert_eq!(texels_for(2, 4.0, 2), 1, "the minimum zoom is 1 texel a cell");
        assert_eq!(texels_for(4, 8.0, 4), 2);
        assert_eq!(texels_for(2, 28.0, 1), 2, "close up, as many as the setting allows");
        assert_eq!(texels_for(1, 80.0, 1), 1, "never more than the setting");
        // At the edge it holds what it has rather than flicking back and forth.
        assert_eq!(texels_for(2, 8.5, 1), 1, "growing needs room to spare");
        assert_eq!(texels_for(2, 8.5, 2), 2, "shrinking waits until it must");
    }
}
