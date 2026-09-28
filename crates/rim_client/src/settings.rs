//! The player's settings file and per-player files (DESIGN.md §11): where
//! they live, and reading and writing each setting, a bad value reported
//! and left at its default.

use super::*;

/// Where a per-player file is kept: the platform's data directory, so it
/// follows the player, not the save.
pub(crate) fn player_file(name: &str) -> Option<PathBuf> {
    layout_path().map(|p| p.with_file_name(name))
}

/// Where the UI layout is kept: the platform's per-user data directory,
/// so it follows the player, not the save.
pub(crate) fn layout_path() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else if cfg!(windows) {
        var("APPDATA")
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("rim").join("ui-layout.toml"))
}

/// What a scroll does on the map. Auto sorts each event by what made it;
/// the others are for devices that look like the other kind (a mouse with
/// a free-spinning smooth wheel reads as a trackpad).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollMode {
    #[default]
    Auto,
    Zoom,
    Pan,
}

impl ScrollMode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            ScrollMode::Auto => "auto",
            ScrollMode::Zoom => "zoom",
            ScrollMode::Pan => "pan",
        }
    }
    pub(crate) fn parse(s: &str) -> Option<ScrollMode> {
        [ScrollMode::Auto, ScrollMode::Zoom, ScrollMode::Pan].into_iter().find(|m| m.name() == s)
    }
}

/// A setting as read from the settings file, or none: one it gets wrong
/// is reported and left at its default.
pub(crate) fn setting<T>(r: Result<Option<T>, String>) -> Option<T> {
    r.unwrap_or_else(|e| {
        eprintln!("  warning: settings file: {e}");
        None
    })
}

/// A yes-or-no setting, if the settings file holds it.
pub(crate) fn saved_flag(text: &str, key: &str) -> Result<Option<bool>, String> {
    let t: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    match t.get(key) {
        None => Ok(None),
        Some(v) => v.as_bool().map(Some).ok_or_else(|| format!("{key} should be true or false, not {v}")),
    }
}

/// The scroll mode a settings file holds, if any.
pub(crate) fn saved_scroll_mode(text: &str) -> Result<Option<ScrollMode>, String> {
    let t: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    match t.get("scroll") {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .and_then(ScrollMode::parse)
            .map(Some)
            .ok_or_else(|| format!("scroll should be \"auto\", \"zoom\" or \"pan\", not {v}")),
    }
}

/// A render scale the world can draw at: 0.25 to 1, or nothing.
pub(crate) fn valid_render_scale(s: f64) -> Option<f32> {
    s.is_finite().then(|| (s as f32).clamp(0.25, 1.0))
}

/// The render scale a settings file holds, if any. A file or value that
/// doesn't parse is an error to report, not a default.
pub(crate) fn saved_render_scale(text: &str) -> Result<Option<f32>, String> {
    saved_scale(text, "render_scale", valid_render_scale, "0.25 to 1")
}

/// The UI scale a settings file holds, if any.
pub(crate) fn saved_ui_scale(text: &str) -> Result<Option<f32>, String> {
    let valid = |s: f64| s.is_finite().then(|| (s as f32).clamp(rim_ui::vm::UI_SCALE.0, rim_ui::vm::UI_SCALE.1));
    saved_scale(text, "ui_scale", valid, "0.75 to 2")
}

pub(crate) fn saved_scale(
    text: &str,
    key: &str,
    valid: impl Fn(f64) -> Option<f32>,
    range: &str,
) -> Result<Option<f32>, String> {
    let t: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_float()
            .or(v.as_integer().map(|i| i as f64))
            .and_then(valid)
            .map(Some)
            .ok_or_else(|| format!("{key} should be a number from {range}, not {v}")),
    }
}

/// Set one key in the settings file, keeping the rest, through a
/// temporary file so a crash mid-write can't leave half a file.
pub(crate) fn save_setting(path: &std::path::Path, key: &str, value: toml::Value) -> Result<(), String> {
    edit_settings(path, |t| {
        t.insert(key.to_string(), value);
    })
    .map(|_| ())
}

/// Save the lighting preset as `[lighting] quality`, keeping whatever else
/// the player set by hand there. The file's new text.
pub(crate) fn save_lighting(path: &std::path::Path, preset: &str) -> Result<String, String> {
    edit_settings(path, |t| {
        let quality = toml::Value::String(preset.to_string());
        match t.get_mut("lighting").and_then(|l| l.as_table_mut()) {
            Some(l) => {
                l.insert("quality".to_string(), quality);
            }
            None => {
                t.insert("lighting".to_string(), toml::Table::from_iter([("quality".to_string(), quality)]).into());
            }
        }
    })
}

/// Change the player's settings file with `edit`, keeping everything else
/// in it, and write it whole or not at all. The file's new text.
pub(crate) fn edit_settings(path: &std::path::Path, edit: impl FnOnce(&mut toml::Table)) -> Result<String, String> {
    let mut t: toml::Table = match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => toml::Table::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    edit(&mut t);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = toml::to_string(&t).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, &text)
        .and_then(|_| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scroll_mode_round_trips() {
        assert_eq!(saved_scroll_mode("scroll = \"pan\""), Ok(Some(ScrollMode::Pan)));
        assert_eq!(saved_scroll_mode("ui_scale = 1.5"), Ok(None));
        assert!(saved_scroll_mode("scroll = \"sideways\"").is_err());
        assert!(saved_scroll_mode("scroll = 3").is_err());
    }

    #[test]
    fn reduce_motion_is_a_yes_or_no() {
        assert_eq!(saved_flag("reduce_motion = true", "reduce_motion"), Ok(Some(true)));
        assert_eq!(saved_flag("reduce_motion = false", "reduce_motion"), Ok(Some(false)));
        assert_eq!(saved_flag("render_scale = 0.5", "reduce_motion"), Ok(None));
        assert!(saved_flag("reduce_motion = \"yes\"", "reduce_motion").is_err());
        assert!(saved_flag("reduce_motion = 1", "reduce_motion").is_err());
    }

    #[test]
    fn the_render_scale_round_trips_and_a_bad_one_is_reported() {
        assert_eq!(saved_render_scale(&format!("render_scale = {}\n", 0.75f32)), Ok(Some(0.75)));
        assert_eq!(saved_render_scale("render_scale = 1"), Ok(Some(1.0)));
        assert_eq!(saved_render_scale("render_scale = 0.1"), Ok(Some(0.25)), "clamped to what draws");
        assert_eq!(saved_render_scale("vsync = true"), Ok(None), "unset is full");
        assert!(saved_render_scale("render_scale = \"half\"").is_err());
        assert!(saved_render_scale("render_scale = nan").is_err());
        assert!(saved_render_scale("render_scale = ").is_err());
    }

    #[test]
    fn the_ui_scale_round_trips_clamped() {
        assert_eq!(saved_ui_scale("ui_scale = 1.25"), Ok(Some(1.25)));
        assert_eq!(saved_ui_scale("ui_scale = 5"), Ok(Some(2.0)), "clamped");
        assert_eq!(saved_ui_scale("render_scale = 0.5"), Ok(None));
        assert!(saved_ui_scale("ui_scale = \"big\"").is_err());
        let p = std::env::temp_dir().join(format!("rim-ui-scale-{}.toml", std::process::id()));
        std::fs::write(&p, "render_scale = 0.5\n").unwrap();
        save_setting(&p, "ui_scale", toml::Value::Float(1.5)).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!((saved_ui_scale(&text), saved_render_scale(&text)), (Ok(Some(1.5)), Ok(Some(0.5))));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn saving_a_setting_keeps_the_others() {
        let p = std::env::temp_dir().join(format!("rim-settings-{}.toml", std::process::id()));
        std::fs::write(&p, "# mine\nvsync = true\n").unwrap();
        save_setting(&p, "render_scale", toml::Value::Float(0.5)).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("vsync = true") && text.contains("render_scale = 0.5"), "{text}");
        assert_eq!(saved_render_scale(&text), Ok(Some(0.5)));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn a_lighting_preset_is_saved_and_the_players_own_settings_stay() {
        let p = std::env::temp_dir().join(format!("rim-lighting-{}.toml", std::process::id()));
        std::fs::write(&p, "vsync = true\n\n[lighting]\nquality = \"low\"\nsun_steps = 40\n").unwrap();
        let text = save_lighting(&p, "ultra").unwrap();
        assert_eq!(text, std::fs::read_to_string(&p).unwrap(), "what it returns is what it wrote");
        let s = crate::quality::Setting::from_settings(&text).unwrap().unwrap();
        assert_eq!((s.name(), s.quality.sun_steps, s.quality.texels), ("ultra", 40, 4), "the override stays");
        assert!(text.contains("vsync = true"), "{text}");
        let _ = std::fs::remove_file(&p);
        // No file yet, or no [lighting] in it: made.
        let text = save_lighting(&p, "high").unwrap();
        assert_eq!(crate::quality::Setting::from_settings(&text).unwrap().map(|s| s.name()), Some("high"));
        let _ = std::fs::remove_file(&p);
    }
}
