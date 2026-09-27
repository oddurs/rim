-- Generated from crates/rim_ui/src/api.rs: don't edit. Regenerate with
-- RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types

type Size = number | string
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
    cost: string, work: number, hp: number,
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
type BoardRole = { index: number, id: string?, label: string, order: number, edited: boolean, planned: boolean, levels: { [string]: number } }
type Board = { levels: number, labels: { string }, high: number, cols: { BoardCol }, rows: { BoardRow }, roles: { BoardRole } }
type StandingOrder = { id: string, label: string, reading: string, value: number?, band: string, effect: string, season: string?, crossed: boolean, enabled: boolean, acting: boolean }
type Markable = { id: number, urgent: boolean, label: string }
type Stance = { id: string, label: string, icon: string, active: boolean }
type Effective = { value: number, why: string }
type Stuff = { id: string, label: string, color: string, have: number, active: boolean, hp: number, work: number }
type Hover = { x: number, y: number, terrain: string, shelter: string, readings: { string }, values: { Reading }, things: { string }, takes: string? }
type Reading = { label: string, value: string }
type WorkWhy = { work: string, level: number, why: string, picked: boolean, dist: number? }
type ProfileRow = { name: string, us: number, mod: boolean }
type ModInfo = { id: string, version: string, name: string }
type ThingInfo = {
    id: number, def: string, label: string, count: number, hp: number, max_hp: number,
    made_of: string?, blueprint: boolean, designated: string?, why: string?, store: boolean,
}
type Save = { path: string, file: string, day: number, colonists: { string }, age: number, error: string? }
type UiStats = { font: string, build_us: number, layout_us: number, paint_us: number, nodes: number, layouts: number }
type Inspect = { id: string, owner: string, kind: string, path: string, x: number, y: number, w: number, h: number }
type TreeRow = { depth: number, kind: string, id: string, owner: string }

declare ui: {
    -- A node attached to a pawn (entity) or cell, on the anchored layer.
    anchored: (node: Node?) -> Node,
    -- A named action with a default key ("space", "f3", "ctrl+k"): it fires from the key when no text input is typing, and from the command palette. With no key it is in the palette alone. The player's keybinds file overrides the key. Two mods binding one id or one key is reported.
    bind: (id: string, opts: { key: string?, label: string? }, run: () -> ()) -> (),
    -- Close a window.
    close: (id: string) -> (),
    -- A column: children top to bottom.
    col: (node: Node?) -> Node,
    -- Define a component under a namespaced id.
    define: (id: string, build: (view: any) -> Node?) -> (),
    -- Add children to another component's extension point.
    extend: (id: string, add: any) -> (),
    -- Give a node (a text input) the keyboard once it is laid out.
    focus: (id: string) -> (),
    -- Rows by cols of cells the engine paints as one node. cell(r, c) describes each cell at build; on_press(r, c) returns the value a drag paints and on_paint(r, c, value) runs once per cell the drag enters; on_wheel(r, c, steps, shift) takes the wheel over a cell; on_key(r, c, key) takes the keys named in keys while the pointer is over a cell, ahead of any binding on them. A cell can carry a bar (0 to 1) along its bottom, its own tooltip (tip), its own text weight, and a dot or ring in its corner (a colour) marking who set it.
    grid: (props: GridProps) -> Node,
    -- A picture from a mod's ui/img: { src = "mod:name", tint = true }. Its own size unless w/h say otherwise; tint draws it in the text colour. A name@2x.png beside name.png is used on dense displays.
    image: (node: Node) -> Node,
    -- A line of text the player edits: { id = ..., value = ..., placeholder = ..., on_change = fn(text), on_submit = fn(text), on_key = fn("up" | "down") }. The engine keeps the buffer by id across rebuilds and reloads; click to focus, Escape to leave.
    input: (node: Node) -> Node,
    -- Whether a window is open.
    is_open: (id: string) -> boolean,
    -- A scroll area that builds only the rows on screen. Needs an id, count, row_h and row(i); spacers stand in for the rows above and below.
    list: (props: ListProps) -> Node,
    -- Show a component on a screen layer. refresh says how often it is rebuilt when nothing forces it: every frame, twenty times a second (the default) or four. slot reserves a side panel's height so its header stays put while its content changes. The float layer places panels over the map between the side columns, and never moves a docked panel.
    mount: (layer: Layer, id: string, opts: { order: number?, align: string?, refresh: ("frame" | "fast" | "slow")?, slot: number? }?) -> (),
    -- What a right-click on a node with a menu subject (menu = { kind, id }) calls, with the point in logical pixels. Core's menus module sets it; the last one set wins.
    on_context: (handler: (subject: { kind: string, id: any }, x: number, y: number) -> ()) -> (),
    -- Open a window (and bring it to the front).
    open: (id: string) -> (),
    -- Hide a node by id.
    remove: (id: string) -> (),
    -- Take over a node by id.
    replace: (id: string, build: (view: any) -> Node?) -> (),
    -- A row: children left to right.
    row: (node: Node?) -> Node,
    -- Run a bound action, as its key would.
    run: (id: string) -> (),
    -- A column that scrolls.
    scroll: (node: Node?) -> Node,
    -- Replace what a text input holds, caret at the end (the buffer is otherwise the player's).
    set_input: (id: string, text: string) -> (),
    -- Keep a value across rebuilds.
    set_state: (key: string, value: any) -> (),
    -- The open sheet's id, or nil: at most one is open.
    sheet: () -> string?,
    -- An extension point other mods fill with ui.extend.
    slot: (id: string) -> Node,
    -- Empty space that grows.
    spacer: (node: Node?) -> Node,
    -- A value kept with ui.set_state, or default.
    state: (key: string, default: any) -> any,
    -- A user-visible string by key: a mod's ui/lang.toml can replace it; until one does, the default.
    t: (key: string, default: string?) -> string,
    -- Text: { "words", size = ..., color = ... }.
    text: (node: Node | string) -> Node,
    -- Open a window if closed, close it if open.
    toggle: (id: string) -> (),
    -- Declare a window the engine moves, sizes, stacks and remembers between runs. The component is shown inside the chrome; a function is defined under the window's id. With sheet = true it is a screen instead: placed in the band between the docked columns, as wide as w allows, one sheet open at a time.
    window: (id: string, opts: WindowOpts, component: ((view: any) -> Node?) | string) -> (),
    -- The function that draws every window's chrome around ui.slot(win.comp); nodes marked handle = "move", "resize" or "close" are routed by the engine. Core sets it.
    window_chrome: (draw: (win: WindowInfo) -> Node) -> (),
    -- Decorate a node: get its tree, return a new one.
    wrap: (id: string, wrap: (inner: Node, view: any) -> Node?) -> (),
}

declare act: {
    -- Run the game forward (devtools).
    advance: (hours: number) -> (),
    -- Put a colonist in a work role, by its index in view.board().roles. Their pins stay.
    assign_role: (id: number, role: number) -> (),
    -- Hand a colonist's work type back: forget their own setting, so they follow what they'd inherit.
    clear_priority: (id: number, work: string) -> (),
    -- Show the next field overlay.
    cycle_overlay: () -> (),
    -- Draft or undraft a colonist.
    draft: (id: number, on: boolean) -> (),
    -- Move the camera to a pawn or thing.
    focus: (id: number) -> (),
    -- Play a save from view.saves() (the title screen).
    load: (path: string) -> (),
    -- Mark a job urgent (a blueprint, a thing or creature marked for work, an order's site), or clear it: everyone takes it a level sooner than its work type.
    mark_urgent: (id: number, on: boolean) -> (),
    -- Start a new colony (the title screen).
    new_colony: () -> (),
    -- Give the selected colonists the order named key at a map spot (a row from view.orders), each one it's on offer to.
    order: (key: string, x: number, y: number, on: number?) -> (),
    -- Point the materials view (view.stuff) at a buildable by its tool key, for a card describing it; nil goes back to the tool in hand.
    preview: (key: string?) -> (),
    -- Draw the world at this fraction of the screen's pixels, 0.25 to 1; the UI stays sharp. Saved for the player.
    render_scale: (scale: number) -> (),
    -- Make a work role of the player's from a colonist's levels (role and pins), keeping what differs from the defaults. It joins the end of view.board().roles.
    role_from_colonist: (label: string, id: number) -> (),
    -- Make a work role of the player's, copying another. It joins the end of view.board().roles.
    role_from_role: (label: string, role: number) -> (),
    -- What a scroll does on the map: 'auto' (a trackpad pans, a wheel zooms), 'zoom' or 'pan'. Saved with the player's settings.
    scroll_mode: (mode: string) -> (),
    -- Select a pawn or thing, or nothing. With add, put a colonist into the selection or take them out of it (a shift-click).
    select: (id: number?, add: boolean?) -> (),
    -- Show a stockpile in the inspector, or none.
    select_zone: (zone: number?) -> (),
    -- Send an event to your mod's own sim scripts ("your_mod:event"), as a player command.
    send: (name: string, data: {[string]: any}?) -> (),
    -- Show a field overlay by its index in view.fields(), or none.
    set_overlay: (index: number?) -> (),
    -- Set a colonist's priority for a work type: 1 first, 0 never.
    set_priority: (id: number, work: string, level: number) -> (),
    -- Set a work role's level for a work type, or nil to leave it to the default. A planned role (Auto) ignores it.
    set_role_priority: (role: number, work: string, level: number?) -> (),
    -- Switch a priority rule (a standing order) off for this colony, or back on.
    set_rule_enabled: (id: string, on: boolean) -> (),
    -- Put the colony in a stance: its priority rules hold until another.
    set_stance: (id: string) -> (),
    -- Set the game speed.
    speed: (speed: number) -> (),
    -- Change what a store takes: a thing, a category of them, a material (on = false refuses it), the condition range in percent (min, max), or everything (all). A stockpile by id, or { zone = id } or { thing = id } for a container.
    store_filter: (store: number | StoreRef, edit: FilterEdit) -> (),
    -- Put a store at a level of the store priority scale (0 is lowest): a stockpile by id, or { zone = id } or { thing = id }. Stacks only move to a higher one.
    store_level: (store: number | StoreRef, level: number) -> (),
    -- Choose the material for the active build tool.
    stuff: (id: string) -> (),
    -- Show or hide devtools.
    toggle_devtools: () -> (),
    -- Show or hide layout outlines (devtools).
    toggle_outlines: () -> (),
    -- Pause or resume.
    toggle_pause: () -> (),
    -- Show or hide the profiler.
    toggle_profiler: () -> (),
    -- Pick a toolbar tool ("designate:core:chop", "build:core:wall").
    tool: (key: string) -> (),
    -- Turn what the build tool will place a quarter turn clockwise (DESIGN.md §6c).
    turn: () -> (),
    -- Set the player's UI scale, 0.75 to 2 on top of the display's; it is saved with their settings.
    ui_scale: (scale: number) -> (),
    -- Take back the last order given: the colonists stop the job it gave them, and a mark it put on something goes.
    undo: () -> (),
    -- Let a stockpile take an item, or stop it.
    zone_allow: (zone: number, item: string, on: boolean) -> (),
    -- Zoom the map by a factor about the middle of the screen (1.12 is one wheel notch in).
    zoom: (factor: number) -> (),
}

declare view: {
    -- A field's outdoor value, or nil for an unknown field.
    ambient: (field: string) -> number?,
    -- Every bound action with its label and current key, in declaration order.
    binds: () -> { Bind },
    -- The Work Board in one read: columns in tie-break order with jobs waiting (`waiting`), colonists on it (`on`) and on it at a high priority (`high`, levels 1 to Board.high); a row per colonist with each cell's base, effective value, why, and skill.
    board: () -> Board,
    -- The time of day, "HH:MM".
    clock: () -> string,
    -- The colonists, or the first max of them (a bar that shows a few should not pay for all of them; view.count_pawns("player") has the total).
    colonists: (max: number?) -> { Pawn },
    -- Whether every colonist is gone.
    colony_lost: () -> boolean,
    -- Whether the screen is small (under 1440 logical pixels wide, UI scale included): core picks denser layouts.
    compact: () -> boolean,
    -- Living pawns of "player", "hostile" or "wild".
    count_pawns: (faction: string) -> number,
    -- A copy of data a sim script stored with rim.set_data, or nil.
    data: (key: string) -> any,
    -- The calendar date, counted from 1.
    date: () -> UiDate,
    -- The day, counted from 1.
    day: () -> number,
    -- A colonist's priority per work type once rules and the stance have had their say, with why: "Build 1 = base 3, Siege -2". Nil if it isn't a pawn.
    effective: (id: number) -> { [string]: Effective }?,
    -- Recent joins, deaths and departures, newest last.
    events: (since_tick: number) -> { WorldEvent },
    -- Each term and push that makes up a field's outdoor value.
    explain: (field: string) -> { Part },
    -- The why panel: each work type in tie-break order with why the colonist would take it or passes it over ("Needs a chopping tool", "Build first"), and which it picks.
    explain_work: (id: number) -> { WorkWhy }?,
    -- The field layers.
    fields: () -> { FieldInfo },
    -- What a right-click would do.
    hint: () -> string?,
    -- Hour of the day, 0 to 24.
    hour: () -> number,
    -- What's under the cursor.
    hover: () -> Hover?,
    -- The node under the cursor (devtools).
    inspect: () -> Inspect?,
    -- The item category tree stores filter by: the top level in order, and each category by id with its children, the items directly in it, and every item under it.
    item_categories: () -> ItemCategories,
    -- Every item def, which a stockpile can take or refuse.
    items: () -> { Item },
    -- The last order given ("Gunnar will deconstruct wall") and how many seconds ago, or nil once it's been undone.
    last_order: () -> { label: string, age: number }?,
    -- A thing's world look, tinted by what it's made of, as an index a token node's `look` takes (kind = "token"; `kit.item` builds one). Made once per thing and material.
    look: (thing: string, made_of: string?) -> number,
    -- The job on a tile an urgent mark could go on, with whether it has one.
    markable: (x: number, y: number) -> Markable?,
    -- How many things each designation has marked, by designation id; ones with none are left out.
    marked: () -> { [string]: number },
    -- How many messages the log holds.
    message_count: () -> number,
    -- The newest messages, newest first; skip that many of the newest to page back through the log.
    messages: (max: number, skip: number?) -> { Message },
    -- Loaded mods, in load order.
    mods: () -> { ModInfo },
    -- Every order the selected colonists could be given at a map spot, merged by key: group 'damaging' for ones that take something away, trailing '2 of 3' when only some can, disabled with a reason when none can. Walks the map, so call it once per menu, not per frame.
    orders: (x: number, y: number, on: number?) -> { caption: string?, actors: number, actor: string?, rows: { { key: string, label: string, group: string, trailing: string?, disabled: string? } } },
    -- Whether layout outlines are on.
    outlines: () -> boolean,
    -- The label of the field overlay shown, if any.
    overlay: () -> string?,
    -- Whether the game is paused.
    paused: () -> boolean,
    -- One pawn, or nil if it's gone.
    pawn: (id: number) -> Pawn?,
    -- Every colonist, lean: what a list of them needs (name, job, health, drafted, idle, selected) and none of the needs or skills view.colonists carries.
    people: () -> { Person },
    -- A colonist's priority per work type, by work type id: 1 first, 0 never. Nil if it isn't a pawn.
    priorities: (id: number) -> { [string]: number }?,
    -- How many priority levels there are; 0 means never.
    priority_levels: () -> number,
    -- Smoothed time per system and mod, in µs.
    profile: () -> { ProfileRow },
    -- The player's saves, newest first, on the title screen; empty in a game.
    saves: () -> { Save },
    -- Screen width and height in logical pixels.
    screen: () -> (number, number),
    -- The selected pawn or thing's id: view.pawn or view.thing says which. With several colonists selected, the first of them.
    selected: () -> number?,
    -- The stockpile the inspector shows, when no pawn or thing is selected.
    selected_zone: () -> number?,
    -- Every selected id: several colonists, or the one pawn or thing, or none.
    selection: () -> { number },
    -- Whether Shift is held: a click on a colonist then adds them to the selection.
    shift: () -> boolean,
    -- Whether devtools are open.
    show_devtools: () -> boolean,
    -- Whether the profiler is open.
    show_profiler: () -> boolean,
    -- What pawns are saying now, oldest first: a need's line or a script's rim.say. `age` runs 0 to 1 over the line's life.
    speech: () -> { Speech },
    -- The game speed.
    speed: () -> number,
    -- The colony's stances, in bar order; `active` is the one it's in.
    stances: () -> { Stance },
    -- The standing orders: rules on colony readings, with the reading now, their marks (`band`), what they do (`effect`), a season they wait for, whether the reading has crossed the mark, whether the colony has them on, and whether they're moving priorities now (`acting`).
    standing: () -> { StandingOrder },
    -- Client statistics lines.
    stats: () -> { string },
    -- Everything the store inspector paints, in one read: a stockpile by id (or { zone = id }) or a container ({ thing = id }). Contents are a container's slots in order (an empty one is { empty = true }) or a stockpile's totals by thing and material.
    store: (store: number | StoreRef) -> StoreView?,
    -- The store priority scale's level names, lowest first.
    store_levels: () -> { string },
    -- Materials for the active build tool: what you have, what you'd get.
    stuff: () -> { Stuff },
    -- A thing on the map: a building, plant, rock or item stack, or nil. why says what stops its designated work.
    thing: (id: number) -> ThingInfo?,
    -- The current tick.
    tick: () -> number,
    -- Ticks in a game day.
    ticks_per_day: () -> number,
    -- Wall-clock seconds, for animation.
    time: () -> number,
    -- Every tool, with the dock category and group it is filed under.
    tools: () -> { Tool },
    -- The player's UI scale, on top of the display's (1 is normal).
    ui_scale: () -> number,
    -- The UI's own timings.
    ui_stats: () -> UiStats,
    -- The node tree (devtools).
    ui_tree: () -> { TreeRow },
    -- How many jobs are marked urgent.
    urgent_count: () -> number,
    -- Pawns on screen, for anchored labels.
    visible_pawns: () -> { VisiblePawn },
    -- Walled rooms on screen, each with the free cell nearest its middle on screen, for their labels. `open` is walled in but not roofed. Empty zoomed out.
    visible_rooms: () -> { VisibleRoom },
    -- Load warnings.
    warnings: () -> { string },
    -- The colony's wealth.
    wealth: () -> number,
    -- The work types, in tie-break order.
    work_types: () -> { WorkType },
    -- The stockpiles, oldest first, with how many cells each has, which items it takes, and its level (0 is lowest) and that level's name.
    zones: () -> { Zone },
}
