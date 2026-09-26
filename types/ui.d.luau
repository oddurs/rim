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
    on_key: ((key: string) -> ())?,
    on_drag: ((fx: number, fy: number) -> ())?,
    [number]: any,
}
type Cell = string | number | { text: string?, bg: string?, color: string?, bar: number?, bar_color: string?, tip: string? } | nil
type GridProps = {
    rows: number, cols: number, cell: (r: number, c: number) -> Cell,
    cell_w: Size?, cell_h: Size?, gap: Size?, size: string?, weight: string?,
    on_press: ((r: number, c: number) -> any)?, on_paint: ((r: number, c: number, value: any) -> ())?,
    on_wheel: ((r: number, c: number, steps: number, shift: boolean) -> ())?,
    id: string?, bg: string?, border: string?,
}
type ListProps = {
    id: string, count: number, row_h: Size, row: (i: number) -> Node,
    h: Size?, grow: number?, bg: string?, border: string?, pad: Size?,
}
type WindowOpts = { title: string?, w: number?, h: number?, resizable: boolean?, open: boolean? }
type WindowInfo = { id: string, title: string, w: number, h: number, resizable: boolean, comp: string }
type Bind = { id: string, label: string, key: string, owner: string }
type Layer = "top" | "bottom" | "left" | "right" | "anchored" | "cursor" | "modal" | "windows" | "title"
type Need = { id: string, label: string, value: number, color: string, low: boolean }
type Pawn = {
    id: number, name: string, label: string, faction: string, player: boolean, founder: boolean,
    drafted: boolean, asleep: boolean, hp: number, max_hp: number, health: number, job: string,
    selected: boolean, needs: { Need }, skills: { Skill }, hand: string?, carrying: string?,
}
type Skill = { id: string, label: string, level: number, progress: number, trains: string }
type VisiblePawn = {
    id: number, name: string, faction: string, intelligent: boolean, asleep: boolean,
    selected: boolean, hovered: boolean, radius: number,
}
type Speech = { id: number, text: string, age: number, priority: number }
type Message = { text: string, kind: string, age: number }
type WorldEvent = { tick: number, kind: string, id: number, name: string }
type FieldInfo = {
    index: number, id: string, label: string, unit: string, hud: boolean, overlay: boolean,
    ambient: number, shown: boolean,
}
type UiDate = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number }
type Part = { label: string, value: number }
type Tool = { key: string, label: string, color: string, active: boolean }
type Zone = { id: number, name: string, cells: number, allows: { [string]: boolean } }
type Item = { id: string, label: string, color: string }
type WorkType = { id: string, label: string, icon: string, order: number, default: number }
type BoardCol = { id: string, label: string, icon: string, skill: string?, waiting: number, on: number, high: number }
type BoardCell = { base: number, value: number, why: string, skill: number?, skill_frac: number? }
type BoardRow = { id: number, name: string, job: string, cells: { BoardCell } }
type Board = { levels: number, high: number, cols: { BoardCol }, rows: { BoardRow } }
type Stance = { id: string, label: string, icon: string, active: boolean }
type Effective = { value: number, why: string }
type Stuff = { id: string, label: string, color: string, have: number, active: boolean, hp: number, work: number }
type Hover = { x: number, y: number, terrain: string, shelter: string, readings: { string }, things: { string }, takes: string? }
type WorkWhy = { work: string, level: number, why: string, picked: boolean, dist: number? }
type ProfileRow = { name: string, us: number, mod: boolean }
type ModInfo = { id: string, version: string, name: string }
type ThingInfo = {
    id: number, def: string, label: string, count: number, hp: number, max_hp: number,
    made_of: string?, blueprint: boolean, designated: string?, why: string?,
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
    -- Rows by cols of cells the engine paints as one node. cell(r, c) describes each cell at build; on_press(r, c) returns the value a drag paints and on_paint(r, c, value) runs once per cell the drag enters; on_wheel(r, c, steps, shift) takes the wheel over a cell. A cell can carry a bar (0 to 1) along its bottom and its own tooltip (tip).
    grid: (props: GridProps) -> Node,
    -- A picture from a mod's ui/img: { src = "mod:name", tint = true }. Its own size unless w/h say otherwise; tint draws it in the text colour. A name@2x.png beside name.png is used on dense displays.
    image: (node: Node) -> Node,
    -- A line of text the player edits: { id = ..., value = ..., placeholder = ..., on_change = fn(text), on_submit = fn(text), on_key = fn("up" | "down") }. The engine keeps the buffer by id across rebuilds and reloads; click to focus, Escape to leave.
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
    -- Run a bound action, as its key would.
    run: (id: string) -> (),
    -- A column that scrolls.
    scroll: (node: Node?) -> Node,
    -- Replace what a text input holds, caret at the end (the buffer is otherwise the player's).
    set_input: (id: string, text: string) -> (),
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
    -- Move the camera to a pawn or thing.
    focus: (id: number) -> (),
    -- Play a save from view.saves() (the title screen).
    load: (path: string) -> (),
    -- Start a new colony (the title screen).
    new_colony: () -> (),
    -- Draw the world at this fraction of the screen's pixels, 0.25 to 1; the UI stays sharp. Saved for the player.
    render_scale: (scale: number) -> (),
    -- Select a pawn or thing, or nothing.
    select: (id: number?) -> (),
    -- Send an event to your mod's own sim scripts ("your_mod:event"), as a player command.
    send: (name: string, data: {[string]: any}?) -> (),
    -- Show a field overlay by its index in view.fields(), or none.
    set_overlay: (index: number?) -> (),
    -- Set a colonist's priority for a work type: 1 first, 0 never.
    set_priority: (id: number, work: string, level: number) -> (),
    -- Put the colony in a stance: its priority rules hold until another.
    set_stance: (id: string) -> (),
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
    -- Let a stockpile take an item, or stop it.
    zone_allow: (zone: number, item: string, on: boolean) -> (),
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
    -- Every item def, which a stockpile can take or refuse.
    items: () -> { Item },
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
    -- The selected pawn or thing's id: view.pawn or view.thing says which.
    selected: () -> number?,
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
    -- Client statistics lines.
    stats: () -> { string },
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
    -- The work types, in tie-break order.
    work_types: () -> { WorkType },
    -- The stockpiles, oldest first, with how many cells each has and which items it takes.
    zones: () -> { Zone },
}
