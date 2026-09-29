//! A colony being played: the App built from an opened game, and its frame
//! loop, until the player leaves for the title screen or quits.

use crate::*;
use rim_sim::savefile::Writer;

/// Where the player's files are, each none in the tests.
pub struct Files {
    pub settings: Option<PathBuf>,
    pub layout: Option<PathBuf>,
    pub keys: Option<PathBuf>,
}

/// Play `game` until the player leaves it, then hand the UI back for the
/// title screen. The autotest and the benchmark exit instead of returning.
pub async fn run(
    game: (Sim, Option<Writer>, Vec<String>),
    mut ui: Ui,
    atlas: Texture2D,
    wheel_sub: usize,
    args: &[String],
    files: &Files,
) -> Result<Ui, String> {
    let (mut sim, saver, notes) = game;
    sim.warnings.extend(notes);
    eprintln!("rim: seed {}, {} mods loaded, UI font {}", sim.world.seed, sim.mods.len(), ui.info.font);
    if saver.is_some() {
        // Closing the window takes a last snapshot first.
        prevent_quit();
    }
    sim.warnings.extend(figures::unbodied(&ui.bodies, &sim.world.defs));
    for w in sim.warnings.iter().chain(&ui.warnings()) {
        eprintln!("  warning: {w}");
    }

    let world_atlas = atlas::WorldAtlas::load(&sim.world.defs.sprite_files, &sim.world.defs.glyphs, &mut ui.text)?;
    let center = sim.world.colony_center().unwrap_or(IVec::new(100, 100));
    let palette = overlay::Palette::from_theme(&ui.theme);
    let (scroll_mode, reduce_motion, lighting, render_scale) = prefs(args, files.settings.as_ref());
    let mut app = App {
        tools: toolbar(&sim),
        sim,
        ui,
        atlas,
        cam: Cam { x: center.x as f32 + 0.5, y: center.y as f32 + 0.5, zoom: 28.0, z: 0 },
        tool: Tool::Select,
        selected: None,
        selected_zone: None,
        group: Vec::new(),
        shift: false,
        subtract: false,
        drag_start: None,
        drag_from: (0.0, 0.0),
        paused: false,
        speed: 1,
        show_profiler: false,
        show_devtools: false,
        frames: Default::default(),
        overlay: None,
        storage_overlay: false,
        hint: None,
        stuff_for: Vec::new(),
        preview: None,
        build_facing: 0,
        hint_key: None,
        order_flash: None,
        mouse_over_ui: false,
        hover_cell: None,
        last_draw: Vec::new(),
        last_anchored: Vec::new(),
        profile: (Vec::new(), Vec::new(), f64::MIN),
        acc: 0.0,
        pan_anchor: None,
        right: None,
        scroll_mode,
        last_precise: f64::MIN,
        last_order: None,
        sky: sky::Sky::default(),
        roofs: roof::Roofs::default(),
        marks: roomstate::RoomMarks::default(),
        light: light::Light::with(lighting),
        ground: draw::Ground::default(),
        water: draw::Water::default(),
        render_us: RenderTimes::default(),
        meshes: mesh::Meshes::default(),
        worksites: worksite::Worksites::default(),
        motion: motion::Motion::default(),
        figures: figures::Batch::default(),
        figures_shown: Vec::new(),
        world_atlas,
        render_scale,
        world_target: None,
        blit: None,
        fade: None,
        drawn_z: 0,
        fade_blit: None,
        settings_file: files.settings.clone(),
        leaving: false,
        wheel_sub,
        saver,
        palette,
        chalk: overlay::State::default(),
        grid: grid::Grid::default(),
        pointer: (0.0, 0.0),
        dragged: false,
        measure: false,
        reduce_motion,
        zone_preview: None,
        order_preview: None,
        build_preview: None,
        refused: None,
        now: 0.0,
        dt: 0.0,
    };
    app.selected = app.sim.world.colonists().next();

    if let Some(i) = args.iter().position(|a| a == "--autotest") {
        let dir = args.get(i + 1).filter(|a| !a.starts_with("--")).map_or("target/autotest".into(), PathBuf::from);
        autotest::run(app, dir).await;
    }
    if args.iter().any(|a| a == "--bench-render") {
        bench::run(app, args).await;
    }

    loop {
        if is_quit_requested() {
            close(&mut app);
            std::process::exit(0);
        }
        let raw = RawInput::gather(&mut app);
        frame(&mut app, &raw);
        if app.ui.take_layout_dirty() {
            if let Some(p) = &files.layout {
                let _ = p.parent().map(std::fs::create_dir_all);
                if let Err(e) = std::fs::write(p, app.ui.layout_toml()) {
                    eprintln!("rim: could not save the UI layout to {}: {e}", p.display());
                }
            }
        }
        if app.ui.take_keys_dirty() {
            if let Some(p) = &files.keys {
                let _ = p.parent().map(std::fs::create_dir_all);
                if let Err(e) = std::fs::write(p, app.ui.keybinds_toml()) {
                    eprintln!("rim: could not save the keybinds to {}: {e}", p.display());
                }
            }
        }
        // After the layout and keys are written: the title may be the
        // last thing the player sees.
        if app.leaving {
            close(&mut app);
            return Ok(app.ui);
        }
        render(&mut app);
        next_frame().await
    }
}

/// Save and close the game being left: a last snapshot, as quitting takes.
pub fn close(app: &mut App) {
    if let Some(w) = app.saver.take() {
        save::close(w, &mut app.sim);
    }
}

/// The player's settings a game starts with: what a scroll does, reduce
/// motion, lighting and render scale. Read from the file for each game, so
/// a change made in the last one holds; the command line and the tests
/// override as they always have.
fn prefs(args: &[String], file: Option<&PathBuf>) -> (ScrollMode, bool, quality::Setting, Option<f32>) {
    let tests = args.iter().any(|a| a == "--autotest" || a == "--bench-render");
    let settings = crate::read_settings(file);
    // What a scroll does on the map: the player's setting, else sorted by
    // device. The tests read devices as they come.
    let scroll_mode = settings
        .as_deref()
        .and_then(|text| {
            saved_scroll_mode(text).unwrap_or_else(|e| {
                eprintln!("  warning: settings file: {e}");
                None
            })
        })
        .unwrap_or_default();
    let reduce_motion =
        settings.as_deref().and_then(|text| setting(saved_flag(text, "reduce_motion"))).unwrap_or(false);
    // `--lighting <preset>` for the bench and tests; else the settings file.
    let lighting = match args.windows(2).find(|w| w[0] == "--lighting") {
        Some(w) => quality::Setting::named(&w[1]).unwrap_or_else(|| {
            eprintln!("  warning: --lighting wants low, medium, high, ultra or auto, not {}", w[1]);
            quality::Setting::default()
        }),
        None => settings
            .as_deref()
            .and_then(|text| {
                quality::Setting::from_settings(text).unwrap_or_else(|e| {
                    eprintln!("  warning: settings file: {e}");
                    None
                })
            })
            .unwrap_or_default(),
    };
    let render_scale = if tests {
        // They measure full resolution unless they ask.
        Some(1.0)
    } else {
        match settings.as_deref() {
            Some(text) => saved_render_scale(text).unwrap_or_else(|e| {
                eprintln!("  warning: settings file: {e}");
                None
            }),
            None => None,
        }
    };
    (scroll_mode, reduce_motion, lighting, render_scale)
}
