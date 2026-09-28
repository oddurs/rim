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
    /// Moving lights that cast shadows, nearest the view first; the rest
    /// glow without.
    pub moving_shadows: u32,
    /// Sky bodies that cast shadows, brightest first, up to 4; the rest
    /// light without.
    pub sky_shadows: u32,
}

/// The presets, cheapest first.
pub const PRESETS: [(&str, Quality); 4] = [
    ("low", Quality { texels: 1, sun_steps: 16, soft: false, sun_rebuild: 1.0, moving_shadows: 4, sky_shadows: 1 }),
    ("medium", Quality { texels: 2, sun_steps: 28, soft: true, sun_rebuild: 0.25, moving_shadows: 8, sky_shadows: 1 }),
    ("high", Quality { texels: 2, sun_steps: 40, soft: true, sun_rebuild: 0.1, moving_shadows: 16, sky_shadows: 2 }),
    ("ultra", Quality { texels: 4, sun_steps: 56, soft: true, sun_rebuild: 0.02, moving_shadows: 32, sky_shadows: 4 }),
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
    /// `auto`: start at medium and step down while the lighting runs slow.
    pub auto: bool,
    /// What the player set by hand, kept over whichever preset runs.
    overrides: Vec<Override>,
}

impl Default for Setting {
    fn default() -> Self {
        Setting { preset: MEDIUM, quality: PRESETS[MEDIUM].1, auto: false, overrides: Vec::new() }
    }
}

/// One setting the player set by hand under `[lighting]`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Override {
    Texels(u32),
    SunSteps(u32),
    Soft(bool),
    MovingShadows(u32),
    SkyShadows(u32),
    SunRebuild(f64),
}

impl Override {
    fn apply(self, q: &mut Quality) {
        match self {
            Override::Texels(n) => q.texels = n,
            Override::SunSteps(n) => q.sun_steps = n,
            Override::Soft(b) => q.soft = b,
            Override::MovingShadows(n) => q.moving_shadows = n,
            Override::SkyShadows(n) => q.sky_shadows = n,
            Override::SunRebuild(d) => q.sun_rebuild = d,
        }
    }
}

impl Setting {
    /// A preset by name: `low`, `medium`, `high`, `ultra`, or `auto`.
    pub fn named(name: &str) -> Option<Setting> {
        let auto = name == "auto";
        let preset = if auto { MEDIUM } else { PRESETS.iter().position(|(n, _)| *n == name)? };
        Some(Setting { preset, quality: PRESETS[preset].1, auto, overrides: Vec::new() })
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
            .ok_or_else(|| format!("lighting.quality should be low, medium, high, ultra or auto, not {name}"))?;
        for (key, v) in l {
            let int = || v.as_integer().ok_or_else(|| format!("lighting.{key} should be a whole number"));
            let flag = || v.as_bool().ok_or_else(|| format!("lighting.{key} should be true or false"));
            s.overrides.push(match key.as_str() {
                "quality" => continue,
                "texels_per_cell" => Override::Texels(match int()? {
                    n @ (1 | 2 | 4) => n as u32,
                    n => return Err(format!("lighting.texels_per_cell should be 1, 2 or 4, not {n}")),
                }),
                "sun_steps" => Override::SunSteps(int()?.clamp(4, 64) as u32),
                "soft_shadows" => Override::Soft(flag()?),
                "moving_shadows" => Override::MovingShadows(int()?.clamp(0, 64) as u32),
                "sky_shadows" => Override::SkyShadows(int()?.clamp(0, 4) as u32),
                "sun_rebuild_degrees" => {
                    let d = v.as_float().or(v.as_integer().map(|i| i as f64)).filter(|d| d.is_finite());
                    Override::SunRebuild(d.ok_or("lighting.sun_rebuild_degrees should be a number")?.clamp(0.01, 5.0))
                }
                _ => return Err(format!("lighting has no setting {key}")),
            });
        }
        s.set_preset(s.preset);
        Ok(Some(s))
    }

    /// The preset that runs: under `auto`, the one it has stepped down to.
    pub fn name(&self) -> &'static str {
        PRESETS[self.preset].0
    }

    /// Run `preset`, with what the player set by hand still over it.
    fn set_preset(&mut self, preset: usize) {
        self.preset = preset;
        self.quality = PRESETS[preset].1;
        for o in &self.overrides {
            o.apply(&mut self.quality);
        }
    }

    /// Under `auto`, a frame's lighting took `us` µs on the GPU at `preset`,
    /// at `now` seconds. Steps down a preset when a window of such frames
    /// ran over budget, and returns the window's mean when it does.
    pub fn watch(&mut self, watch: &mut Watch, now: f64, preset: usize, us: f64) -> Option<f64> {
        // A frame timed before a step measured the preset before it.
        if !self.auto || preset != self.preset || self.preset == 0 {
            return None;
        }
        let mean = watch.over(now, us)?;
        self.set_preset(self.preset - 1);
        Some(mean)
    }
}

/// The GPU time `auto` allows the lighting, µs a frame: an eighth of a
/// 60 Hz frame.
pub const AUTO_BUDGET_US: f64 = 2000.0;
/// How long `auto` watches before it judges, seconds.
pub const AUTO_WINDOW: f64 = 3.0;
/// And the fewest frames it judges on, however slow they come.
const AUTO_FRAMES: u32 = 30;

/// `auto`'s window: the lighting's GPU time, frame by frame. It sees
/// nothing else, so a slow sim or UI never turns the lights down.
#[derive(Clone, Debug, Default)]
pub struct Watch {
    since: Option<f64>,
    last: f64,
    sum: f64,
    frames: u32,
}

impl Watch {
    /// Add a frame's lighting, `us` µs at `now` s. When that closes a
    /// window, starts the next and returns the closed one's mean if it ran
    /// over budget.
    fn over(&mut self, now: f64, us: f64) -> Option<f64> {
        // Frames stopped coming for a window (a pause, a GPU far behind):
        // what came before says nothing about what comes now.
        if now - self.last > AUTO_WINDOW {
            *self = Watch::default();
        }
        self.last = now;
        let since = *self.since.get_or_insert(now);
        self.sum += us;
        self.frames += 1;
        if now - since < AUTO_WINDOW || self.frames < AUTO_FRAMES {
            return None;
        }
        let mean = self.sum / self.frames as f64;
        *self = Watch::default();
        (mean > AUTO_BUDGET_US).then_some(mean)
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
        let want =
            Quality { texels: 4, sun_steps: 64, soft: false, sun_rebuild: 1.0, moving_shadows: 16, sky_shadows: 2 };
        assert_eq!(s.quality, want, "steps are capped");
        let s = Setting::from_settings("[lighting]\nmoving_shadows = 2").unwrap().unwrap();
        assert_eq!(s.quality.moving_shadows, 2, "and the moving lights' cap");
        let s = Setting::from_settings("[lighting]\nsky_shadows = 9").unwrap().unwrap();
        assert_eq!(s.quality.sky_shadows, 4, "and the sky's, four at most");
        let s = Setting::from_settings("[lighting]\nsun_rebuild_degrees = 0.5").unwrap().unwrap();
        assert_eq!((s.name(), s.quality.sun_rebuild), ("medium", 0.5), "no preset is medium");
        assert_eq!(Setting::from_settings("render_scale = 0.5"), Ok(None), "unset is the default");
        let s = Setting::from_settings("[lighting]\nquality = \"auto\"\nsun_steps = 40").unwrap().unwrap();
        assert!(s.auto && s.name() == "medium" && s.quality.sun_steps == 40, "auto starts at medium: {s:?}");
        assert!(Setting::from_settings("[lighting]\nquality = \"shiny\"").is_err());
        assert!(Setting::from_settings("[lighting]\ntexels_per_cell = 3").is_err());
        assert!(Setting::from_settings("[lighting]\nbicubic = true").is_err(), "and bicubic is no longer a setting");
        assert!(Setting::from_settings("lighting = 2").is_err());
    }

    /// Frames at `fps` for `secs` seconds from `t`, each with `us` of
    /// lighting on the GPU: every mean `auto` stepped down on.
    fn run(s: &mut Setting, w: &mut Watch, t: &mut f64, fps: f64, secs: f64, us: f64) -> Vec<f64> {
        let mut steps = Vec::new();
        let end = *t + secs;
        while *t < end {
            steps.extend(s.watch(w, *t, s.preset, us));
            *t += 1.0 / fps;
        }
        steps
    }

    #[test]
    fn auto_steps_down_a_preset_a_window_while_the_lighting_runs_slow_and_stops_at_low() {
        let (mut w, mut t) = (Watch::default(), 0.0);
        let mut s = Setting::from_settings("[lighting]\nquality = \"auto\"").unwrap().unwrap();
        s.set_preset(3);
        let slow = AUTO_BUDGET_US * 2.0;
        let steps = run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 0.9, slow);
        assert_eq!((s.name(), steps.len()), ("ultra", 0), "not before a window has passed");
        run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 0.2, slow);
        assert_eq!(s.name(), "high", "then one preset");
        run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW, slow);
        assert_eq!(s.name(), "medium", "one a window");
        let steps = run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 10.0, slow);
        assert_eq!((s.name(), steps.len()), ("low", 1), "and never past low");
        assert!(steps[0] > AUTO_BUDGET_US, "it steps on the mean it saw: {steps:?}");
        assert_eq!(s.quality, PRESETS[0].1);
    }

    #[test]
    fn auto_keeps_what_the_player_set_by_hand_as_it_steps() {
        let (mut w, mut t) = (Watch::default(), 0.0);
        let mut s = Setting::from_settings("[lighting]\nquality = \"auto\"\nsoft_shadows = true").unwrap().unwrap();
        run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 2.0, AUTO_BUDGET_US * 2.0);
        assert_eq!(s.name(), "low");
        assert!(s.quality.soft, "low's shadows are hard, but the player's aren't");
    }

    #[test]
    fn a_slow_frame_with_cheap_lighting_leaves_auto_where_it_is() {
        // A slow sim or UI makes frames few and far apart; the lighting in
        // each is still cheap, and that is all auto reads.
        let (mut w, mut t) = (Watch::default(), 0.0);
        let mut s = Setting::named("auto").unwrap();
        let steps = run(&mut s, &mut w, &mut t, 4.0, 120.0, AUTO_BUDGET_US * 0.3);
        assert_eq!((s.name(), steps), ("medium", vec![]));
        // A few slow frames in a window of cheap ones: a sun rebuild, a bake.
        let mut frame = 0;
        let mut steps = Vec::new();
        while t < 240.0 {
            let us = if frame % 60 == 0 { AUTO_BUDGET_US * 20.0 } else { AUTO_BUDGET_US * 0.3 };
            steps.extend(s.watch(&mut w, t, s.preset, us));
            (t, frame) = (t + 1.0 / 60.0, frame + 1);
        }
        assert_eq!((s.name(), steps), ("medium", vec![]), "a spike now and then is not slow");
    }

    #[test]
    fn a_gap_in_the_frames_starts_the_window_afresh() {
        // Slow frames, then none for a while, then a burst: the window
        // judges the burst alone, not the slow frames before the gap.
        let (mut w, mut t) = (Watch::default(), 0.0);
        let mut s = Setting::named("auto").unwrap();
        run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 0.5, AUTO_BUDGET_US * 10.0);
        t += AUTO_WINDOW * 2.0;
        let steps = run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 1.5, AUTO_BUDGET_US * 0.3);
        assert_eq!((s.name(), steps), ("medium", vec![]));
    }

    #[test]
    fn auto_ignores_frames_timed_at_another_preset_and_a_chosen_preset_never_steps() {
        let (mut w, mut t) = (Watch::default(), 0.0);
        let mut s = Setting::named("auto").unwrap();
        for _ in 0..600 {
            assert_eq!(s.watch(&mut w, t, 2, AUTO_BUDGET_US * 5.0), None, "timed before a step");
            t += 1.0 / 60.0;
        }
        assert_eq!(s.name(), "medium");
        let mut s = Setting::named("high").unwrap();
        run(&mut s, &mut w, &mut t, 60.0, AUTO_WINDOW * 5.0, AUTO_BUDGET_US * 5.0);
        assert_eq!(s.name(), "high", "only auto steps");
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
