# UI API reference: `ui`, `act`, `view`

Generated from `crates/rim_ui/src/api.rs`: don't edit. Regenerate with
`RIM_UPDATE_TYPES=1 cargo test -p rim_ui --test api_types`. Types for
editors are in [`types/ui.d.luau`](../../types/ui.d.luau); the guide is
[Modding the interface](ui.md).

| Name | Type | What it does |
|---|---|---|
| `act.advance` | `(hours: number) -> ()` | Run the game forward (devtools). |
| `act.cycle_overlay` | `() -> ()` | Show the next field overlay. |
| `act.draft` | `(id: number, on: boolean) -> ()` | Draft or undraft a colonist. |
| `act.focus` | `(id: number) -> ()` | Move the camera to a pawn. |
| `act.select` | `(id: number?) -> ()` | Select a pawn, or nothing. |
| `act.send` | `(name: string, data: {[string]: any}?) -> ()` | Send an event to your mod's own sim scripts ("your_mod:event"), as a player command. |
| `act.set_overlay` | `(index: number?) -> ()` | Show a field overlay by its index in view.fields(), or none. |
| `act.speed` | `(speed: number) -> ()` | Set the game speed. |
| `act.stuff` | `(id: string) -> ()` | Choose the material for the active build tool. |
| `act.toggle_devtools` | `() -> ()` | Show or hide devtools. |
| `act.toggle_outlines` | `() -> ()` | Show or hide layout outlines (devtools). |
| `act.toggle_pause` | `() -> ()` | Pause or resume. |
| `act.toggle_profiler` | `() -> ()` | Show or hide the profiler. |
| `act.tool` | `(key: string) -> ()` | Pick a toolbar tool ("designate:chop", "build:wall"). |
| `ui.anchored` | `(node: Node?) -> Node` | A node attached to a pawn (entity) or cell, on the anchored layer. |
| `ui.col` | `(node: Node?) -> Node` | A column: children top to bottom. |
| `ui.define` | `(id: string, build: (view: any) -> Node?) -> ()` | Define a component under a namespaced id. |
| `ui.extend` | `(id: string, add: any) -> ()` | Add children to another component's extension point. |
| `ui.mount` | `(layer: Layer, id: string, opts: { order: number?, align: string? }?) -> ()` | Show a component on a screen layer. |
| `ui.remove` | `(id: string) -> ()` | Hide a node by id. |
| `ui.replace` | `(id: string, build: (view: any) -> Node?) -> ()` | Take over a node by id. |
| `ui.row` | `(node: Node?) -> Node` | A row: children left to right. |
| `ui.scroll` | `(node: Node?) -> Node` | A column that scrolls. |
| `ui.set_state` | `(key: string, value: any) -> ()` | Keep a value across rebuilds. |
| `ui.slot` | `(id: string) -> Node` | An extension point other mods fill with ui.extend. |
| `ui.spacer` | `(node: Node?) -> Node` | Empty space that grows. |
| `ui.state` | `(key: string, default: any) -> any` | A value kept with ui.set_state, or default. |
| `ui.text` | `(node: Node \| string) -> Node` | Text: { "words", size = ..., color = ... }. |
| `ui.wrap` | `(id: string, wrap: (inner: Node, view: any) -> Node?) -> ()` | Decorate a node: get its tree, return a new one. |
| `view.ambient` | `(field: string) -> number?` | A field's outdoor value, or nil for an unknown field. |
| `view.clock` | `() -> string` | The time of day, "HH:MM". |
| `view.colonists` | `() -> { Pawn }` | The colonists. |
| `view.colony_lost` | `() -> boolean` | Whether every colonist is gone. |
| `view.count_pawns` | `(faction: string) -> number` | Living pawns of "player", "hostile" or "wild". |
| `view.data` | `(key: string) -> any` | A copy of data a sim script stored with rim.set_data, or nil. |
| `view.date` | `() -> UiDate` | The calendar date, counted from 1. |
| `view.day` | `() -> number` | The day, counted from 1. |
| `view.events` | `(since_tick: number) -> { WorldEvent }` | Recent joins, deaths and departures, newest last. |
| `view.explain` | `(field: string) -> { Part }` | Each term and push that makes up a field's outdoor value. |
| `view.fields` | `() -> { FieldInfo }` | The field layers. |
| `view.hint` | `() -> string?` | What a right-click would do. |
| `view.hour` | `() -> number` | Hour of the day, 0 to 24. |
| `view.hover` | `() -> Hover?` | What's under the cursor. |
| `view.inspect` | `() -> Inspect?` | The node under the cursor (devtools). |
| `view.messages` | `(max: number) -> { Message }` | The newest messages, newest first. |
| `view.mods` | `() -> { ModInfo }` | Loaded mods, in load order. |
| `view.outlines` | `() -> boolean` | Whether layout outlines are on. |
| `view.overlay` | `() -> string?` | The label of the field overlay shown, if any. |
| `view.paused` | `() -> boolean` | Whether the game is paused. |
| `view.pawn` | `(id: number) -> Pawn?` | One pawn, or nil if it's gone. |
| `view.profile` | `() -> { ProfileRow }` | Smoothed time per system and mod, in µs. |
| `view.screen` | `() -> (number, number)` | Screen width and height in logical pixels. |
| `view.selected` | `() -> number?` | The selected pawn's id. |
| `view.show_devtools` | `() -> boolean` | Whether devtools are open. |
| `view.show_profiler` | `() -> boolean` | Whether the profiler is open. |
| `view.speed` | `() -> number` | The game speed. |
| `view.stats` | `() -> { string }` | Client statistics lines. |
| `view.stuff` | `() -> { Stuff }` | Materials for the active build tool: what you have, what you'd get. |
| `view.tick` | `() -> number` | The current tick. |
| `view.ticks_per_day` | `() -> number` | Ticks in a game day. |
| `view.time` | `() -> number` | Wall-clock seconds, for animation. |
| `view.tools` | `() -> { Tool }` | The toolbar's tools. |
| `view.ui_stats` | `() -> UiStats` | The UI's own timings. |
| `view.ui_tree` | `() -> { TreeRow }` | The node tree (devtools). |
| `view.visible_pawns` | `() -> { VisiblePawn }` | Pawns on screen, for anchored labels. |
| `view.warnings` | `() -> { string }` | Load warnings. |
| `view.wealth` | `() -> number` | The colony's wealth. |
