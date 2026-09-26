# Script API reference: `rim`

Generated from `crates/rim_sim/src/script.rs`, where each function is
registered: don't edit. Regenerate with
`RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types`. Types for
editors are in [`types/rim.d.luau`](../../types/rim.d.luau); the rules
are in [Scripting rules](scripting.md).

| Name | Type | What it does |
|---|---|---|
| `rim.ambient` | `(id: string) -> number` | A field's outdoor value. |
| `rim.api_version` | `string` | The engine's plugin API version, "MAJOR.MINOR". |
| `rim.cancel_order` | `(site: number) -> boolean` | Take your work order off its site. What was brought is put back down there. False if it had none. |
| `rim.clear_ambient` | `(field: string, key: string, ease_hours: number?) -> ()` | Ease a named contribution out and remove it. |
| `rim.colonists` | `() -> number` | How many colonists are alive. |
| `rim.colony_center` | `() -> (number?, number?)` | The colonists' average cell, or nil if there are none. |
| `rim.colony_strength` | `() -> number` | Rough melee output of the colony, which raids are weighed against. |
| `rim.count_items` | `(what: ItemQuery) -> number` | Items lying on the map, by thing ({ thing = "core:wood" }) or by tag ({ tag = "knappable" }). |
| `rim.count_pawns` | `(faction: Faction) -> number` | Living pawns of a faction. |
| `rim.creature_defs` | `{CreatureInfo}` | Every creature def. |
| `rim.date` | `() -> Date` | The calendar date. |
| `rim.day` | `() -> number` | Days since the game began, from 0. |
| `rim.defs` | `(kind: string) -> { {[string]: any} }` | Entries of a def kind a mod declared with [[kind]], in load order: "type" for your own kind, "weather:type" for another mod's. |
| `rim.edge_cell` | `() -> (number?, number?)` | A random open cell on the map edge that can reach the colony. |
| `rim.emit` | `(name: string, data: {[string]: any}?) -> ()` | Send an event to rim.on handlers in any mod. Only under your own name: "your_mod:event". |
| `rim.every` | `(interval: number, fn: () -> ()) -> ()` | Run fn every `interval` ticks (hooks are staggered). Register at load time. |
| `rim.explain` | `(field: string) -> {Part}` | Each part of a field's outdoor value: its terms, then pushes. |
| `rim.field` | `(id: string, x: number, y: number) -> number` | A field's value at a cell (temperature, light, ...). |
| `rim.get_data` | `(key: string) -> any` | A copy of stored script data, or nil. A bare key is your mod's; "weather:forecast" reads another's. |
| `rim.hour` | `() -> number` | Hour of the day, 0 to 24 (tick 0 is 06:00). |
| `rim.indoors` | `(x: number, y: number) -> boolean` | Whether a cell is inside an enclosed room. |
| `rim.leave_after` | `(id: number, ticks: number) -> ()` | Make a pawn give up and walk off the map after `ticks`. |
| `rim.log` | `(message: string) -> ()` | Print a line to the console, tagged with your mod. |
| `rim.map_size` | `() -> (number, number)` | Map width and height in cells. |
| `rim.message` | `(text: string, kind: MessageKind?) -> ()` | Post a message to the feed (default kind "info"). |
| `rim.near_cell` | `(x: number, y: number, r: number) -> (number?, number?)` | A random open cell within r of (x, y). |
| `rim.on` | `(event: string, fn: (event: {[string]: any}) -> ()) -> ()` | Handle an engine event (`pawn_died`, `season_changed`, ...) or a mod event (`weather:changed`). |
| `rim.on_migrate` | `(fn: (from_version: string, data: {[string]: any}) -> {[string]: any}) -> ()` | Upgrade your script data from a save made with a different version of your mod: fn gets that version and your data (bare keys) and returns the data to keep. It sees no world: only your data. Runs on load, before any hook. Register at load time. |
| `rim.order` | `(site: number) -> OrderInfo?` | The work order on a thing and how far it's got, or nil. |
| `rim.post_order` | `(site: number, order: OrderSpec) -> ()` | Post a work order on a thing (a station): bring what `needs` lists, by thing or by tag, then work `work` ticks there, holding a tool with every tag in `requires`. Colonists take it as `work_type` work. When it's done, `order_done` names what went in; make what it makes then. One order a site at a time. |
| `rim.priority` | `(id: number, work: string) -> number?` | A colonist's priority for a work type, rules and stance included: 1 first, 0 never. Nil if it isn't a pawn. |
| `rim.priority_parts` | `(id: number, work: string) -> { PriorityPart }?` | How a colonist's priority came about: the base, then each rule that moved it. The deltas sum to rim.priority. |
| `rim.push_ambient` | `(field: string, key: string, value: number, hours: number?, ease_hours: number?) -> ()` | Add a named contribution to a field's outdoor value, easing in over ease_hours and expiring after hours (nil: until cleared). |
| `rim.random` | `() -> number` | A number in [0, 1) from the world's random numbers: the same on every machine. |
| `rim.random_int` | `(lo: number, hi: number) -> number` | A whole number from lo to hi inclusive, from the world's random numbers. |
| `rim.room_at` | `(x: number, y: number) -> Room?` | The room at a cell, or nil on a wall or door. |
| `rim.season` | `() -> string` | The current season's name. |
| `rim.seasons` | `{string}` | The calendar's season names, in order. |
| `rim.set_ambient` | `(id: string, value: number?) -> ()` | Pin a field's outdoor value, overriding its terms and pushes; nil unpins. For tests and tools: mods push instead. |
| `rim.set_data` | `(key: string, value: any) -> ()` | Keep plain data in the world (hashed, saved, readable by the UI as view.data). A bare key is your mod's ("state" is "your_mod:state"); you can't write another mod's. |
| `rim.set_stance` | `(stance: string) -> ()` | Put the colony in a stance: its priority rules hold until another. For incidents; the player's comes as a command. |
| `rim.spawn_item` | `(thing: string, x: number, y: number, count: number, stuff: string?) -> number` | Drop items near a cell, merging into stacks; returns how many didn't fit. stuff is what they're made of (a flint axe): it sets their hp and quality, and they stack only with the same. |
| `rim.spawn_pawn` | `(creature: string, faction: Faction, x: number, y: number, name: string?) -> (number?, string?)` | Spawn a creature; returns its id and name, or nil if the cell is blocked. |
| `rim.stance` | `() -> string?` | The colony's stance, or nil if no mod defines any. |
| `rim.stat` | `(id: number, name: string) -> number?` | A thing's stat by name: its def's base times its material's factor. |
| `rim.thing` | `(id: number) -> ThingAt?` | A thing by id: what it is and where, or nil if it's gone. |
| `rim.thing_defs` | `{ThingInfo}` | Every thing def. |
| `rim.tick` | `() -> number` | The current tick. A day is `rim.ticks_per_day` ticks. |
| `rim.ticks_per_day` | `number` | Ticks in a game day. |
| `rim.wealth` | `() -> number` | The colony's wealth (recomputed every few hundred ticks). |
| `rim.year` | `() -> number` | The year, from 1. |

Types used above:

```lua
type Faction = "player" | "hostile" | "wild"
type MessageKind = "info" | "good" | "threat" | "bad"
type CreatureInfo = { id: string, label: string, intelligent: boolean, aggressive: boolean, flees: boolean, plural: string, market_value: number, max_hp: number, wild: boolean }
type ThingInfo = { id: string, label: string, market_value: number, food: boolean, item: boolean, tags: { string } }
type Date = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number, year_fraction: number }
type Room = { id: number, cells: number, enclosed: boolean }
type PriorityPart = { label: string, delta: number }
type Part = { label: string, value: number }
type OrderNeed = { thing: string?, tag: string?, count: number }
type OrderSpec = { label: string, needs: { OrderNeed }, work: number, work_type: string, requires: { string }? }
type OrderInput = { thing: string?, tag: string?, count: number, have: number }
type OrderInfo = { owner: string, label: string, needs: { OrderInput }, work: number, done: number, total: number, requires: { string } }
type ItemQuery = { thing: string?, tag: string? }
type ThingAt = { id: number, thing: string, x: number, y: number, count: number, blueprint: boolean }
```
