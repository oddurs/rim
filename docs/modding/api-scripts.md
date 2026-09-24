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
| `rim.clear_ambient` | `(field: string, key: string, ease_hours: number?) -> ()` | Ease a named contribution out and remove it. |
| `rim.colonists` | `() -> number` | How many colonists are alive. |
| `rim.colony_center` | `() -> (number?, number?)` | The colonists' average cell, or nil if there are none. |
| `rim.colony_strength` | `() -> number` | Rough melee output of the colony, which raids are weighed against. |
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
| `rim.get_data` | `(key: string) -> any` | A copy of stored script data, or nil. |
| `rim.hour` | `() -> number` | Hour of the day, 0 to 24 (tick 0 is 06:00). |
| `rim.indoors` | `(x: number, y: number) -> boolean` | Whether a cell is inside an enclosed room. |
| `rim.leave_after` | `(id: number, ticks: number) -> ()` | Make a pawn give up and walk off the map after `ticks`. |
| `rim.log` | `(message: string) -> ()` | Print a line to the console, tagged with your mod. |
| `rim.map_size` | `() -> (number, number)` | Map width and height in cells. |
| `rim.message` | `(text: string, kind: MessageKind?) -> ()` | Post a message to the feed (default kind "info"). |
| `rim.near_cell` | `(x: number, y: number, r: number) -> (number?, number?)` | A random open cell within r of (x, y). |
| `rim.on` | `(event: string, fn: (event: {[string]: any}) -> ()) -> ()` | Handle an engine event (`pawn_died`, `season_changed`, ...) or a mod event (`weather:changed`). |
| `rim.push_ambient` | `(field: string, key: string, value: number, hours: number?, ease_hours: number?) -> ()` | Add a named contribution to a field's outdoor value, easing in over ease_hours and expiring after hours (nil: until cleared). |
| `rim.random` | `() -> number` | A number in [0, 1) from the world's random numbers: the same on every machine. |
| `rim.random_int` | `(lo: number, hi: number) -> number` | A whole number from lo to hi inclusive, from the world's random numbers. |
| `rim.room_at` | `(x: number, y: number) -> Room?` | The room at a cell, or nil on a wall or door. |
| `rim.season` | `() -> string` | The current season's name. |
| `rim.seasons` | `{string}` | The calendar's season names, in order. |
| `rim.set_ambient` | `(id: string, value: number?) -> ()` | Pin a field's outdoor value, overriding its terms and pushes; nil unpins. For tests and tools: mods push instead. |
| `rim.set_data` | `(key: string, value: any) -> ()` | Keep plain data in the world (hashed, saved, readable by the UI as view.data). Use "your_mod:key". |
| `rim.spawn_item` | `(thing: string, x: number, y: number, count: number) -> number` | Drop items near a cell, merging into stacks; returns how many didn't fit. |
| `rim.spawn_pawn` | `(creature: string, faction: Faction, x: number, y: number, name: string?) -> (number?, string?)` | Spawn a creature; returns its id and name, or nil if the cell is blocked. |
| `rim.stat` | `(id: number, name: string) -> number?` | A thing's stat by name: its def's base times its material's factor. |
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
type ThingInfo = { id: string, label: string, market_value: number, food: boolean, item: boolean }
type Date = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number, year_fraction: number }
type Room = { id: number, cells: number, enclosed: boolean }
type Part = { label: string, value: number }
```
