//! What the player's input does: actions, orders, the selection, and
//! what the pointer is over.

use super::*;

pub(crate) fn apply_ui(app: &mut App, a: UiAction) {
    match a {
        UiAction::Select(e) => {
            select(app, e.into_iter().collect());
            // A colonist picked from a list on another level: go to them
            // (DESIGN.md §6d). On this level the camera stays put.
            let there = e.and_then(|e| app.sim.world.ecs.get::<&Pawn>(e).ok().map(|p| p.pos.z));
            if let Some(e) = e.filter(|_| there.is_some_and(|z| z != app.cam.z)) {
                focus(app, e);
            }
        }
        UiAction::ToggleSelect(e) => toggle_selected(app, e),
        UiAction::Focus(e) => focus(app, e),
        UiAction::Level(z) => {
            let levels = app.sim.world.map.levels();
            app.cam.z = z.clamp(*levels.start(), *levels.end());
        }
        UiAction::Tool(key) => {
            if let Some(t) = app.tools.iter().find(|t| t.key == key) {
                app.tool = t.tool;
                app.drag_start = None;
                app.refused = None;
            }
        }
        UiAction::Preview(key) => {
            app.preview = key.and_then(|k| match app.tools.iter().find(|t| t.key == k)?.tool {
                Tool::Build(d) => Some(d),
                _ => None,
            });
        }
        // The material for the thing the tray describes, else the tool's.
        UiAction::Stuff(id) => {
            let target = app.preview.or(match app.tool {
                Tool::Build(t) => Some(t),
                _ => None,
            });
            if let (Some(t), Some(m)) = (target, app.sim.world.defs.thing_id(&id)) {
                let sc = app.sim.world.defs.thing(t).build.as_ref().and_then(|b| b.stuff.as_ref());
                if sc.is_some_and(|sc| app.sim.world.defs.is_material_for(m, &sc.category)) {
                    app.stuff_for.retain(|(b, _)| *b != t);
                    app.stuff_for.push((t, m));
                }
            }
        }
        UiAction::Speed(s) => apply(app, Action::Speed(s)),
        UiAction::TogglePause => apply(app, Action::TogglePause),
        UiAction::Draft(e, on) => app.sim.push(Command::Draft { pawn: e, on }),
        // A pick from the orders menu: every selected colonist it's on
        // offer to gets it, by name.
        // The UI names a spot by x and y: it is on the level shown.
        UiAction::Order { key, cell, on } => give_orders(app, IVec::at(cell.x, cell.y, app.cam.z), on, Some(&key)),
        UiAction::Turn => app.build_facing = (app.build_facing + 1) & 3,
        UiAction::Undo => {
            if let Some(last) = app.last_order.take() {
                for c in last.undo {
                    app.sim.push(c);
                }
            }
        }
        UiAction::StoreLevel(store, level) => app.sim.push(Command::StoreLevel { store, level }),
        UiAction::StoreFilter(store, edit) => {
            use rim_sim::filter::FilterEdit as F;
            use rim_ui::view::UiFilterEdit as U;
            let defs = &app.sim.world.defs;
            let thing = |id: &str| defs.thing_id(id);
            let edit = match edit {
                U::Thing(id, on) => thing(&id).map(|thing| F::Thing { thing, on }),
                U::Material(id, on) => thing(&id).map(|material| F::Material { material, on }),
                U::Category(id, on) => defs.lookup("item_category", &id).map(|category| F::Category { category, on }),
                U::Condition(min, max) => Some(F::Condition { min, max }),
                U::All(on) => Some(F::All { on }),
            };
            if let Some(edit) = edit {
                app.sim.push(Command::StoreFilter { store, edit });
            }
        }
        UiAction::SelectZone(zone) => {
            select(app, Vec::new());
            app.selected_zone = zone;
        }
        UiAction::ZonePlant(zone, plant) => {
            if let Some(plant) = app.sim.world.defs.thing_id(&plant) {
                app.sim.push(Command::ZonePlant { zone, plant });
            }
        }
        UiAction::ZoneAllow(zone, item, on) => {
            if let Some(thing) = app.sim.world.defs.thing_id(&item) {
                app.sim.push(Command::ZoneAllow { zone, thing, on });
            }
        }
        UiAction::SetPriority(e, work, level) => {
            if let Some(w) = app.sim.world.defs.lookup("work_type", &work) {
                app.sim.push(Command::SetPriority { pawn: e, work: w, level });
            }
        }
        UiAction::AssignWorkRole(pawn, role) => app.sim.push(Command::AssignWorkRole { pawn, role }),
        UiAction::SetRolePriority(role, work, level) => {
            if let Some(w) = app.sim.world.defs.lookup("work_type", &work) {
                app.sim.push(Command::SetRolePriority { role, work: w, level });
            }
        }
        UiAction::CreateRoleFromPawn(label, e) => {
            app.sim.push(Command::CreateWorkRole { label, from: rim_sim::command::RoleSource::Pawn(e) })
        }
        UiAction::CreateRoleFromRole(label, r) => {
            app.sim.push(Command::CreateWorkRole { label, from: rim_sim::command::RoleSource::Role(r) })
        }
        UiAction::DeleteRole(role) => app.sim.push(Command::DeleteWorkRole { role }),
        UiAction::ClearPriority(e, work) => {
            if let Some(w) = app.sim.world.defs.lookup("work_type", &work) {
                app.sim.push(Command::ClearPriority { pawn: e, work: w });
            }
        }
        UiAction::SetRuleEnabled(id, on) => {
            if let Some(rule) = app.sim.world.defs.lookup("priority_rule", &id) {
                app.sim.push(Command::SetRuleEnabled { rule, on });
            }
        }
        UiAction::MarkUrgent(target, on) => app.sim.push(Command::MarkUrgent { target, on }),
        UiAction::SetStance(id) => {
            if let Some(stance) = app.sim.world.defs.lookup("stance", &id) {
                app.sim.push(Command::SetStance { stance });
            }
        }
        UiAction::CycleOverlay => apply(app, Action::CycleOverlay),
        UiAction::SetOverlay(o) => {
            app.overlay = o.filter(|i| *i < app.sim.world.defs.fields.len());
            app.storage_overlay = false;
        }
        UiAction::ToggleProfiler => apply(app, Action::ToggleProfiler),
        UiAction::ToggleDevtools => apply(app, Action::ToggleDevtools),
        UiAction::ToggleMeasure => app.measure = !app.measure,
        UiAction::ToggleOutlines => app.ui.toggle_outlines(),
        UiAction::ReduceMotion(on) => {
            app.reduce_motion = on;
            if let Some(p) = &app.settings_file {
                if let Err(e) = save_setting(p, "reduce_motion", toml::Value::Boolean(on)) {
                    eprintln!("rim: could not save settings: {e}");
                }
            }
        }
        UiAction::RenderScale(s) => {
            let Some(s) = valid_render_scale(s as f64) else { return };
            if app.render_scale == Some(s) {
                return;
            }
            app.render_scale = Some(s);
            if let Some(p) = &app.settings_file {
                if let Err(e) = save_setting(p, "render_scale", toml::Value::Float(s as f64)) {
                    eprintln!("rim: could not save settings: {e}");
                }
            }
        }
        UiAction::Zoom(f) => {
            let (w, h) = (screen_width(), screen_height());
            apply(app, Action::Zoom(f, w / 2.0, h / 2.0));
        }
        UiAction::ScrollMode(m) => {
            let Some(mode) = ScrollMode::parse(&m) else { return };
            app.scroll_mode = mode;
            if let Some(p) = &app.settings_file {
                if let Err(e) = save_setting(p, "scroll", toml::Value::String(m)) {
                    eprintln!("rim: could not save settings: {e}");
                }
            }
        }
        UiAction::UiScale(s) => {
            app.ui.set_user_scale(s);
            let s = app.ui.user_scale();
            if let Some(p) = &app.settings_file {
                if let Err(e) = save_setting(p, "ui_scale", toml::Value::Float(s as f64)) {
                    eprintln!("rim: could not save settings: {e}");
                }
            }
        }
        UiAction::Send(name, data) => app.sim.push(Command::ModEvent { name, data }),
        // The title screen's; a game is already chosen.
        UiAction::Load(_) | UiAction::NewColony => {}
        UiAction::Advance(hours) => {
            // Devtools only: step the sim now, as fast as it goes.
            let ticks = (hours * rim_sim::TICKS_PER_DAY as f64 / 24.0) as u64;
            for _ in 0..ticks {
                app.sim.step();
                app.motion.stepped(&app.sim.world);
                if let Some(w) = &app.saver {
                    save::after_step(w, &mut app.sim);
                }
            }
        }
    }
}

/// Is a select press at `(x, y)` a box rather than a click: has the
/// pointer moved far enough from where it went down? The preview and the
/// release ask the same question.
pub fn is_box(app: &App, (x, y): (f32, f32)) -> bool {
    const BOX_POINTS: f32 = 6.0;
    let (fx, fy) = app.drag_from;
    app.tool == Tool::Select && (x - fx).abs().max((y - fy).abs()) >= BOX_POINTS
}

/// The colonists standing in the box from `a` to `b`: what a select drag
/// picks, and what its preview shows.
pub fn boxed_colonists(app: &App, a: IVec, b: IVec) -> Vec<Entity> {
    let (lo, hi) = (IVec::new(a.x.min(b.x), a.y.min(b.y)), IVec::new(a.x.max(b.x), a.y.max(b.y)));
    let w = &app.sim.world;
    w.colonists()
        .filter(|&e| {
            w.ecs.get::<&Pawn>(e).is_ok_and(|p| {
                // Only the level on screen.
                if p.pos.z != app.cam.z {
                    return false;
                }
                let (px, py) = draw::pawn_pos(app, e, &p);
                let c = IVec::new(px.floor() as i32, py.floor() as i32);
                (lo.x..=hi.x).contains(&c.x) && (lo.y..=hi.y).contains(&c.y)
            })
        })
        .collect()
}

/// Every selected id: the group, or the one thing.
pub fn selection(app: &App) -> Vec<Entity> {
    if app.group.is_empty() {
        app.selected.into_iter().collect()
    } else {
        app.group.clone()
    }
}

/// Select these: nothing, one pawn or thing, or several colonists (the
/// first is the one the inspector shows).
pub fn select(app: &mut App, v: Vec<Entity>) {
    app.selected_zone = None;
    app.selected = v.first().copied();
    app.group = if v.len() > 1 { v } else { Vec::new() };
}

/// An active, living colonist of the player's.
pub(crate) fn colonist(app: &App, e: Entity) -> bool {
    app.sim.world.ecs.get::<&Pawn>(e).is_ok_and(|p| p.faction == Faction::Player && p.active && !p.dead)
}

/// A shift-click: put a colonist into the selection or take them out. A
/// selected thing, or anything that isn't a colonist, starts it afresh.
pub fn toggle_selected(app: &mut App, e: Entity) {
    if !colonist(app, e) {
        return select(app, vec![e]);
    }
    let mut v: Vec<Entity> = selection(app).into_iter().filter(|&s| colonist(app, s)).collect();
    match v.iter().position(|&s| s == e) {
        Some(i) => {
            v.remove(i);
        }
        None => v.push(e),
    }
    select(app, v);
}

/// Open the orders menu for the map spot under a point (logical).
pub(crate) fn open_orders(app: &mut App, cv: &rim_ui::view::ClientView, at: (f32, f32)) {
    let cell = app.cam.tile_at(at.0, at.1);
    let on = pawn_under(app, at.0, at.1);
    let id = match on {
        Some(e) => format!("{},{},{}", cell.x, cell.y, e.to_bits().get()),
        None => format!("{},{}", cell.x, cell.y),
    };
    let dpi = screen_dpi_scale();
    app.ui.context(&app.sim.world, cv, "tile", &id, (at.0 * dpi, at.1 * dpi));
}

/// The last order given, for a few seconds: what to say, and the commands
/// that take it back.
/// Write what stands in `a`..`b` as a plan in the player's `plans/`
/// folder, as a mod's defs would hold it, and say where. Loading the
/// player's own files as defs waits on the mod manager (7f26e2e3).
fn save_plan(app: &mut App, a: IVec, b: IVec) {
    let label = match app.settings_file.as_ref().map(|p| p.with_file_name("plans")) {
        None => "No folder to save plans in".to_string(),
        Some(dir) => {
            let n = (1..).find(|n| !dir.join(format!("plan_{n}.toml")).exists()).unwrap_or(1);
            let (id, path) = (format!("plan_{n}"), dir.join(format!("plan_{n}.toml")));
            let text = rim_sim::plan::plan_text(&app.sim.world, a, b, &id, &format!("saved plan {n}"));
            match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
                Ok(()) => format!("Saved as a plan: {}", path.display()),
                Err(e) => format!("Couldn't save the plan: {e}"),
            }
        }
    };
    app.last_order = Some(LastOrder { label, at: app.now, undo: Vec::new() });
}

pub struct LastOrder {
    pub label: String,
    pub at: f64,
    pub undo: Vec<Command>,
}

/// Give every selected colonist the order at a spot, by name (`pick`, the
/// orders menu) or the first safe one (a plain right-click), and remember
/// how to take it back.
fn give_orders(app: &mut App, cell: IVec, on: Option<Entity>, pick: Option<&str>) {
    let mut undo = Vec::new();
    let mut said: Option<(Entity, String)> = None;
    for e in selection(app) {
        let w = &app.sim.world;
        let o = match pick {
            Some(key) => order::choose(w, e, cell, on, key),
            None => order::resolve(w, e, cell, on),
        };
        let Some(o) = o else { continue };
        let target = order::target_of(&o.job);
        // A deconstruct marks what it takes down; undo takes the mark back
        // only if the order put it there.
        let unmark = match o.job {
            rim_sim::world::Job::Deconstruct { target, .. }
                if w.ecs.get::<&rim_sim::world::Designated>(target).is_err() =>
            {
                Some(target)
            }
            _ => None,
        };
        undo.push(Command::UndoOrder { pawn: e, target, cell, unmark });
        said.get_or_insert((e, o.label.clone()));
        app.sim.push(Command::Order { pawn: e, cell, on, pick: pick.map(str::to_string) });
    }
    let Some((first, label)) = said else { return };
    app.order_flash = Some((cell, app.chalk.now()));
    let what = label.to_lowercase();
    let who = match undo.len() {
        1 => app.sim.world.ecs.get::<&Pawn>(first).map(|p| p.name.clone()).unwrap_or_default(),
        n => format!("{n} colonists"),
    };
    app.last_order = Some(LastOrder { label: format!("{who} will {what}"), at: app.now, undo });
}

/// Label the cursor with what a right-click would do. Resolving an order
/// walks the map, so it happens once per tile rather than once per frame.
pub(crate) fn hint(app: &mut App, mouse: (f32, f32)) {
    let (mx, my) = mouse;
    let Some(e) = app.selected.filter(|_| app.tool == Tool::Select && !app.mouse_over_ui) else {
        app.hint = None;
        app.hint_key = None;
        return;
    };
    let key = (e, app.cam.tile_at(mx, my), pawn_under(app, mx, my));
    if app.hint_key == Some(key) {
        return;
    }
    app.hint_key = Some(key);
    let options = order::options(&app.sim.world, e, key.1, key.2);
    let safe = options.iter().filter(|c| !c.damaging).find_map(|c| c.order.as_ref().map(|o| o.label.clone()));
    app.hint = match safe {
        Some(label) if options.len() > 1 => Some(format!("{label} · hold for more")),
        Some(label) => Some(label),
        None if !options.is_empty() => Some("Hold for orders".into()),
        None => None,
    };
}

pub fn pawn_under(app: &App, sx: f32, sy: f32) -> Option<Entity> {
    let (wx, wy) = app.cam.to_world(sx, sy);
    let w = &app.sim.world;
    let mut best: Option<(f32, Entity)> = None;
    for &e in &w.pawns {
        let Ok(p) = w.ecs.get::<&Pawn>(e) else { continue };
        if p.pos.z != app.cam.z {
            continue;
        }
        let (px, py) = draw::pawn_pos(app, e, &p);
        let d = ((px - wx).powi(2) + (py - wy).powi(2)).sqrt();
        let bias = if p.faction == Faction::Player { -0.2 } else { 0.0 };
        if d < 0.7 && best.is_none_or(|b| d + bias < b.0) {
            best = Some((d + bias, e));
        }
    }
    best.map(|b| b.1)
}

/// What a click with the select tool at a screen point would pick: a
/// pawn, else a thing, else a stockpile.
pub(crate) fn hovered(app: &App, sx: f32, sy: f32) -> Option<overlay::Hovered> {
    use overlay::Hovered;
    let w = &app.sim.world;
    pawn_under(app, sx, sy)
        .or_else(|| thing_under(app, sx, sy))
        .map(Hovered::Thing)
        .or_else(|| w.zones.at(&w.map, app.cam.tile_at(sx, sy)).map(|z| Hovered::Zone(z.id)))
}

/// The thing in the cell under the cursor, topmost first: an item stack
/// lying there, the fixture, then the floor.
pub fn thing_under(app: &App, sx: f32, sy: f32) -> Option<Entity> {
    let cell = app.cam.tile_at(sx, sy);
    let map = &app.sim.world.map;
    map.item_at(cell).or_else(|| map.fixture_at(cell)).or_else(|| map.floor_at(cell))
}

/// Everything the player can do to the world, independent of which key or
/// button did it. UI controls produce `UiAction`s instead.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    TogglePause,
    Speed(u32),
    ToggleProfiler,
    ToggleDevtools,
    /// Cycle the field overlay: off, then each field the mods define.
    CycleOverlay,
    NextColonist,
    /// Move the camera by this many tiles.
    Pan(f32, f32),
    /// Zoom by a factor, keeping the world point under (x, y) fixed.
    Zoom(f32, f32, f32),
    /// Left button went down / up on the world at a screen position.
    LeftDown(f32, f32),
    LeftUp(f32, f32),
    RightClick(f32, f32),
}

pub fn apply(app: &mut App, action: Action) {
    match action {
        Action::TogglePause => app.paused = !app.paused,
        Action::Speed(s) => {
            app.speed = s;
            app.paused = false;
        }
        Action::ToggleProfiler => app.show_profiler = !app.show_profiler,
        Action::ToggleDevtools => {
            app.show_devtools = !app.show_devtools;
            app.ui.devtools = app.show_devtools;
        }
        Action::CycleOverlay => {
            // Off, each field that varies over the map, then storage, then
            // off again.
            if app.storage_overlay {
                app.storage_overlay = false;
            } else {
                let fields = &app.sim.world.defs.fields;
                let from = app.overlay.map_or(0, |i| i + 1);
                app.overlay = (from..fields.len()).find(|&i| fields[i].overlay);
                app.storage_overlay = app.overlay.is_none();
            }
        }
        Action::NextColonist => {
            let cols: Vec<Entity> = app.sim.world.colonists().collect();
            if !cols.is_empty() {
                let i =
                    app.selected.and_then(|s| cols.iter().position(|&c| c == s)).map_or(0, |i| (i + 1) % cols.len());
                select(app, vec![cols[i]]);
                focus(app, cols[i]);
            }
        }
        Action::Pan(dx, dy) => {
            app.cam.x += dx;
            app.cam.y += dy;
        }
        Action::Zoom(f, x, y) => {
            let before = app.cam.to_world(x, y);
            app.cam.zoom = (app.cam.zoom * f).clamp(MIN_ZOOM, 80.0);
            let after = app.cam.to_world(x, y);
            app.cam.x += before.0 - after.0;
            app.cam.y += before.1 - after.1;
        }
        // Select acts on release: a click picks what's under it, a drag
        // picks the colonists in the box.
        Action::LeftDown(x, y) => {
            // A new press is a new question: the last refusal is answered.
            app.refused = None;
            // With the select tool, a chevron for a selection off screen
            // brings it back.
            if app.tool == Tool::Select {
                if let Some(e) = overlay::offscreen_at(app, (x, y)) {
                    focus(app, e);
                    return;
                }
            }
            app.drag_start = Some(app.cam.tile_at(x, y));
            app.drag_from = (x, y);
        }
        Action::LeftUp(x, y) => {
            let Some(a) = app.drag_start.take() else { return };
            let b = app.cam.tile_at(x, y);
            let defs = app.sim.world.defs.clone();
            match app.tool {
                Tool::Designate(d) => {
                    refuse_if_idle(app, Some(d), a, b);
                    let (a, b) = overlay::designate_box(app, d, a, b);
                    app.sim.push(Command::Designate { designation: d, a, b });
                }
                Tool::Build(t) => {
                    // A click where nothing can go up is refused, with why:
                    // asked of the cell released on, not last frame's.
                    if a == b {
                        let bp = overlay::BuildPreview::between(app, t, a, b, false);
                        if bp.counts().0 == 0 {
                            app.refused = Some((bp.refusal(), a, app.chalk.now()));
                        }
                    }
                    let stuff = chosen_material(app, t);
                    for (a, b) in build_rects(defs.thing(t).blocks, a, b) {
                        app.sim.push(Command::Build { stuff, thing: t, a, b, facing: app.build_facing });
                    }
                }
                Tool::Plan(plan) => {
                    app.sim.push(Command::PlacePlan { plan, at: b, facing: app.build_facing, stuff: None })
                }
                Tool::SavePlan => save_plan(app, a, b),
                Tool::Stockpile => {
                    let zone = app.sim.world.zones.touched(&app.sim.world.map, a, b);
                    app.sim.push(Command::Stockpile { a, b, zone });
                }
                Tool::Grow(plant) => {
                    // Extends a field of the same crop; never a stockpile.
                    let w = &app.sim.world;
                    let zone = w
                        .zones
                        .touched(&w.map, a, b)
                        .filter(|&id| w.zones.get(id).is_some_and(|z| z.plant == Some(plant)));
                    app.sim.push(Command::GrowZone { a, b, zone, plant });
                }
                Tool::ClearZone => app.sim.push(Command::ClearZone { a, b }),
                Tool::Cancel => {
                    refuse_if_idle(app, None, a, b);
                    app.sim.push(Command::Cancel { a, b });
                }
                Tool::Select => {
                    let (fx, fy) = app.drag_from;
                    if !is_box(app, (x, y)) {
                        match pawn_under(app, fx, fy).or_else(|| thing_under(app, fx, fy)) {
                            Some(e) if app.shift => toggle_selected(app, e),
                            under => {
                                select(app, under.into_iter().collect());
                                // Nothing there but a stockpile's cell: the stockpile.
                                if under.is_none() {
                                    let w = &app.sim.world;
                                    app.selected_zone = w.zones.at(&w.map, app.cam.tile_at(fx, fy)).map(|z| z.id);
                                }
                            }
                        }
                    } else {
                        let boxed = boxed_colonists(app, a, b);
                        let picked = selection(app);
                        if app.subtract {
                            // Takes the boxed out; a box that takes no one
                            // changes nothing.
                            if picked.iter().any(|e| boxed.contains(e)) {
                                select(app, picked.into_iter().filter(|e| !boxed.contains(e)).collect());
                            }
                        } else if app.shift {
                            // Adds to the colonists already picked; a thing
                            // picked before doesn't join a group.
                            let mut v: Vec<Entity> = picked.into_iter().filter(|&e| colonist(app, e)).collect();
                            v.extend(boxed.into_iter().filter(|e| !v.contains(e)).collect::<Vec<_>>());
                            select(app, v);
                        } else {
                            select(app, boxed);
                        }
                    }
                }
            }
        }
        Action::RightClick(x, y) => {
            let cell = app.cam.tile_at(x, y);
            let on = pawn_under(app, x, y);
            give_orders(app, cell, on, None);
        }
    }
    let (mw, mh) = (app.sim.world.map.w as f32, app.sim.world.map.h as f32);
    app.cam.x = app.cam.x.clamp(0.0, mw);
    app.cam.y = app.cam.y.clamp(0.0, mh);
}

/// Walls (anything that blocks) are drawn as a room outline; everything
/// else fills the dragged rectangle.
/// How much of an item the colony has lying around, blueprints aside.
fn stock(w: &rim_sim::world::World, d: DefId) -> u32 {
    w.ecs
        .query::<&rim_sim::world::Thing>()
        .without::<&rim_sim::world::Blueprint>()
        .iter()
        .filter(|t| t.def == d)
        .map(|t| t.count)
        .sum::<u32>()
}

/// The material `thing` will be built from: what the player last picked
/// for it, else whatever the colony has most of, else the first the def
/// would accept so a blueprint can still be placed and waited on.
pub(crate) fn chosen_material(app: &App, thing: DefId) -> Option<DefId> {
    let w = &app.sim.world;
    let sc = w.defs.thing(thing).build.as_ref()?.stuff.as_ref()?;
    let options = w.defs.materials(&sc.category);
    if let Some((_, m)) = app.stuff_for.iter().find(|(b, _)| *b == thing) {
        if options.contains(m) {
            return Some(*m);
        }
    }
    options.iter().copied().max_by_key(|&d| stock(w, d)).or_else(|| options.first().copied())
}

/// What `n` of a buildable cost, in its chosen material and parts:
/// "90 logs", "48 planks · 16 nails"; "free" for none.
pub(crate) fn build_cost_for(app: &App, t: DefId, n: u32) -> String {
    let defs = &app.sim.world.defs;
    let Some(b) = defs.thing(t).build.as_ref() else { return String::new() };
    if b.free {
        return "free".into();
    }
    let mut parts: Vec<String> = b.cost_r.iter().map(|&(d, k)| format!("{} {}", k * n, defs.thing(d).label)).collect();
    if let Some(sc) = &b.stuff {
        let label = chosen_material(app, t).map_or_else(|| sc.category.clone(), |m| defs.thing(m).label.clone());
        parts.insert(0, format!("{} {label}", sc.count * n));
    }
    parts.join(" · ")
}

/// A buildable's cost in words, in the material it would use now.
pub(crate) fn build_cost(app: &App, t: DefId) -> String {
    build_cost_for(app, t, 1)
}

/// The material row for the active build tool: every material its def
/// accepts, with stock and what the result would be, or nothing at all.
pub(crate) fn stuff_view(app: &App) -> Vec<rim_ui::view::StuffView> {
    let t = match (app.preview, app.tool) {
        (Some(p), _) => p,
        (None, Tool::Build(t)) => t,
        _ => return Vec::new(),
    };
    let w = &app.sim.world;
    let td = w.defs.thing(t);
    let Some(b) = td.build.as_ref() else { return Vec::new() };
    let Some(sc) = b.stuff.as_ref() else { return Vec::new() };
    let active = chosen_material(app, t);
    w.defs
        .materials(&sc.category)
        .into_iter()
        .map(|m| {
            let md = w.defs.thing(m);
            rim_ui::view::StuffView {
                id: md.id.clone(),
                label: md.label.clone(),
                color: md.rgb,
                have: stock(w, m),
                active: active == Some(m),
                hp: (td.hp as f64 * w.defs.factor(Some(m), "hp")).round() as u32,
                work: (b.work as f64 * w.defs.factor(Some(m), "work")).round() as u32,
            }
        })
        .collect()
}

pub fn build_rects(blocks: bool, a: IVec, b: IVec) -> Vec<(IVec, IVec)> {
    let (x0, x1, y0, y1) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
    if !blocks || x1 - x0 < 2 || y1 - y0 < 2 {
        return vec![(a, b)];
    }
    vec![
        (IVec::at(x0, y0, a.z), IVec::at(x1, y0, a.z)),
        (IVec::at(x0, y1, a.z), IVec::at(x1, y1, a.z)),
        (IVec::at(x0, y0 + 1, a.z), IVec::at(x0, y1 - 1, a.z)),
        (IVec::at(x1, y0 + 1, a.z), IVec::at(x1, y1 - 1, a.z)),
    ]
}

pub(crate) fn focus(app: &mut App, e: Entity) {
    let w = &app.sim.world;
    let at = w.ecs.get::<&Pawn>(e).map(|p| p.pos).ok().or_else(|| w.thing(e).map(|t| t.pos));
    if let Some(p) = at {
        app.cam.x = p.x as f32 + 0.5;
        app.cam.y = p.y as f32 + 0.5;
        app.cam.z = p.z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_of_walls_is_planned_on_the_level_it_was_dragged_on() {
        let rects = build_rects(true, IVec::at(2, 2, -1), IVec::at(6, 5, -1));
        assert_eq!(rects.len(), 4);
        assert!(rects.iter().all(|(a, b)| a.z == -1 && b.z == -1), "{rects:?}");
    }
}
