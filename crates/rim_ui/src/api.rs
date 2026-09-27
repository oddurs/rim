//! The UI scripting API (`ui`, `act`, `view`), declared once: the source
//! for the Luau type definitions (`types/ui.d.luau`) and the reference
//! (`docs/modding/api-ui.md`). `tests/api_types.rs` checks this list against
//! the functions the VM actually registers, so neither can drift.
//!
//! Kept apart from `vm.rs` rather than beside each registration, as the sim
//! does, because `view` functions come and go with every feature and a flat
//! list here is easier to review than signatures spread through closures.

/// The version of this surface, as a mod names it in `mod.toml` as
/// `ui_api`. Before 1.0 every minor is breaking: a mod written against a
/// newer surface names calls this engine lacks, and is refused.
pub const UI_API_VERSION: (u32, u32) = (0, 6);

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
    size: string?, weight: string?, wrap: boolean?, tracking: Size?,
    hover: Colors?, press: Colors?, focus: Colors?, focusable: boolean?,
    on_click: (() -> ())?, on_right_click: (() -> ())?, tooltip: string?, disabled: boolean?,
    entity: number?, cell: { number }?, priority: number?, offset: number?,
    handle: ("move" | "resize" | "close")?,
    src: string?, tint: boolean?,
    value: string?, placeholder: string?, on_change: ((text: string) -> ())?, on_submit: ((text: string) -> ())?,
    on_key: ((key: string) -> ())?,
    on_drag: ((fx: number, fy: number) -> ())?,
    [number]: any,
}
type Cell = string | number | { text: string?, bg: string?, color: string?, bar: number?, bar_color: string?, tip: string?, weight: string?, dot: string?, ring: string? } | nil
type GridProps = {
    rows: number, cols: number, cell: (r: number, c: number) -> Cell,
    cell_w: Size?, cell_h: Size?, gap: Size?, size: string?, weight: string?,
    on_press: ((r: number, c: number) -> any)?, on_paint: ((r: number, c: number, value: any) -> ())?,
    on_wheel: ((r: number, c: number, steps: number, shift: boolean) -> ())?,
    on_key: ((r: number, c: number, key: string) -> ())?, keys: { string }?,
    id: string?, bg: string?, border: string?,
}
type ListProps = {
    id: string, count: number, row_h: Size, row: (i: number) -> Node,
    h: Size?, grow: number?, bg: string?, border: string?, pad: Size?,
}
type WindowOpts = { title: string?, w: number?, h: number?, resizable: boolean?, sheet: boolean?, open: boolean? }
type WindowInfo = { id: string, title: string, w: number, h: number, resizable: boolean, sheet: boolean, comp: string }
type Bind = { id: string, label: string, key: string, owner: string }
type Layer = "top" | "bottom" | "left" | "right" | "float" | "anchored" | "cursor" | "popup" | "modal" | "windows" | "title"
type Need = { id: string, label: string, value: number, color: string, low: boolean }
type Pawn = {
    id: number, name: string, label: string, faction: string, player: boolean, founder: boolean,
    drafted: boolean, asleep: boolean, hp: number, max_hp: number, health: number, job: string,
    selected: boolean, needs: { Need }, skills: { Skill }, hand: string?, carrying: string?,
}
type Person = {
    id: number, name: string, label: string, drafted: boolean, asleep: boolean, health: number,
    job: string, idle: boolean, selected: boolean,
}
type Skill = { id: string, label: string, level: number, progress: number, trains: string }
type VisiblePawn = {
    id: number, name: string, faction: string, intelligent: boolean, asleep: boolean,
    selected: boolean, hovered: boolean, radius: number,
}
type VisibleRoom = {
    id: number, x: number, y: number, cells: number, open: boolean,
    role: string?, role_id: string?, temperature: string?,
}
type Speech = { id: number, text: string, age: number, priority: number }
type Message = { text: string, kind: string, age: number, day: number, clock: string }
type WorldEvent = { tick: number, kind: string, id: number, name: string }
type FieldInfo = {
    index: number, id: string, label: string, unit: string, hud: boolean, overlay: boolean,
    ambient: number, shown: boolean,
}
type UiDate = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number }
type Part = { label: string, value: number }
type Tool = {
    key: string, label: string, color: string, active: boolean, category: string, group: string,
    cost: string, work: number, hp: number, locked: string,
}
type StoreRef = { zone: number?, thing: number? }
type FilterEdit = { thing: string?, category: string?, material: string?, on: boolean?, min: number?, max: number?, all: boolean? }
type StoreRow = {
    thing: string?, label: string?, made_of: string?, made_of_label: string?, count: number?, limit: number?,
    hp: number?, value: number?, stacks: number?, category: string?, category_label: string?, category_order: number?,
    empty: boolean?,
}
type StoreFilterView = { allows: { [string]: boolean }, refuses: { [string]: boolean }, can: { [string]: boolean }, min: number, max: number }
type StoreMaterial = { id: string, label: string, refused: boolean }
type StoreView = {
    kind: string, id: number, name: string, level: number, level_label: string, levels: { string },
    capacity: number, used: number, incoming: number, contents: { StoreRow }, filter: StoreFilterView,
    materials: { StoreMaterial },
}
type ItemCategory = { id: string, label: string, children: { string }, items: { string }, under: { string } }
type ItemCategories = { roots: { string }, by_id: { [string]: ItemCategory } }
type Zone = { id: number, name: string, cells: number, allows: { [string]: boolean }, level: number, level_label: string }
type Item = { id: string, label: string, color: string }
type WorkType = { id: string, label: string, icon: string, order: number, default: number }
type BoardCol = { id: string, label: string, icon: string, skill: string?, waiting: number, default: number, on: number, high: number }
type BoardCell = { base: number, value: number, inherit: number, pinned: boolean, planned: boolean, reason: string?, why: string, skill: number?, skill_frac: number? }
type BoardRow = { id: number, name: string, job: string, role: number?, role_label: string?, cells: { BoardCell } }
type BoardRole = { index: number, id: string?, label: string, order: number, edited: boolean, planned: boolean, default: boolean, levels: { [string]: number } }
type Board = { levels: number, labels: { string }, high: number, cols: { BoardCol }, rows: { BoardRow }, roles: { BoardRole } }
type StandingOrder = { id: string, label: string, reading: string, value: number?, band: string, effect: string, season: string?, crossed: boolean, enabled: boolean, acting: boolean }
type Markable = { id: number, urgent: boolean, label: string }
type Stance = { id: string, label: string, icon: string, active: boolean }
type Effective = { value: number, why: string }
type Stuff = { id: string, label: string, color: string, have: number, active: boolean, hp: number, work: number }
type Hover = { x: number, y: number, terrain: string, shelter: string, readings: { string }, values: { Reading }, things: { string }, takes: string?, store: StoreRef? }
type StockRow = { thing: string, label: string, total: number, stored: number, loose: number, stores: number, value: number, category: string?, category_label: string?, category_order: number }
type Reading = { label: string, value: string }
type WorkWhy = { work: string, level: number, why: string, picked: boolean, dist: number?, urgent: boolean }
type ProfileRow = { name: string, us: number, mod: boolean }
type ModInfo = { id: string, version: string, name: string }
type ThingInfo = {
    id: number, def: string, label: string, count: number, hp: number, max_hp: number,
    made_of: string?, blueprint: boolean, designated: string?, why: string?, store: boolean,
}
type Save = { path: string, file: string, day: number, colonists: { string }, age: number, error: string? }
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
    d!("act.assign_role", "(id: number, role: number) -> ()", "Put a colonist in a work role, by its index in view.board().roles. Their pins stay."),
    d!("act.clear_priority", "(id: number, work: string) -> ()", "Hand a colonist's work type back: forget their own setting, so they follow what they'd inherit."),
    d!("act.cycle_overlay", "() -> ()", "Show the next field overlay."),
    d!("act.delete_role", "(role: number) -> ()", "Delete one of the player's own work roles (a role from a mod can't be deleted), by its index in view.board().roles. Its members go to the default role and keep their pins; the roles after it move down one."),
    d!("act.draft", "(id: number, on: boolean) -> ()", "Draft or undraft a colonist."),
    d!("act.focus", "(id: number) -> ()", "Move the camera to a pawn or thing, and to its level."),
    d!("act.level", "(z: number) -> ()", "Show level z: 0 is the surface, below it is negative. Past the top or bottom level it stays at the last."),
    d!("act.lighting", "(preset: string) -> ()", "The lighting preset: 'low', 'medium', 'high' or 'ultra'. Settings the player set by hand under [lighting] stay. Saved for the player."),
    d!("act.load", "(path: string) -> ()", "Play a save from view.saves() (the title screen)."),
    d!("act.mark_urgent", "(id: number, on: boolean) -> ()", "Mark a job urgent (a blueprint, a thing or creature marked for work, an order's site), or clear it: everyone takes it a level sooner than its work type."),
    d!("act.new_colony", "() -> ()", "Start a new colony (the title screen)."),
    d!("act.order", "(key: string, x: number, y: number, on: number?) -> ()", "Give the selected colonists the order named key at a map spot (a row from view.orders), each one it's on offer to."),
    d!("act.preview", "(key: string?) -> ()", "Point the materials view (view.stuff) at a buildable by its tool key, for a card describing it; nil goes back to the tool in hand."),
    d!(
        "act.render_scale",
        "(scale: number) -> ()",
        "Draw the world at this fraction of the screen's pixels, 0.25 to 1; the UI stays sharp. Saved for the player."
    ),
    d!("act.role_from_colonist", "(label: string, id: number) -> ()", "Make a work role of the player's from a colonist's levels (role and pins), keeping what differs from the defaults. It joins the end of view.board().roles."),
    d!("act.role_from_role", "(label: string, role: number) -> ()", "Make a work role of the player's, copying another. It joins the end of view.board().roles."),
    d!("act.scroll_mode", "(mode: string) -> ()", "What a scroll does on the map: 'auto' (a trackpad pans, a wheel zooms), 'zoom' or 'pan'. Saved with the player's settings."),
    d!("act.select", "(id: number?, add: boolean?) -> ()", "Select a pawn or thing, or nothing. With add, put a colonist into the selection or take them out of it (a shift-click)."),
    d!("act.select_zone", "(zone: number?) -> ()", "Show a stockpile in the inspector, or none."),
    d!(
        "act.send",
        "(name: string, data: {[string]: any}?) -> ()",
        "Send an event to your mod's own sim scripts (\"your_mod:event\"), as a player command."
    ),
    d!("act.set_overlay", "(index: number?) -> ()", "Show a field overlay by its index in view.fields(), or none."),
    d!("act.set_priority", "(id: number, work: string, level: number) -> ()", "Set a colonist's priority for a work type: 1 first, 0 never."),
    d!("act.set_role_priority", "(role: number, work: string, level: number?) -> ()", "Set a work role's level for a work type, or nil to leave it to the default. A planned role (Auto) ignores it."),
    d!("act.set_rule_enabled", "(id: string, on: boolean) -> ()", "Switch a priority rule (a standing order) off for this colony, or back on."),
    d!("act.set_stance", "(id: string) -> ()", "Put the colony in a stance: its priority rules hold until another."),
    d!("act.speed", "(speed: number) -> ()", "Set the game speed."),
    d!("act.store_filter", "(store: number | StoreRef, edit: FilterEdit) -> ()", "Change what a store takes: a thing, a category of them, a material (on = false refuses it), the condition range in percent (min, max), or everything (all). A stockpile by id, or { zone = id } or { thing = id } for a container."),
    d!("act.store_level", "(store: number | StoreRef, level: number) -> ()", "Put a store at a level of the store priority scale (0 is lowest): a stockpile by id, or { zone = id } or { thing = id }. Stacks only move to a higher one."),
    d!("act.stuff", "(id: string) -> ()", "Choose the material for the active build tool."),
    d!("act.toggle_devtools", "() -> ()", "Show or hide devtools."),
    d!("act.toggle_outlines", "() -> ()", "Show or hide layout outlines (devtools)."),
    d!("act.toggle_pause", "() -> ()", "Pause or resume."),
    d!("act.toggle_profiler", "() -> ()", "Show or hide the profiler."),
    d!("act.tool", "(key: string) -> ()", "Pick a toolbar tool (\"designate:core:chop\", \"build:core:wall\")."),
    d!("act.turn", "() -> ()", "Turn what the build tool will place a quarter turn clockwise (DESIGN.md §6c)."),
    d!("act.ui_scale", "(scale: number) -> ()", "Set the player's UI scale, 0.75 to 2 on top of the display's; it is saved with their settings."),
    d!("act.undo", "() -> ()", "Take back the last order given: the colonists stop the job it gave them, and a mark it put on something goes."),
    d!("act.zone_allow", "(zone: number, item: string, on: boolean) -> ()", "Let a stockpile take an item, or stop it."),
    d!("act.zoom", "(factor: number) -> ()", "Zoom the map by a factor about the middle of the screen (1.12 is one wheel notch in)."),
    d!("ui.anchored", "(node: Node?) -> Node", "A node attached to a pawn (entity) or cell, on the anchored layer."),
    d!(
        "ui.bind",
        "(id: string, opts: { key: string?, label: string?, when: (() -> boolean)? }, run: () -> ()) -> ()",
        "A named action with a default key (\"space\", \"f3\", \"ctrl+k\"): it fires from the key when no text input is typing, and from the command palette. With no key it is in the palette alone. With when, the key is the action's only while when returns true; otherwise the key goes on to the game (Tab to the next colonist). The player's keybinds file overrides the key. Two mods binding one id or one key is reported."
    ),
    d!("ui.close", "(id: string) -> ()", "Close a window."),
    d!("ui.col", "(node: Node?) -> Node", "A column: children top to bottom."),
    d!("ui.define", "(id: string, build: (view: any) -> Node?) -> ()", "Define a component under a namespaced id."),
    d!("ui.extend", "(id: string, add: any) -> ()", "Add children to another component's extension point."),
    d!("ui.focus", "(id: string) -> ()", "Give a node (a text input) the keyboard once it is laid out."),
    d!(
        "ui.grid",
        "(props: GridProps) -> Node",
        "Rows by cols of cells the engine paints as one node. cell(r, c) describes each cell at build; on_press(r, c) returns the value a drag paints and on_paint(r, c, value) runs once per cell the drag enters; on_wheel(r, c, steps, shift) takes the wheel over a cell; on_key(r, c, key) takes the keys named in keys while the pointer is over a cell, ahead of any binding on them. A cell can carry a bar (0 to 1) along its bottom, its own tooltip (tip), its own text weight, and a dot or ring in its corner (a colour) marking who set it."
    ),
    d!(
        "ui.image",
        "(node: Node) -> Node",
        "A picture from a mod's ui/img: { src = \"mod:name\", tint = true }. Its own size unless w/h say otherwise; tint draws it in the text colour. A name@2x.png beside name.png is used on dense displays."
    ),
    d!(
        "ui.input",
        "(node: Node) -> Node",
        "A line of text the player edits: { id = ..., value = ..., placeholder = ..., on_change = fn(text), on_submit = fn(text), on_key = fn(\"up\" | \"down\") }. The engine keeps the buffer by id across rebuilds and reloads; click to focus, Escape to leave."
    ),
    d!("ui.is_open", "(id: string) -> boolean", "Whether a window is open."),
    d!(
        "ui.list",
        "(props: ListProps) -> Node",
        "A scroll area that builds only the rows on screen. Needs an id, count, row_h and row(i); spacers stand in for the rows above and below."
    ),
    d!(
        "ui.mount",
        "(layer: Layer, id: string, opts: { order: number?, align: string?, refresh: (\"frame\" | \"fast\" | \"slow\")?, slot: number? }?) -> ()",
        "Show a component on a screen layer. refresh says how often it is rebuilt when nothing forces it: every frame, twenty times a second (the default) or four. slot reserves a side panel's height so its header stays put while its content changes. The float layer places panels over the map between the side columns, and never moves a docked panel."
    ),
    d!("ui.on_context", "(handler: (subject: { kind: string, id: any }, x: number, y: number) -> ()) -> ()", "What a right-click on a node with a menu subject (menu = { kind, id }) calls, with the point in logical pixels. Core's menus module sets it; the last one set wins."),
    d!("ui.open", "(id: string) -> ()", "Open a window (and bring it to the front)."),
    d!("ui.remove", "(id: string) -> ()", "Hide a node by id."),
    d!("ui.replace", "(id: string, build: (view: any) -> Node?) -> ()", "Take over a node by id."),
    d!("ui.row", "(node: Node?) -> Node", "A row: children left to right. With wrap = true they go on in lines within the row's width."),
    d!("ui.run", "(id: string) -> ()", "Run a bound action, as its key would."),
    d!("ui.scroll", "(node: Node?) -> Node", "A column that scrolls."),
    d!("ui.set_input", "(id: string, text: string) -> ()", "Replace what a text input holds, caret at the end (the buffer is otherwise the player's)."),
    d!("ui.set_state", "(key: string, value: any) -> ()", "Keep a value across rebuilds."),
    d!("ui.sheet", "() -> string?", "The open sheet's id, or nil: at most one is open."),
    d!("ui.slot", "(id: string) -> Node", "An extension point other mods fill with ui.extend."),
    d!("ui.spacer", "(node: Node?) -> Node", "Empty space that grows."),
    d!("ui.state", "(key: string, default: any) -> any", "A value kept with ui.set_state, or default."),
    d!(
        "ui.t",
        "(key: string, default: string?) -> string",
        "A user-visible string by key: a mod's ui/lang.toml can replace it; until one does, the default."
    ),
    d!("ui.text", "(node: Node | string) -> Node", "Text: { \"words\", size = ..., color = ... }."),
    d!("ui.toggle", "(id: string) -> ()", "Open a window if closed, close it if open."),
    d!(
        "ui.window",
        "(id: string, opts: WindowOpts, component: ((view: any) -> Node?) | string) -> ()",
        "Declare a window the engine moves, sizes, stacks and remembers between runs. The component is shown inside the chrome; a function is defined under the window's id. With sheet = true it is a screen instead: placed in the band between the docked columns, as wide as w allows, one sheet open at a time."
    ),
    d!(
        "ui.window_chrome",
        "(draw: (win: WindowInfo) -> Node) -> ()",
        "The function that draws every window's chrome around ui.slot(win.comp); nodes marked handle = \"move\", \"resize\" or \"close\" are routed by the engine. Core sets it."
    ),
    d!(
        "ui.wrap",
        "(id: string, wrap: (inner: Node, view: any) -> Node?) -> ()",
        "Decorate a node: get its tree, return a new one."
    ),
    d!("view.ambient", "(field: string) -> number?", "A field's outdoor value, or nil for an unknown field."),
    d!("view.binds", "() -> { Bind }", "Every bound action with its label and current key, in declaration order."),
    d!("view.board", "() -> Board", "The Work Board in one read: columns in tie-break order with jobs waiting (`waiting`), colonists on it (`on`) and on it at a high priority (`high`, levels 1 to Board.high); a row per colonist with each cell's base, effective value, why, and skill."),
    d!("view.clock", "() -> string", "The time of day, \"HH:MM\"."),
    d!("view.colonists", "(max: number?) -> { Pawn }", "The colonists, or the first max of them (a bar that shows a few should not pay for all of them; view.count_pawns(\"player\") has the total)."),
    d!("view.colony_lost", "() -> boolean", "Whether every colonist is gone."),
    d!("view.compact", "() -> boolean", "Whether the screen is small (under 1440 logical pixels wide, UI scale included): core picks denser layouts."),
    d!("view.count_pawns", "(faction: string) -> number", "Living pawns of \"player\", \"hostile\" or \"wild\"."),
    d!("view.data", "(key: string) -> any", "A copy of data a sim script stored with rim.set_data, or nil."),
    d!("view.date", "() -> UiDate", "The calendar date, counted from 1."),
    d!("view.day", "() -> number", "The day, counted from 1."),
    d!("view.effective", "(id: number) -> { [string]: Effective }?", "A colonist's priority per work type once rules and the stance have had their say, with why: \"Build 1 = base 3, Siege -2\". Nil if it isn't a pawn."),
    d!("view.events", "(since_tick: number) -> { WorldEvent }", "Recent joins, deaths and departures, newest last."),
    d!("view.explain", "(field: string) -> { Part }", "Each term and push that makes up a field's outdoor value."),
    d!("view.explain_work", "(id: number) -> { WorkWhy }?", "The why panel: each work type in tie-break order with why the colonist would take it or passes it over (\"Needs a chopping tool\", \"Build first\"), and which it picks; `urgent` if that pick is a job the player marked."),
    d!("view.fields", "() -> { FieldInfo }", "The field layers."),
    d!("view.hint", "() -> string?", "What a right-click would do."),
    d!("view.hour", "() -> number", "Hour of the day, 0 to 24."),
    d!("view.hover", "() -> Hover?", "What's under the cursor."),
    d!("view.inspect", "() -> Inspect?", "The node under the cursor (devtools)."),
    d!("view.item_categories", "() -> ItemCategories", "The item category tree stores filter by: the top level in order, and each category by id with its children, the items directly in it, and every item under it."),
    d!("view.items", "() -> { Item }", "Every item def, which a stockpile can take or refuse."),
    d!("view.last_order", "() -> { label: string, age: number, undoable: boolean }?", "The last order given (\"Gunnar will deconstruct wall\"), how many seconds ago, and whether an undo takes it back; nil once it's been undone."),
    d!("view.level", "() -> number", "The level on screen: 0 is the surface, below it is negative."),
    d!("view.level_of", "(id: number) -> number?", "The level a pawn or thing is on."),
    d!("view.levels", "() -> { { z: number, colonists: number, others: number, reached: boolean } }", "Every level, the highest first: the colonists and other creatures on it, and whether it is reached (the surface and above, or a level a portal goes down to or a pit looks into)."),
    d!("view.look", "(thing: string, made_of: string?) -> number", "A thing's world look, tinted by what it's made of, as an index a token node's `look` takes (kind = \"token\"; `kit.item` builds one). Made once per thing and material."),
    d!("view.markable", "(x: number, y: number) -> Markable?", "The job on a tile an urgent mark could go on, with whether it has one."),
    d!("view.marked", "() -> { [string]: number }", "How many things each designation has marked, by designation id; ones with none are left out."),
    d!("view.message_count", "() -> number", "How many messages the log holds."),
    d!("view.messages", "(max: number, skip: number?) -> { Message }", "The newest messages, newest first; skip that many of the newest to page back through the log."),
    d!("view.mods", "() -> { ModInfo }", "Loaded mods, in load order."),
    d!("view.orders", "(x: number, y: number, on: number?) -> { caption: string?, actors: number, actor: string?, rows: { { key: string, label: string, group: string, trailing: string?, disabled: string? } } }", "Every order the selected colonists could be given at a map spot, merged by key: group 'damaging' for ones that take something away, trailing '2 of 3' when only some can, disabled with a reason when none can. Walks the map, so call it once per menu, not per frame."),
    d!("view.outlines", "() -> boolean", "Whether layout outlines are on."),
    d!("view.overlay", "() -> string?", "The label of the overlay shown, if any: a field's, or \"Storage\"."),
    d!("view.paused", "() -> boolean", "Whether the game is paused."),
    d!("view.pawn", "(id: number) -> Pawn?", "One pawn, or nil if it's gone."),
    d!("view.people", "() -> { Person }", "Every colonist, lean: what a list of them needs (name, job, health, drafted, idle, selected) and none of the needs or skills view.colonists carries."),
    d!("view.priorities", "(id: number) -> { [string]: number }?", "A colonist's priority per work type, by work type id: 1 first, 0 never. Nil if it isn't a pawn."),
    d!("view.priority_levels", "() -> number", "How many priority levels there are; 0 means never."),
    d!("view.profile", "() -> { ProfileRow }", "Smoothed time per system and mod, in µs."),
    d!("view.saves", "() -> { Save }", "The player's saves, newest first, on the title screen; empty in a game."),
    d!("view.screen", "() -> (number, number)", "Screen width and height in logical pixels."),
    d!("view.selected", "() -> number?", "The selected pawn or thing's id: view.pawn or view.thing says which. With several colonists selected, the first of them."),
    d!("view.selected_zone", "() -> number?", "The stockpile the inspector shows, when no pawn or thing is selected."),
    d!("view.selection", "() -> { number }", "Every selected id: several colonists, or the one pawn or thing, or none."),
    d!("view.shift", "() -> boolean", "Whether Shift is held: a click on a colonist then adds them to the selection."),
    d!("view.show_devtools", "() -> boolean", "Whether devtools are open."),
    d!("view.show_profiler", "() -> boolean", "Whether the profiler is open."),
    d!(
        "view.speech",
        "() -> { Speech }",
        "What pawns are saying now, oldest first: a need's line or a script's rim.say. `age` runs 0 to 1 over the line's life."
    ),
    d!("view.speed", "() -> number", "The game speed."),
    d!("view.stances", "() -> { Stance }", "The colony's stances, in bar order; `active` is the one it's in."),
    d!("view.standing", "() -> { StandingOrder }", "The standing orders: rules on colony readings, with the reading now, their marks (`band`), what they do (`effect`), a season they wait for, whether the reading has crossed the mark, whether the colony has them on, and whether they're moving priorities now (`acting`)."),
    d!("view.stats", "() -> { string }", "Client statistics lines."),
    d!("view.stock", "() -> { StockRow }", "What the colony has, from the stock ledger: one row per thing it has any of, in def order, with units stored and loose, how many stores hold it, and its category."),
    d!("view.store", "(store: number | StoreRef) -> StoreView?", "Everything the store inspector paints, in one read: a stockpile by id (or { zone = id }) or a container ({ thing = id }). Contents are a container's slots in order (an empty one is { empty = true }) or a stockpile's totals by thing and material."),
    d!("view.store_levels", "() -> { string }", "The store priority scale's level names, lowest first."),
    d!("view.stuff", "() -> { Stuff }", "Materials for the active build tool: what you have, what you'd get."),
    d!(
        "view.thing",
        "(id: number) -> ThingInfo?",
        "A thing on the map: a building, plant, rock or item stack, or nil. why says what stops its designated work."
    ),
    d!("view.tick", "() -> number", "The current tick."),
    d!("view.ticks_per_day", "() -> number", "Ticks in a game day."),
    d!("view.time", "() -> number", "Wall-clock seconds, for animation."),
    d!("view.tools", "() -> { Tool }", "Every tool, with the dock category and group it is filed under. A buildable a modifier locks says why in `locked` (empty when it can be placed)."),
    d!("view.ui_scale", "() -> number", "The player's UI scale, on top of the display's (1 is normal)."),
    d!("view.ui_stats", "() -> UiStats", "The UI's own timings."),
    d!("view.ui_tree", "() -> { TreeRow }", "The node tree (devtools)."),
    d!("view.urgent_count", "() -> number", "How many jobs are marked urgent."),
    d!("view.visible_pawns", "() -> { VisiblePawn }", "Pawns on screen, for anchored labels."),
    d!(
        "view.visible_rooms",
        "() -> { VisibleRoom }",
        "Walled rooms on screen, each with the free cell nearest its middle on screen, for their labels. `open` is walled in but not roofed. Empty zoomed out."
    ),
    d!("view.warnings", "() -> { string }", "Load warnings."),
    d!("view.wealth", "() -> number", "The colony's wealth."),
    d!("view.work_types", "() -> { WorkType }", "The work types, in tie-break order."),
    d!("view.zones", "() -> { Zone }", "The stockpiles, oldest first, with how many cells each has, which items it takes, and its level (0 is lowest) and that level's name."),
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
