//! The UI scripting API (`ui`, `act`, `view`), declared once: the source
//! for the Luau type definitions (`types/ui.d.luau`) and the reference
//! (`docs/modding/api-ui.md`). `tests/api_types.rs` checks this list against
//! the functions the VM actually registers, so neither can drift.
//!
//! Kept apart from `vm.rs` rather than beside each registration, as the sim
//! does, because `view` functions come and go with every feature and a flat
//! list here is easier to review than signatures spread through closures.

/// One member of `ui`, `act` or `view`.
pub struct UiDoc {
    /// `"view.tick"`.
    pub name: &'static str,
    /// A Luau function type.
    pub sig: &'static str,
    pub doc: &'static str,
}

/// Types the declarations refer to.
pub const UI_TYPES: &str = r#"type Size = number | string
type Colors = { bg: string?, border: string?, color: string? }
type Node = {
    kind: string?, id: string?, dir: string?, text: string?,
    gap: Size?, pad: Size?, padx: Size?, pady: Size?,
    w: Size?, h: Size?, minw: Size?, maxw: Size?, minh: Size?, maxh: Size?,
    grow: number?, align: string?, justify: string?, clip: boolean?,
    bg: string?, border: string?, color: string?, radius: Size?,
    size: string?, weight: string?, wrap: boolean?,
    hover: Colors?, press: Colors?, focus: Colors?, focusable: boolean?,
    on_click: (() -> ())?, on_right_click: (() -> ())?, tooltip: string?, disabled: boolean?,
    entity: number?, cell: { number }?, priority: number?, offset: number?,
    [number]: any,
}
type Layer = "top" | "bottom" | "left" | "right" | "anchored" | "cursor" | "modal" | "windows"
type Need = { id: string, label: string, value: number, color: string, low: boolean }
type Pawn = {
    id: number, name: string, label: string, faction: string, player: boolean, founder: boolean,
    drafted: boolean, asleep: boolean, hp: number, max_hp: number, health: number, job: string,
    selected: boolean, needs: { Need },
}
type VisiblePawn = {
    id: number, name: string, faction: string, intelligent: boolean, asleep: boolean,
    selected: boolean, hovered: boolean, radius: number,
}
type Message = { text: string, kind: string, age: number }
type WorldEvent = { tick: number, kind: string, id: number, name: string }
type FieldInfo = {
    index: number, id: string, label: string, unit: string, hud: boolean, overlay: boolean,
    ambient: number, shown: boolean,
}
type UiDate = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number }
type Part = { label: string, value: number }
type Tool = { key: string, label: string, color: string, active: boolean }
type Stuff = { id: string, label: string, color: string, have: number, active: boolean, hp: number, work: number }
type Hover = { x: number, y: number, terrain: string, shelter: string, readings: { string }, things: { string } }
type ProfileRow = { name: string, us: number, mod: boolean }
type ModInfo = { id: string, version: string, name: string }
type UiStats = { font: string, build_us: number, layout_us: number, paint_us: number, nodes: number, layouts: number }
type Inspect = { id: string, owner: string, kind: string, path: string, x: number, y: number, w: number, h: number }
type TreeRow = { depth: number, kind: string, id: string, owner: string }"#;

macro_rules! d {
    ($name:literal, $sig:literal, $doc:literal) => {
        UiDoc { name: $name, sig: $sig, doc: $doc }
    };
}

/// Every member, sorted by name.
pub const UI_API: &[UiDoc] = &[
    d!("act.advance", "(hours: number) -> ()", "Run the game forward (devtools)."),
    d!("act.cycle_overlay", "() -> ()", "Show the next field overlay."),
    d!("act.draft", "(id: number, on: boolean) -> ()", "Draft or undraft a colonist."),
    d!("act.focus", "(id: number) -> ()", "Move the camera to a pawn."),
    d!("act.select", "(id: number?) -> ()", "Select a pawn, or nothing."),
    d!(
        "act.send",
        "(name: string, data: {[string]: any}?) -> ()",
        "Send an event to your mod's own sim scripts (\"your_mod:event\"), as a player command."
    ),
    d!("act.set_overlay", "(index: number?) -> ()", "Show a field overlay by its index in view.fields(), or none."),
    d!("act.speed", "(speed: number) -> ()", "Set the game speed."),
    d!("act.stuff", "(id: string) -> ()", "Choose the material for the active build tool."),
    d!("act.toggle_devtools", "() -> ()", "Show or hide devtools."),
    d!("act.toggle_outlines", "() -> ()", "Show or hide layout outlines (devtools)."),
    d!("act.toggle_pause", "() -> ()", "Pause or resume."),
    d!("act.toggle_profiler", "() -> ()", "Show or hide the profiler."),
    d!("act.tool", "(key: string) -> ()", "Pick a toolbar tool (\"designate:core:chop\", \"build:core:wall\")."),
    d!("ui.anchored", "(node: Node?) -> Node", "A node attached to a pawn (entity) or cell, on the anchored layer."),
    d!("ui.col", "(node: Node?) -> Node", "A column: children top to bottom."),
    d!("ui.define", "(id: string, build: (view: any) -> Node?) -> ()", "Define a component under a namespaced id."),
    d!("ui.extend", "(id: string, add: any) -> ()", "Add children to another component's extension point."),
    d!(
        "ui.mount",
        "(layer: Layer, id: string, opts: { order: number?, align: string? }?) -> ()",
        "Show a component on a screen layer."
    ),
    d!("ui.remove", "(id: string) -> ()", "Hide a node by id."),
    d!("ui.replace", "(id: string, build: (view: any) -> Node?) -> ()", "Take over a node by id."),
    d!("ui.row", "(node: Node?) -> Node", "A row: children left to right."),
    d!("ui.scroll", "(node: Node?) -> Node", "A column that scrolls."),
    d!("ui.set_state", "(key: string, value: any) -> ()", "Keep a value across rebuilds."),
    d!("ui.slot", "(id: string) -> Node", "An extension point other mods fill with ui.extend."),
    d!("ui.spacer", "(node: Node?) -> Node", "Empty space that grows."),
    d!("ui.state", "(key: string, default: any) -> any", "A value kept with ui.set_state, or default."),
    d!("ui.text", "(node: Node | string) -> Node", "Text: { \"words\", size = ..., color = ... }."),
    d!(
        "ui.wrap",
        "(id: string, wrap: (inner: Node, view: any) -> Node?) -> ()",
        "Decorate a node: get its tree, return a new one."
    ),
    d!("view.ambient", "(field: string) -> number?", "A field's outdoor value, or nil for an unknown field."),
    d!("view.clock", "() -> string", "The time of day, \"HH:MM\"."),
    d!("view.colonists", "() -> { Pawn }", "The colonists."),
    d!("view.colony_lost", "() -> boolean", "Whether every colonist is gone."),
    d!("view.count_pawns", "(faction: string) -> number", "Living pawns of \"player\", \"hostile\" or \"wild\"."),
    d!("view.data", "(key: string) -> any", "A copy of data a sim script stored with rim.set_data, or nil."),
    d!("view.date", "() -> UiDate", "The calendar date, counted from 1."),
    d!("view.day", "() -> number", "The day, counted from 1."),
    d!("view.events", "(since_tick: number) -> { WorldEvent }", "Recent joins, deaths and departures, newest last."),
    d!("view.explain", "(field: string) -> { Part }", "Each term and push that makes up a field's outdoor value."),
    d!("view.fields", "() -> { FieldInfo }", "The field layers."),
    d!("view.hint", "() -> string?", "What a right-click would do."),
    d!("view.hour", "() -> number", "Hour of the day, 0 to 24."),
    d!("view.hover", "() -> Hover?", "What's under the cursor."),
    d!("view.inspect", "() -> Inspect?", "The node under the cursor (devtools)."),
    d!("view.messages", "(max: number) -> { Message }", "The newest messages, newest first."),
    d!("view.mods", "() -> { ModInfo }", "Loaded mods, in load order."),
    d!("view.outlines", "() -> boolean", "Whether layout outlines are on."),
    d!("view.overlay", "() -> string?", "The label of the field overlay shown, if any."),
    d!("view.paused", "() -> boolean", "Whether the game is paused."),
    d!("view.pawn", "(id: number) -> Pawn?", "One pawn, or nil if it's gone."),
    d!("view.profile", "() -> { ProfileRow }", "Smoothed time per system and mod, in µs."),
    d!("view.screen", "() -> (number, number)", "Screen width and height in logical pixels."),
    d!("view.selected", "() -> number?", "The selected pawn's id."),
    d!("view.show_devtools", "() -> boolean", "Whether devtools are open."),
    d!("view.show_profiler", "() -> boolean", "Whether the profiler is open."),
    d!("view.speed", "() -> number", "The game speed."),
    d!("view.stats", "() -> { string }", "Client statistics lines."),
    d!("view.stuff", "() -> { Stuff }", "Materials for the active build tool: what you have, what you'd get."),
    d!("view.tick", "() -> number", "The current tick."),
    d!("view.ticks_per_day", "() -> number", "Ticks in a game day."),
    d!("view.time", "() -> number", "Wall-clock seconds, for animation."),
    d!("view.tools", "() -> { Tool }", "The toolbar's tools."),
    d!("view.ui_stats", "() -> UiStats", "The UI's own timings."),
    d!("view.ui_tree", "() -> { TreeRow }", "The node tree (devtools)."),
    d!("view.visible_pawns", "() -> { VisiblePawn }", "Pawns on screen, for anchored labels."),
    d!("view.warnings", "() -> { string }", "Load warnings."),
    d!("view.wealth", "() -> number", "The colony's wealth."),
];

/// `types/ui.d.luau`.
pub fn luau_definitions() -> String {
    let mut out = String::from(
        "-- Generated from crates/rim_ui/src/api.rs: don't edit. Regenerate with\n\
         -- RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types\n\n",
    );
    out.push_str(UI_TYPES);
    out.push('\n');
    for global in ["ui", "act", "view"] {
        out.push_str(&format!("\ndeclare {global}: {{\n"));
        for d in UI_API.iter().filter(|d| d.name.split('.').next() == Some(global)) {
            let member = &d.name[global.len() + 1..];
            out.push_str(&format!("    -- {}\n    {member}: {},\n", d.doc, d.sig));
        }
        out.push_str("}\n");
    }
    out
}

/// `docs/modding/api-ui.md`.
pub fn api_reference() -> String {
    let mut out = String::from(
        "# UI API reference: `ui`, `act`, `view`\n\n\
         Generated from `crates/rim_ui/src/api.rs`: don't edit. Regenerate with\n\
         `RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types`. Types for\n\
         editors are in [`types/ui.d.luau`](../../types/ui.d.luau); the guide is\n\
         [Modding the interface](ui.md).\n\n\
         | Name | Type | What it does |\n|---|---|---|\n",
    );
    for d in UI_API {
        out.push_str(&format!("| `{}` | `{}` | {} |\n", d.name, d.sig.replace('|', "\\|"), d.doc.replace('|', "\\|")));
    }
    out
}
