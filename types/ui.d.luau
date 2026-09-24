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
    size: string?, weight: string?, wrap: boolean?,
    hover: Colors?, press: Colors?, focus: Colors?, focusable: boolean?,
    on_click: (() -> ())?, on_right_click: (() -> ())?, tooltip: string?, disabled: boolean?,
    entity: number?, cell: { number }?, priority: number?, offset: number?,
    handle: ("move" | "resize" | "close")?,
    src: string?, tint: boolean?,
    value: string?, placeholder: string?, on_change: ((text: string) -> ())?, on_submit: ((text: string) -> ())?,
    on_drag: ((fx: number, fy: number) -> ())?,
    [number]: any,
}
type Cell = string | number | { text: string?, bg: string?, color: string? } | nil
type GridProps = {
    rows: number, cols: number, cell: (r: number, c: number) -> Cell,
    cell_w: Size?, cell_h: Size?, gap: Size?, size: string?, weight: string?,
    on_press: ((r: number, c: number) -> any)?, on_paint: ((r: number, c: number, value: any) -> ())?,
    id: string?, bg: string?, border: string?,
}
type ListProps = {
    id: string, count: number, row_h: Size, row: (i: number) -> Node,
    h: Size?, grow: number?, bg: string?, border: string?, pad: Size?,
}
type WindowOpts = { title: string?, w: number?, h: number?, resizable: boolean?, open: boolean? }
type WindowInfo = { id: string, title: string, w: number, h: number, resizable: boolean, comp: string }
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
type TreeRow = { depth: number, kind: string, id: string, owner: string }

declare ui: {
    -- A node attached to a pawn (entity) or cell, on the anchored layer.
    anchored: (node: Node?) -> Node,
    -- Close a window.
    close: (id: string) -> (),
    -- A column: children top to bottom.
    col: (node: Node?) -> Node,
    -- Define a component under a namespaced id.
    define: (id: string, build: (view: any) -> Node?) -> (),
    -- Add children to another component's extension point.
    extend: (id: string, add: any) -> (),
    -- Rows by cols of cells the engine paints as one node. cell(r, c) describes each cell at build; on_press(r, c) returns the value a drag paints and on_paint(r, c, value) runs once per cell the drag enters.
    grid: (props: GridProps) -> Node,
    -- A picture from a mod's ui/img: { src = "mod:name", tint = true }. Its own size unless w/h say otherwise; tint draws it in the text colour. A name@2x.png beside name.png is used on dense displays.
    image: (node: Node) -> Node,
    -- A line of text the player edits: { id = ..., value = ..., placeholder = ..., on_change = fn(text), on_submit = fn(text) }. The engine keeps the buffer by id across rebuilds and reloads; click to focus, Escape to leave.
    input: (node: Node) -> Node,
    -- Whether a window is open.
    is_open: (id: string) -> boolean,
    -- A scroll area that builds only the rows on screen. Needs an id, count, row_h and row(i); spacers stand in for the rows above and below.
    list: (props: ListProps) -> Node,
    -- Show a component on a screen layer. refresh says how often it is rebuilt when nothing forces it: every frame, twenty times a second (the default) or four.
    mount: (layer: Layer, id: string, opts: { order: number?, align: string?, refresh: ("frame" | "fast" | "slow")? }?) -> (),
    -- Open a window (and bring it to the front).
    open: (id: string) -> (),
    -- Hide a node by id.
    remove: (id: string) -> (),
    -- Take over a node by id.
    replace: (id: string, build: (view: any) -> Node?) -> (),
    -- A row: children left to right.
    row: (node: Node?) -> Node,
    -- A column that scrolls.
    scroll: (node: Node?) -> Node,
    -- Keep a value across rebuilds.
    set_state: (key: string, value: any) -> (),
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
    -- Declare a window the engine moves, sizes, stacks and remembers between runs. The component is shown inside the chrome; a function is defined under the window's id.
    window: (id: string, opts: WindowOpts, component: ((view: any) -> Node?) | string) -> (),
    -- The function that draws every window's chrome around ui.slot(win.comp); nodes marked handle = "move", "resize" or "close" are routed by the engine. Core sets it.
    window_chrome: (draw: (win: WindowInfo) -> Node) -> (),
    -- Decorate a node: get its tree, return a new one.
    wrap: (id: string, wrap: (inner: Node, view: any) -> Node?) -> (),
}

declare act: {
    -- Run the game forward (devtools).
    advance: (hours: number) -> (),
    -- Show the next field overlay.
    cycle_overlay: () -> (),
    -- Draft or undraft a colonist.
    draft: (id: number, on: boolean) -> (),
    -- Move the camera to a pawn.
    focus: (id: number) -> (),
    -- Select a pawn, or nothing.
    select: (id: number?) -> (),
    -- Send an event to your mod's own sim scripts ("your_mod:event"), as a player command.
    send: (name: string, data: {[string]: any}?) -> (),
    -- Show a field overlay by its index in view.fields(), or none.
    set_overlay: (index: number?) -> (),
    -- Set the game speed.
    speed: (speed: number) -> (),
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
}

declare view: {
    -- A field's outdoor value, or nil for an unknown field.
    ambient: (field: string) -> number?,
    -- The time of day, "HH:MM".
    clock: () -> string,
    -- The colonists.
    colonists: () -> { Pawn },
    -- Whether every colonist is gone.
    colony_lost: () -> boolean,
    -- Living pawns of "player", "hostile" or "wild".
    count_pawns: (faction: string) -> number,
    -- A copy of data a sim script stored with rim.set_data, or nil.
    data: (key: string) -> any,
    -- The calendar date, counted from 1.
    date: () -> UiDate,
    -- The day, counted from 1.
    day: () -> number,
    -- Recent joins, deaths and departures, newest last.
    events: (since_tick: number) -> { WorldEvent },
    -- Each term and push that makes up a field's outdoor value.
    explain: (field: string) -> { Part },
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
    -- The newest messages, newest first.
    messages: (max: number) -> { Message },
    -- Loaded mods, in load order.
    mods: () -> { ModInfo },
    -- Whether layout outlines are on.
    outlines: () -> boolean,
    -- The label of the field overlay shown, if any.
    overlay: () -> string?,
    -- Whether the game is paused.
    paused: () -> boolean,
    -- One pawn, or nil if it's gone.
    pawn: (id: number) -> Pawn?,
    -- Smoothed time per system and mod, in µs.
    profile: () -> { ProfileRow },
    -- Screen width and height in logical pixels.
    screen: () -> (number, number),
    -- The selected pawn's id.
    selected: () -> number?,
    -- Whether devtools are open.
    show_devtools: () -> boolean,
    -- Whether the profiler is open.
    show_profiler: () -> boolean,
    -- The game speed.
    speed: () -> number,
    -- Client statistics lines.
    stats: () -> { string },
    -- Materials for the active build tool: what you have, what you'd get.
    stuff: () -> { Stuff },
    -- The current tick.
    tick: () -> number,
    -- Ticks in a game day.
    ticks_per_day: () -> number,
    -- Wall-clock seconds, for animation.
    time: () -> number,
    -- The toolbar's tools.
    tools: () -> { Tool },
    -- The UI's own timings.
    ui_stats: () -> UiStats,
    -- The node tree (devtools).
    ui_tree: () -> { TreeRow },
    -- Pawns on screen, for anchored labels.
    visible_pawns: () -> { VisiblePawn },
    -- Load warnings.
    warnings: () -> { string },
    -- The colony's wealth.
    wealth: () -> number,
}
