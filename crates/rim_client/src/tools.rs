//! The toolbar's tools, and the view of the client the interface reads.

use super::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Designate(DefId),
    Build(DefId),
    /// Place a house plan whole (DESIGN.md §6c), its corner under the
    /// pointer, turned with T as a build is.
    Plan(DefId),
    /// Save what stands in a rectangle as a house plan (DESIGN.md §6c).
    SavePlan,
    /// Paint a stockpile: extends the one zone a drag touches, else a new one.
    Stockpile,
    /// Paint a growing zone of this plant: extends the one field of it a
    /// drag touches, else a new one.
    Grow(DefId),
    /// Take cells out of their zone.
    ClearZone,
    Cancel,
}

/// A toolbar entry: generated from defs, addressed by a stable key so UI
/// scripts can name it ("designate:core:chop", "build:core:wall").
pub struct ToolDef {
    pub key: String,
    pub label: String,
    pub tool: Tool,
    pub color: Color,
    /// The dock's category and group; see `ToolView`.
    pub category: &'static str,
    pub group: String,
    /// A buildable's work and hit points before material factors.
    pub work: u32,
    pub hp: u32,
}

/// Whether anything loaded can be marked with this designation. One that
/// nothing can is no use as a button: core names `gather` for plugins, and
/// has nothing of its own to gather.
pub fn markable(defs: &rim_sim::defs::DefDb, d: DefId) -> bool {
    defs.designations[d as usize].targets != Targets::Thing || defs.things.iter().any(|t| t.harvest_for(d).is_some())
}

/// The toolbar is generated from defs: a mod that adds a designation or a
/// buildable thing gets a button without touching the client.
/// Every tool: the orders, the buildables and the zone tools. The zone
/// tools take the theme's `zone` colour when drawn (`tool_colour`).
pub(crate) fn toolbar(sim: &Sim) -> Vec<ToolDef> {
    let defs = &sim.world.defs;
    let tool = |key: String, label: &str, tool, color, category, group: &str| ToolDef {
        key,
        label: label.into(),
        tool,
        color,
        category,
        group: group.into(),
        work: 0,
        hp: 0,
    };
    let mut items = vec![tool("select".into(), "Select", Tool::Select, GRAY, "", "")];
    for (i, d) in defs.designations.iter().enumerate().filter(|(i, _)| markable(defs, *i as DefId)) {
        items.push(tool(
            format!("designate:{}", d.id),
            &d.label,
            Tool::Designate(i as DefId),
            rgb(d.rgb),
            "orders",
            "",
        ));
    }
    items.push(tool("cancel".into(), "Cancel", Tool::Cancel, Color::from_rgba(200, 80, 80, 255), "orders", ""));
    let mut builds: Vec<(usize, &rim_sim::defs::ThingDef, &str)> =
        // What another work raises (a crop is sown) isn't built from the menu.
        defs.things
            .iter()
            .enumerate()
            .filter_map(|(i, t)| Some((i, t, t.build.as_ref().filter(|b| b.by.is_none())?.menu.as_str())))
            .collect();
    // Menus in the order their first thing is defined, so core's come first
    // and a mod's new menu lands after them.
    let mut menus: Vec<&str> = Vec::new();
    for &(_, _, menu) in &builds {
        if !menus.contains(&menu) {
            menus.push(menu);
        }
    }
    builds.sort_by_key(|&(i, _, menu)| (menus.iter().position(|m| *m == menu), i));
    for (i, t, menu) in builds {
        let b = t.build.as_ref().expect("a buildable");
        items.push(ToolDef {
            work: b.work,
            hp: t.hp,
            ..tool(format!("build:{}", t.id), &t.label, Tool::Build(i as DefId), rgb(t.rgb), "build", menu)
        });
    }
    // House plans, after the things they're built of, coloured like their
    // first piece.
    for (i, p) in defs.plans.iter().enumerate() {
        let first = p.pieces.first().map_or([128; 3], |pc| defs.thing(pc.stuff.unwrap_or(pc.thing)).rgb);
        items.push(tool(format!("plan:{}", p.id), &p.label, Tool::Plan(i as DefId), rgb(first), "build", "plans"));
    }
    let plan = Color::from_rgba(160, 200, 240, 255);
    items.push(tool("plan:save".into(), "Save as plan", Tool::SavePlan, plan, "build", "plans"));
    items.push(tool("stockpile".into(), "Stockpile", Tool::Stockpile, WHITE, "zones", ""));
    for (i, t) in defs.things.iter().enumerate().filter(|&(i, _)| defs.sowable(i as DefId)) {
        let label = format!("Grow {}", t.label);
        items.push(tool(format!("grow:{}", t.id), &label, Tool::Grow(i as DefId), WHITE, "zones", ""));
    }
    items.push(tool("clear_zone".into(), "Clear zone", Tool::ClearZone, WHITE, "zones", ""));
    items
}

/// A click with an order tool that would do nothing: say why, at the cell.
/// A drag that takes nothing is quiet; a click is a question.
pub(crate) fn refuse_if_idle(app: &mut App, designation: Option<DefId>, a: IVec, b: IVec) {
    if a != b {
        return;
    }
    // Asked of the cell released on: last frame's preview may be for
    // another, when the pointer moved and clicked in one frame.
    let op = overlay::OrderPreview::between(app, designation, a, b, false);
    if op.targets.is_empty() {
        app.refused = Some((op.refusal(app), a, app.chalk.now()));
    }
}

/// A tool's colour: its own, or for the zone tools the theme's `zone`.
pub fn tool_colour(app: &App, t: &ToolDef) -> Color {
    match t.tool {
        Tool::Stockpile | Tool::ClearZone => app.palette.zone,
        Tool::Grow(_) => GROW,
        _ => t.color,
    }
}

/// Growing zones: a field's green, set apart from a stockpile's blue.
pub const GROW: Color = Color::new(0.55, 0.8, 0.35, 1.0);

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

fn to_u8(c: Color) -> [u8; 3] {
    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8]
}

/// What the UI reads each frame.
pub fn client_view(app: &mut App, mouse: (f32, f32), time: f64) -> ClientView {
    let dpi = screen_dpi_scale();
    let s = &app.sim;
    // Profiler rows refresh a few times a second: a number changing every
    // frame would reflow the panel every frame.
    if app.show_profiler && time - app.profile.2 >= 0.25 {
        let w = &s.world;
        let (hooks, handlers) = s.scripts.hook_count();
        let mut rows = s.profile.entries.clone();
        for (m, us) in &app.ui.vm.mod_time {
            rows.push((format!("ui:{m}"), *us));
        }
        for (pass, us) in app.render_us.rows() {
            rows.push((format!("draw:{pass}"), us));
        }
        app.profile = (
            rows,
            vec![
                format!("tick {} · pawns {} · entities {}", w.tick, w.pawns.len(), w.ecs.len()),
                format!("paths {} · nodes {} · reservations {}", w.pf.searches, w.pf.expanded, w.reservations.len()),
                format!("script hooks {hooks} · handlers {handlers} · emitters {}", w.fields.emitter_count()),
                format!(
                    "chunk meshes: {} calls · {}k indices · {} rebuilt",
                    app.meshes.calls,
                    app.meshes.indices / 1000,
                    app.meshes.rebuilt
                ),
                format!("figures: {} calls · {} parts", app.figures.calls, app.figures.len()),
            ],
            time,
        );
    }
    let hover_cell = (!app.mouse_over_ui).then(|| app.cam.tile_at(mouse.0, mouse.1));
    let hover_pawn = if app.mouse_over_ui { None } else { pawn_under(app, mouse.0, mouse.1) };
    let drag = overlay::drag_hint(app);
    ClientView {
        screen: (screen_width() * dpi, screen_height() * dpi),
        scale: app.ui.theme.scale,
        cam: (app.cam.x, app.cam.y, app.cam.zoom * dpi),
        level: app.cam.z,
        frac: app.tick_frac(),
        came_from: app.motion.came_from(),
        selected: app.selected,
        selected_zone: app.selected_zone,
        group: app.group.clone(),
        shift: app.shift,
        paused: app.paused,
        speed: app.speed,
        overlay: app.overlay,
        storage_overlay: app.storage_overlay,
        show_profiler: app.show_profiler,
        show_devtools: app.show_devtools,
        tools: app
            .tools
            .iter()
            .map(|t| ToolView {
                key: t.key.clone(),
                label: t.label.clone(),
                color: to_u8(tool_colour(app, t)),
                active: t.tool == app.tool,
                category: t.category.into(),
                group: t.group.clone(),
                cost: match t.tool {
                    Tool::Build(d) => build_cost(app, d),
                    _ => String::new(),
                },
                work: t.work,
                hp: t.hp,
                locked: match t.tool {
                    Tool::Build(d) => app.sim.world.build_lock(d).unwrap_or_default(),
                    _ => String::new(),
                },
            })
            .collect(),
        stuff: stuff_view(app),
        hint: drag.or_else(|| app.hint.clone()),
        last_order: app.last_order.as_ref().map(|o| (o.label.clone(), app.now - o.at, !o.undo.is_empty())),
        hover_cell,
        hover_pawn,
        time,
        profile: app.profile.0.clone(),
        stats: app.profile.1.clone(),
        frame: app.frames.line().to_string(),
        mods: s.mods.iter().map(|m| (m.id.clone(), m.version.clone(), m.name.clone())).collect(),
        warnings: s.warnings.iter().cloned().chain(app.ui.warnings()).collect(),
        title: false,
        saves: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Core names `gather` for plugins but has nothing to gather, so its
    /// toolbar has no Gather button; chop and the rest stay.
    #[test]
    fn a_designation_nothing_can_be_marked_for_has_no_button() {
        let mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let s = rim_sim::Sim::with_mods(&mods, 1, &|m| m == "core").unwrap();
        let d = |id: &str| s.world.defs.lookup("designation", id).unwrap();
        assert!(!markable(&s.world.defs, d("core:gather")));
        for id in ["core:chop", "core:mine", "core:harvest", "core:hunt", "core:deconstruct"] {
            assert!(markable(&s.world.defs, d(id)), "{id}");
        }
        // With the stone age, there's something to gather.
        let s = rim_sim::Sim::new(&mods, 1).unwrap();
        let gather = s.world.defs.lookup("designation", "core:gather").unwrap();
        assert!(markable(&s.world.defs, gather));
    }
}
