# UI API reference: `ui`, `act`, `view`

Generated from `crates/rim_ui/src/api.rs`: don't edit. Regenerate with
`RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types`. Types for
editors are in [`types/ui.d.luau`](../../types/ui.d.luau); the guide is
[Modding the interface](ui.md).

| Name | Type | What it does |
|---|---|---|
| `act.advance` | `(hours: number) -> ()` | Run the game forward (devtools). |
| `act.assign_role` | `(id: number, role: number) -> ()` | Put a colonist in a work role, by its index in view.board().roles. Their pins stay. |
| `act.clear_priority` | `(id: number, work: string) -> ()` | Hand a colonist's work type back: forget their own setting, so they follow what they'd inherit. |
| `act.cycle_overlay` | `() -> ()` | Show the next field overlay. |
| `act.delete_role` | `(role: number) -> ()` | Delete one of the player's own work roles (a role from a mod can't be deleted), by its index in view.board().roles. Its members go to the default role and keep their pins; the roles after it move down one. |
| `act.draft` | `(id: number, on: boolean) -> ()` | Draft or undraft a colonist. |
| `act.focus` | `(id: number) -> ()` | Move the camera to a pawn or thing, and to its level. |
| `act.leave` | `() -> ()` | Save the colony and go back to the title screen. A last snapshot is taken first, as quitting does. |
| `act.level` | `(z: number) -> ()` | Show level z: 0 is the surface, below it is negative. Past the top or bottom level it stays at the last. |
| `act.lighting` | `(preset: string) -> ()` | The lighting preset: 'low', 'medium', 'high', 'ultra', or 'auto', which starts at medium and steps down while the lighting runs slow. Settings the player set by hand under [lighting] stay. Saved for the player. |
| `act.load` | `(path: string) -> ()` | Play a save from view.saves() (the title screen). |
| `act.mark_urgent` | `(id: number, on: boolean) -> ()` | Mark a job urgent (a blueprint, a thing or creature marked for work, an order's site), or clear it: everyone takes it a level sooner than its work type. |
| `act.new_colony` | `() -> ()` | Start a new colony (the title screen). |
| `act.order` | `(key: string, x: number, y: number, on: number?) -> ()` | Give the selected colonists the order named key at a map spot (a row from view.orders), each one it's on offer to. |
| `act.preview` | `(key: string?) -> ()` | Point the materials view (view.stuff) at a buildable by its tool key, for a card describing it; nil goes back to the tool in hand. |
| `act.reduce_motion` | `(on: boolean) -> ()` | Make the map's overlays still: hover, selection and the grid appear and go at once, and marks don't move. Saved for the player. |
| `act.render_scale` | `(scale: number) -> ()` | Draw the world at this fraction of the screen's pixels, 0.25 to 1; the UI stays sharp. Saved for the player. |
| `act.role_from_colonist` | `(label: string, id: number) -> ()` | Make a work role of the player's from a colonist's levels (role and pins), keeping what differs from the defaults. It joins the end of view.board().roles. |
| `act.role_from_role` | `(label: string, role: number) -> ()` | Make a work role of the player's, copying another. It joins the end of view.board().roles. |
| `act.scroll_mode` | `(mode: string) -> ()` | What a scroll does on the map: 'auto' (a trackpad pans, a wheel zooms), 'zoom' or 'pan'. Saved with the player's settings. |
| `act.select` | `(id: number?, add: boolean?) -> ()` | Select a pawn or thing, or nothing. With add, put a colonist into the selection or take them out of it (a shift-click). |
| `act.select_zone` | `(zone: number?) -> ()` | Show a stockpile in the inspector, or none. |
| `act.send` | `(name: string, data: {[string]: any}?) -> ()` | Send an event to your mod's own sim scripts ("your_mod:event"), as a player command. |
| `act.set_overlay` | `(index: number?) -> ()` | Show a field overlay by its index in view.fields(), or none. |
| `act.set_priority` | `(id: number, work: string, level: number) -> ()` | Set a colonist's priority for a work type: 1 first, 0 never. |
| `act.set_role_priority` | `(role: number, work: string, level: number?) -> ()` | Set a work role's level for a work type, or nil to leave it to the default. A planned role (Auto) ignores it. |
| `act.set_rule_enabled` | `(id: string, on: boolean) -> ()` | Switch a priority rule (a standing order) off for this colony, or back on. |
| `act.set_stance` | `(id: string) -> ()` | Put the colony in a stance: its priority rules hold until another. |
| `act.speed` | `(speed: number) -> ()` | Set the game speed. |
| `act.store_filter` | `(store: number \| StoreRef, edit: FilterEdit) -> ()` | Change what a store takes: a thing, a category of them, a material (on = false refuses it), the condition range in percent (min, max), or everything (all). A stockpile by id, or { zone = id } or { thing = id } for a container. |
| `act.store_level` | `(store: number \| StoreRef, level: number) -> ()` | Put a store at a level of the store priority scale (0 is lowest): a stockpile by id, or { zone = id } or { thing = id }. Stacks only move to a higher one. |
| `act.stuff` | `(id: string) -> ()` | Choose the material for the active build tool. |
| `act.toggle_devtools` | `() -> ()` | Show or hide devtools. |
| `act.toggle_measure` | `() -> ()` | Show or hide the measuring grid: every fifth line heavier, labelled along the pointer's row and column. |
| `act.toggle_outlines` | `() -> ()` | Show or hide layout outlines (devtools). |
| `act.toggle_pause` | `() -> ()` | Pause or resume. |
| `act.toggle_profiler` | `() -> ()` | Show or hide the profiler. |
| `act.tool` | `(key: string) -> ()` | Pick a toolbar tool ("designate:core:chop", "build:core:wall"). |
| `act.turn` | `() -> ()` | Turn what the build tool will place a quarter turn clockwise (DESIGN.md §6c). |
| `act.ui_scale` | `(scale: number) -> ()` | Set the player's UI scale, 0.75 to 2 on top of the display's; it is saved with their settings. |
| `act.undo` | `() -> ()` | Take back the last order given: the colonists stop the job it gave them, and a mark it put on something goes. |
| `act.zone_allow` | `(zone: number, item: string, on: boolean) -> ()` | Let a stockpile take an item, or stop it. |
| `act.zone_plant` | `(zone: number, plant: string) -> ()` | Change what a growing zone sows, by the plant's id. |
| `act.zoom` | `(factor: number) -> ()` | Zoom the map by a factor about the middle of the screen (1.12 is one wheel notch in). |
| `ui.anchored` | `(node: Node?) -> Node` | A node attached to a pawn (entity) or cell, on the anchored layer. |
| `ui.bind` | `(id: string, opts: { key: string?, label: string?, when: (() -> boolean)? }, run: () -> ()) -> ()` | A named action with a default key ("space", "f3", "ctrl+k"): it fires from the key when no text input is typing, and from the command palette. With no key it is in the palette alone. With when, the key is the action's only while when returns true; otherwise the key goes on to the game (Tab to the next colonist). The player's keybinds file overrides the key. Two mods binding one id or one key is reported. |
| `ui.close` | `(id: string) -> ()` | Close a window. |
| `ui.col` | `(node: Node?) -> Node` | A column: children top to bottom. |
| `ui.define` | `(id: string, build: (view: any) -> Node?) -> ()` | Define a component under a namespaced id. |
| `ui.extend` | `(id: string, add: any) -> ()` | Add children to another component's extension point. |
| `ui.focus` | `(id: string) -> ()` | Give a node (a text input) the keyboard once it is laid out. |
| `ui.grid` | `(props: GridProps) -> Node` | Rows by cols of cells the engine paints as one node. cell(r, c) describes each cell at build; on_press(r, c) returns the value a drag paints and on_paint(r, c, value) runs once per cell the drag enters; on_wheel(r, c, steps, shift) takes the wheel over a cell; on_key(r, c, key) takes the keys named in keys while the pointer is over a cell, ahead of any binding on them. A cell can carry a bar (0 to 1) along its bottom, its own tooltip (tip), its own text weight, and a dot or ring in its corner (a colour) marking who set it. |
| `ui.image` | `(node: Node) -> Node` | A picture from a mod's ui/img: { src = "mod:name", tint = true }. Its own size unless w/h say otherwise; tint draws it in the text colour. A name@2x.png beside name.png is used on dense displays. |
| `ui.input` | `(node: Node) -> Node` | A line of text the player edits: { id = ..., value = ..., placeholder = ..., on_change = fn(text), on_submit = fn(text), on_key = fn("up" \| "down") }. The engine keeps the buffer by id across rebuilds and reloads; click to focus, Escape to leave. |
| `ui.is_open` | `(id: string) -> boolean` | Whether a window is open. |
| `ui.list` | `(props: ListProps) -> Node` | A scroll area that builds only the rows on screen. Needs an id, count, row_h and row(i); spacers stand in for the rows above and below. |
| `ui.mount` | `(layer: Layer, id: string, opts: { order: number?, align: string?, refresh: ("frame" \| "fast" \| "slow")?, slot: number? }?) -> ()` | Show a component on a screen layer. refresh says how often it is rebuilt when nothing forces it: every frame, twenty times a second (the default) or four. slot reserves a side panel's height so its header stays put while its content changes. The float layer places panels over the map between the side columns, and never moves a docked panel. |
| `ui.on_context` | `(handler: (subject: { kind: string, id: any }, x: number, y: number) -> ()) -> ()` | What a right-click on a node with a menu subject (menu = { kind, id }) calls, with the point in logical pixels. Core's menus module sets it; the last one set wins. |
| `ui.open` | `(id: string) -> ()` | Open a window (and bring it to the front). |
| `ui.remove` | `(id: string) -> ()` | Hide a node by id. |
| `ui.replace` | `(id: string, build: (view: any) -> Node?) -> ()` | Take over a node by id. |
| `ui.row` | `(node: Node?) -> Node` | A row: children left to right. With wrap = true they go on in lines within the row's width. |
| `ui.run` | `(id: string) -> ()` | Run a bound action, as its key would. |
| `ui.scroll` | `(node: Node?) -> Node` | A column that scrolls. |
| `ui.set_input` | `(id: string, text: string) -> ()` | Replace what a text input holds, caret at the end (the buffer is otherwise the player's). |
| `ui.set_state` | `(key: string, value: any) -> ()` | Keep a value across rebuilds. |
| `ui.sheet` | `() -> string?` | The open sheet's id, or nil: at most one is open. |
| `ui.slot` | `(id: string) -> Node` | An extension point other mods fill with ui.extend. |
| `ui.spacer` | `(node: Node?) -> Node` | Empty space that grows. |
| `ui.state` | `(key: string, default: any) -> any` | A value kept with ui.set_state, or default. |
| `ui.t` | `(key: string, default: string?) -> string` | A user-visible string by key: a mod's ui/lang.toml can replace it; until one does, the default. |
| `ui.text` | `(node: Node \| string) -> Node` | Text: { "words", size = ..., color = ... }. |
| `ui.toggle` | `(id: string) -> ()` | Open a window if closed, close it if open. |
| `ui.window` | `(id: string, opts: WindowOpts, component: ((view: any) -> Node?) \| string) -> ()` | Declare a window the engine moves, sizes, stacks and remembers between runs. The component is shown inside the chrome; a function is defined under the window's id. With sheet = true it is a screen instead: placed in the band between the docked columns, as wide as w allows, one sheet open at a time. |
| `ui.window_chrome` | `(draw: (win: WindowInfo) -> Node) -> ()` | The function that draws every window's chrome around ui.slot(win.comp); nodes marked handle = "move", "resize" or "close" are routed by the engine. Core sets it. |
| `ui.wrap` | `(id: string, wrap: (inner: Node, view: any) -> Node?) -> ()` | Decorate a node: get its tree, return a new one. |
| `view.ambient` | `(field: string) -> number?` | A field's outdoor value, or nil for an unknown field. |
| `view.binds` | `() -> { Bind }` | Every bound action with its label and current key, in declaration order. |
| `view.board` | `() -> Board` | The Work Board in one read: columns in tie-break order with jobs waiting (`waiting`), colonists on it (`on`) and on it at a high priority (`high`, levels 1 to Board.high); a row per colonist with each cell's base, effective value, why, and skill. |
| `view.clock` | `() -> string` | The time of day, "HH:MM". |
| `view.colonists` | `(max: number?) -> { Pawn }` | The colonists, or the first max of them (a bar that shows a few should not pay for all of them; view.count_pawns("player") has the total). |
| `view.colony_lost` | `() -> boolean` | Whether every colonist is gone. |
| `view.compact` | `() -> boolean` | Whether the screen is small (under 1440 logical pixels wide, UI scale included): core picks denser layouts. |
| `view.count_pawns` | `(faction: string) -> number` | Living pawns of "player", "hostile" or "wild". |
| `view.crops` | `() -> { Item }` | The plants a growing zone can sow, in load order: each grows, and is raised by a work of its own (`build.by`). |
| `view.data` | `(key: string) -> any` | A copy of data a sim script stored with rim.set_data, or nil. |
| `view.date` | `() -> UiDate` | The calendar date, counted from 1. |
| `view.day` | `() -> number` | The day, counted from 1. |
| `view.effective` | `(id: number) -> { [string]: Effective }?` | A colonist's priority per work type once rules and the stance have had their say, with why: "Build 1 = base 3, Siege -2". Nil if it isn't a pawn. |
| `view.events` | `(since_tick: number) -> { WorldEvent }` | Recent joins, deaths and departures, newest last. |
| `view.explain` | `(field: string) -> { Part }` | Each term and push that makes up a field's outdoor value. |
| `view.explain_work` | `(id: number) -> { WorkWhy }?` | The why panel: each work type in tie-break order with why the colonist would take it or passes it over ("Needs a chopping tool", "Build first"), and which it picks; `urgent` if that pick is a job the player marked. |
| `view.fields` | `() -> { FieldInfo }` | The field layers. |
| `view.frame` | `() -> string` | The live frame budget: the last second's median and worst frame, draw calls and the biggest render pass, refreshed a few times a second. |
| `view.hint` | `() -> string?` | What a right-click would do. |
| `view.hour` | `() -> number` | Hour of the day, 0 to 24. |
| `view.hover` | `() -> Hover?` | What's under the cursor. |
| `view.inspect` | `() -> Inspect?` | The node under the cursor (devtools). |
| `view.item_categories` | `() -> ItemCategories` | The item category tree stores filter by: the top level in order, and each category by id with its children, the items directly in it, and every item under it. |
| `view.items` | `() -> { Item }` | Every item def, which a stockpile can take or refuse. |
| `view.last_order` | `() -> { label: string, age: number, undoable: boolean }?` | The last order given ("Gunnar will deconstruct wall"), how many seconds ago, and whether an undo takes it back; nil once it's been undone. |
| `view.level` | `() -> number` | The level on screen: 0 is the surface, below it is negative. |
| `view.level_of` | `(id: number) -> number?` | The level a pawn or thing is on. |
| `view.levels` | `() -> { { z: number, colonists: number, others: number, reached: boolean } }` | Every level, the highest first: the colonists and other creatures on it, and whether it is reached (the surface and above, or a level a portal goes down to or a pit looks into). |
| `view.look` | `(thing: string, made_of: string?) -> number` | A thing's world look, tinted by what it's made of, as an index a token node's `look` takes (kind = "token"; `kit.item` builds one). Made once per thing and material. |
| `view.markable` | `(x: number, y: number) -> Markable?` | The job on a tile an urgent mark could go on, with whether it has one. |
| `view.marked` | `() -> { [string]: number }` | How many things each designation has marked, by designation id; ones with none are left out. |
| `view.message_count` | `() -> number` | How many messages the log holds. |
| `view.messages` | `(max: number, skip: number?) -> { Message }` | The newest messages, newest first; skip that many of the newest to page back through the log. |
| `view.mods` | `() -> { ModInfo }` | Loaded mods, in load order. |
| `view.orders` | `(x: number, y: number, on: number?) -> { caption: string?, actors: number, actor: string?, rows: { { key: string, label: string, group: string, trailing: string?, disabled: string? } } }` | Every order the selected colonists could be given at a map spot, merged by key: group 'damaging' for ones that take something away, trailing '2 of 3' when only some can, disabled with a reason when none can. Walks the map, so call it once per menu, not per frame. |
| `view.outlines` | `() -> boolean` | Whether layout outlines are on. |
| `view.overlay` | `() -> string?` | The label of the overlay shown, if any: a field's, or "Storage". |
| `view.paused` | `() -> boolean` | Whether the game is paused. |
| `view.pawn` | `(id: number) -> Pawn?` | One pawn, or nil if it's gone. |
| `view.people` | `() -> { Person }` | Every colonist, lean: what a list of them needs (name, job, health, drafted, idle, selected) and none of the needs or skills view.colonists carries. |
| `view.priorities` | `(id: number) -> { [string]: number }?` | A colonist's priority per work type, by work type id: 1 first, 0 never. Nil if it isn't a pawn. |
| `view.priority_levels` | `() -> number` | How many priority levels there are; 0 means never. |
| `view.profile` | `() -> { ProfileRow }` | Smoothed time per system and mod, in µs. |
| `view.saves` | `() -> { Save }` | The player's saves, newest first, on the title screen; empty in a game. |
| `view.screen` | `() -> (number, number)` | Screen width and height in logical pixels. |
| `view.selected` | `() -> number?` | The selected pawn or thing's id: view.pawn or view.thing says which. With several colonists selected, the first of them. |
| `view.selected_zone` | `() -> number?` | The stockpile the inspector shows, when no pawn or thing is selected. |
| `view.selection` | `() -> { number }` | Every selected id: several colonists, or the one pawn or thing, or none. |
| `view.shift` | `() -> boolean` | Whether Shift is held: a click on a colonist then adds them to the selection. |
| `view.show_devtools` | `() -> boolean` | Whether devtools are open. |
| `view.show_profiler` | `() -> boolean` | Whether the profiler is open. |
| `view.speech` | `() -> { Speech }` | What pawns are saying now, oldest first: a need's line or a script's rim.say. `age` runs 0 to 1 over the line's life. |
| `view.speed` | `() -> number` | The game speed. |
| `view.stances` | `() -> { Stance }` | The colony's stances, in bar order; `active` is the one it's in. |
| `view.standing` | `() -> { StandingOrder }` | The standing orders: rules on colony readings, with the reading now, their marks (`band`), what they do (`effect`), a season they wait for, whether the reading has crossed the mark, whether the colony has them on, and whether they're moving priorities now (`acting`). |
| `view.stats` | `() -> { string }` | Client statistics lines. |
| `view.stock` | `() -> { StockRow }` | What the colony has, from the stock ledger: one row per thing it has any of, in def order, with units stored and loose, how many stores hold it, and its category. |
| `view.store` | `(store: number \| StoreRef) -> StoreView?` | Everything the store inspector paints, in one read: a stockpile by id (or { zone = id }) or a container ({ thing = id }). Contents are a container's slots in order (an empty one is { empty = true }) or a stockpile's totals by thing and material. |
| `view.store_levels` | `() -> { string }` | The store priority scale's level names, lowest first. |
| `view.stuff` | `() -> { Stuff }` | Materials for the active build tool: what you have, what you'd get. |
| `view.thing` | `(id: number) -> ThingInfo?` | A thing on the map: a building, plant, rock or item stack, or nil. why says what stops its designated work. |
| `view.tick` | `() -> number` | The current tick. |
| `view.ticks_per_day` | `() -> number` | Ticks in a game day. |
| `view.time` | `() -> number` | Wall-clock seconds, for animation. |
| `view.tools` | `() -> { Tool }` | Every tool, with the dock category and group it is filed under. A buildable a modifier locks says why in `locked` (empty when it can be placed). |
| `view.ui_scale` | `() -> number` | The player's UI scale, on top of the display's (1 is normal). |
| `view.ui_stats` | `() -> UiStats` | The UI's own timings. |
| `view.ui_tree` | `() -> { TreeRow }` | The node tree (devtools). |
| `view.urgent_count` | `() -> number` | How many jobs are marked urgent. |
| `view.visible_pawns` | `() -> { VisiblePawn }` | Pawns on screen, for anchored labels. |
| `view.visible_rooms` | `() -> { VisibleRoom }` | Walled rooms on screen, each with the free cell nearest its middle on screen, for their labels. `open` is walled in but not roofed. Empty zoomed out. |
| `view.warnings` | `() -> { string }` | Load warnings. |
| `view.wealth` | `() -> number` | The colony's wealth. |
| `view.work_types` | `() -> { WorkType }` | The work types, in tie-break order. |
| `view.zones` | `() -> { Zone }` | The zones, oldest first: stockpiles, with how many cells each has, which items it takes, and its level (0 is lowest) and that level's name; and growing zones, which say the `plant` they sow and take nothing. |
