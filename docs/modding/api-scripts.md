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
| `rim.colony_strength` | `() -> number` | Rough melee output of the colony, which raids are weighed against: each colonist's damage (skill and the founder's edge included) per second, by health. |
| `rim.count_items` | `(what: ItemQuery) -> number` | Items lying on the map, by thing ({ thing = "core:wood" }) or by tag ({ tag = "knappable" }). |
| `rim.count_pawns` | `(faction: Faction) -> number` | Living pawns of a faction. |
| `rim.creature_defs` | `{CreatureInfo}` | Every creature def. |
| `rim.damage` | `(id: number, hp: number) -> boolean` | Take hit points off a thing; at none left it's destroyed (true). |
| `rim.date` | `() -> Date` | The calendar date. |
| `rim.day` | `() -> number` | Days since the game began, from 0. |
| `rim.defs` | `(kind: string) -> { {[string]: any} }` | Entries of a def kind a mod declared with [[kind]], in load order: "type" for your own kind, "weather:type" for another mod's. |
| `rim.designate` | `(id: number, designation: string?) -> ()` | Mark a thing for work with a designation, as the player's drag would, or clear its mark with nil. |
| `rim.edge_cell` | `() -> (number?, number?)` | A random open cell on the map edge that can reach the colony, from your mod's stream. |
| `rim.emit` | `(name: string, data: {[string]: any}?) -> ()` | Send an event to rim.on handlers in any mod. Only under your own name: "your_mod:event". |
| `rim.every` | `(interval: number, fn: () -> ()) -> ()` | Run fn every `interval` ticks (hooks are staggered). Register at load time. |
| `rim.explain` | `(field: string) -> {Part}` | Each part of a field's outdoor value: its terms, then pushes. |
| `rim.explain_work` | `(id: number) -> { WorkWhy }` | Why a colonist would do what it would, and passes over the rest, work type by work type in tie-break order. `urgent` if the pick is a job the player marked. Empty if it isn't a pawn. |
| `rim.field` | `(id: string, x: number, y: number, z: number?) -> number` | A field's value at a cell (temperature, light, ...), on level z (the surface if nil). |
| `rim.field_add` | `(id: string, x: number, y: number, amount: number, z: number?) -> number` | Add to a stock field at a cell on level z (the surface if nil), within its range; returns the new value. Only stock fields keep what is added. |
| `rim.field_set` | `(id: string, x: number, y: number, value: number, z: number?) -> number` | Set a stock field at a cell on level z (the surface if nil), within its range; returns the new value. |
| `rim.fixture_at` | `(x: number, y: number, z: number?) -> number?` | The thing standing in a cell on level z (the surface if nil): a plant, rock someone works, a building. Nil if none. |
| `rim.floor_at` | `(x: number, y: number, z: number?) -> number?` | The thing on a cell's floor layer, under what stands there. Nil if none. |
| `rim.get_data` | `(key: string) -> any` | A copy of stored script data, or nil. A bare key is your mod's; "weather:forecast" reads another's. |
| `rim.has_tool` | `(tags: { string }) -> boolean` | Whether some tool in the colony, lying about or in a hand, has every one of these tool tags. False for a tag no tool has. |
| `rim.hour` | `() -> number` | Hour of the day, 0 to 24 (tick 0 is 06:00). |
| `rim.indoors` | `(x: number, y: number, z: number?) -> boolean` | Whether a cell is inside an enclosed room, on level z (the surface if nil). |
| `rim.item_categories` | `{ItemCategoryInfo}` | The item category tree stores and bills filter by, in load order. Each lists its children and the items directly in it, by id. |
| `rim.leave_after` | `(id: number, ticks: number) -> ()` | Make a pawn give up and walk off the map after `ticks`. |
| `rim.levels` | `() -> (number, number)` | The lowest level and the highest: 0 is the surface, below is negative (DESIGN.md §6d). |
| `rim.log` | `(message: string) -> ()` | Print a line to the console, tagged with your mod. |
| `rim.map_size` | `() -> (number, number)` | Map width and height in cells. |
| `rim.message` | `(text: string, kind: MessageKind?) -> ()` | Post a message to the feed (default kind "info"). |
| `rim.modifier_defs` | `{ModifierInfo}` | Every modifier def on a loaded thing (the stat pipeline): what it adds to which stat of which thing, whether it's on from the start, and its group. rim.set_modifiers switches a group. |
| `rim.near_cell` | `(x: number, y: number, r: number, z: number?) -> (number?, number?)` | A random open cell within r of (x, y), on level z (the surface if nil), from your mod's stream. |
| `rim.need_defs` | `{NeedInfo}` | Every need def: what satisfies it ("food", "rest", "field") and how many days a full one lasts. |
| `rim.noise` | `(x: number, y: number, scale: number, salt: number?) -> number` | Smooth noise in [0, 1] from the world's seed: patches about `scale` cells across. The same on every machine, so a generated level is too. `salt` gives another pattern. |
| `rim.on` | `(event: string, fn: (event: {[string]: any}) -> ()) -> ()` | Handle an engine event (`pawn_died`, `season_changed`, ...) or a mod event (`weather:changed`). |
| `rim.on_generate_level` | `(z: number, fn: (z: number) -> ()) -> ()` | Make level z (below 0) yourself: fn runs once when a new map is made, after the level's [[stratum]] has filled it, and changes it with rim.set_terrain. One mod per level. Register at load time. |
| `rim.on_migrate` | `(fn: (from_version: string, data: {[string]: any}) -> {[string]: any}) -> ()` | Upgrade your script data from a save made with a different version of your mod: fn gets that version and your data (bare keys) and returns the data to keep. It sees no world: only your data. Runs on load, before any hook. Register at load time. |
| `rim.order` | `(site: number) -> OrderInfo?` | The work order on a thing and how far it's got, or nil. |
| `rim.place` | `(thing: string, x: number, y: number, z: number?) -> number?` | Put a whole thing that isn't an item (items are rim.spawn_item's) in a cell: on the floor layer for a floor, else standing. Nil if the cell's layer is taken. Returns its id. |
| `rim.planner` | `(name: string, fn: (board: WorkBoard) -> { [number]: { [string]: PlanCell \| number } }) -> ()` | Register a planner under your mod's name, for a planned work role (`planner = "mod:name"`). Once an in-game hour the engine calls it with the board (rim.work_board, its members marked) and takes back levels for its members: `{ [colonist id] = { [work] = { level = 2, reason = "..." } } }`. A level changes when two plans in a row agree. Never (0) and pinned cells are refused. Register at load time. |
| `rim.post_order` | `(site: number, order: OrderSpec) -> ()` | Post a work order on a thing (a station): bring what `needs` lists, by thing or by tag, then work `work` ticks there, holding a tool with every tag in `requires`. Colonists take it as `work_type` work. When it's done, `order_done` names what went in; make what it makes then. One order a site at a time. |
| `rim.priority` | `(id: number, work: string) -> number?` | A colonist's priority for a work type, rules and stance included: 1 first, 0 never. Nil if it isn't a pawn. |
| `rim.priority_parts` | `(id: number, work: string) -> { PriorityPart }?` | How a colonist's priority came about: the work type's default, their work role if it sets one, their pin if they have one, then each rule that moved it. The deltas sum to rim.priority. |
| `rim.push_ambient` | `(field: string, key: string, value: number, hours: number?, ease_hours: number?) -> ()` | Add a named contribution to a field's outdoor value, easing in over ease_hours and expiring after hours (nil: until cleared). |
| `rim.random` | `() -> number` | A number in [0, 1) from your mod's own random stream: the same on every machine, and untouched by other mods' draws. |
| `rim.random_int` | `(lo: number, hi: number) -> number` | A whole number from lo to hi inclusive, from your mod's own random stream. |
| `rim.reading` | `(id: string) -> number?` | A colony reading, by qualified id ("core:food_days"); a bare name is your own mod's. Nil until published. |
| `rim.remove` | `(id: number) -> boolean` | Take a thing off the map for good, as if it were never there: false if it's already gone. |
| `rim.room_at` | `(x: number, y: number, z: number?) -> Room?` | The room at a cell on level z (the surface if nil), or nil on a wall or door. `uncovered` counts cells beyond every roof support's span; `role` is the first [[room_role]] it meets, if any. |
| `rim.say` | `(id: number, text: string, ticks: number?, priority: number?) -> ()` | A pawn says something: a speech bubble over it for `ticks` ticks (600 unless given). Higher `priority` wins when it has several lines or the screen is crowded; needs speak at 1, and 2 is the default. Only presentation: nothing in the sim reads it back. |
| `rim.season` | `() -> string` | The current season's name. |
| `rim.seasons` | `{string}` | The calendar's season names, in order. |
| `rim.set_ambient` | `(id: string, value: number?) -> ()` | Pin a field's outdoor value, overriding its terms and pushes; nil unpins. For tests and tools: mods push instead. |
| `rim.set_data` | `(key: string, value: any) -> ()` | Keep plain data in the world (hashed, saved, readable by the UI as view.data). A bare key is your mod's ("state" is "your_mod:state"); you can't write another mod's. |
| `rim.set_modifiers` | `(group: string, on: boolean) -> number` | Switch every modifier your mod declares in `group` on or off: the stat pipeline. Saved with the world. Returns how many there are. |
| `rim.set_reading` | `(id: string, value: number) -> ()` | Publish a colony reading, like "food_days", under your mod's name (kept to thousandths). Rules with `when = { reading = "mod:id", below = ..., until = ... }` switch on and off as it crosses their marks, firing `rule_started` and `rule_stopped`. Publish on your own cadence: hourly is plenty. |
| `rim.set_stance` | `(stance: string) -> ()` | Put the colony in a stance: its priority rules hold until another. For incidents; the player's comes as a command. |
| `rim.set_terrain` | `(x: number, y: number, terrain: string, z: number?) -> ()` | Change the terrain at a cell on level z (the surface if nil): what a level generator uses. |
| `rim.spawn_item` | `(thing: string, x: number, y: number, count: number, stuff: string?, z: number?) -> number` | Drop items near a cell, merging into stacks; returns how many didn't fit. stuff is what they're made of (a flint axe): it sets their hp and quality, and they stack only with the same. z is the level (the surface if nil). |
| `rim.spawn_pawn` | `(creature: string, faction: Faction, x: number, y: number, name: string?, z: number?) -> (number?, string?)` | Spawn a creature on level z (the surface if nil); returns its id and name, or nil if the cell is blocked. |
| `rim.stance` | `() -> string?` | The colony's stance, or nil if no mod defines any. |
| `rim.stat` | `(id: number, name: string) -> number?` | A thing's stat by name: its def's base times its material's factor. |
| `rim.stat_of` | `(thing: string, stat: string) -> number?` | A thing def's stat through the pipeline: what the def says plus every modifier on it that is on. `buildable` is 1 for a buildable, and 0 or less locks it. nil when neither says anything. |
| `rim.stock` | `(what: string \| StockQuery, place: ("stored" \| "loose")?) -> number` | How many the colony has on the map, read from the stock ledger (never counted): a thing by id, or { thing = }, { tag = } or { category = } (an item category and those under it). `place` narrows it to what lies where a stockpile keeps it, or to what doesn't. |
| `rim.store` | `(id: number) -> StoreInfo?` | A container's level (0 is lowest), how many slots it has, and what is in them (slots from 1). Nil for anything that isn't a built container. |
| `rim.store_put` | `(id: number, what: { thing: string, count: number, made_of: string? }) -> number` | Put things into a container, onto its stacks of the same kind first: a caravan unloading, a chest that fills itself. Only what the container can ever take goes in. Returns how many didn't fit. |
| `rim.store_take` | `(id: number, slot: number, count: number) -> number` | Take up to `count` from a container's slot (from 1); they're gone, for the script to account for. Returns how many were taken. |
| `rim.terrain_at` | `(x: number, y: number, z: number?) -> string` | The terrain at a cell on level z (the surface if nil), by id. |
| `rim.terrain_prop` | `(x: number, y: number, name: string, z: number?) -> number` | A property of the terrain at a cell on level z (the surface if nil), as its [[terrain]] props give it: 0 if they don't. |
| `rim.thing` | `(id: number) -> ThingAt?` | A thing by id: what it is and where, or nil if it's gone. |
| `rim.thing_defs` | `{ThingInfo}` | Every thing def. A food's nutrition is the fraction of a full stomach one unit restores. |
| `rim.tick` | `() -> number` | The current tick. A day is `rim.ticks_per_day` ticks. |
| `rim.ticks_per_day` | `number` | Ticks in a game day. |
| `rim.wealth` | `() -> number` | The colony's wealth (recomputed every few hundred ticks). |
| `rim.who_takes` | `(id: number) -> { Taker }` | Who would take the job on a thing next, soonest first, with about how many ticks until they're there: colonists free to choose. `urgent` if the player marked the job. Empty if someone already holds it. |
| `rim.work_board` | `() -> WorkBoard` | What a planner reads: the scale, each work type (in tie-break order) with what's waiting and its `auto` numbers, and each colonist with their role, skills, pins and level before the rules. |
| `rim.work_role` | `(id: number) -> number?` | A colonist's work role, as an index into rim.work_roles(). Nil if it isn't a colonist or the colony has no roles. |
| `rim.work_roles` | `() -> { WorkRoleInfo }` | The colony's work roles in its own order, each with its index (what a colonist's role names), the def it came from (nil for the player's own) and whether the player edited it. |
| `rim.year` | `() -> number` | The year, from 1. |

Types used above:

```lua
type Faction = "player" | "hostile" | "wild"
type MessageKind = "info" | "good" | "threat" | "bad"
type CreatureInfo = { id: string, label: string, intelligent: boolean, aggressive: boolean, flees: boolean, plural: string, market_value: number, max_hp: number, wild: boolean }
type ThingInfo = { id: string, label: string, market_value: number, food: boolean, nutrition: number?, item: boolean, category: string, tags: { string } }
type NeedInfo = { id: string, label: string, satisfier: string, days_to_empty: number }
type ModifierInfo = { id: string, stat: string, thing: string, value: number, reason: string, group: string, on: boolean }
type StoreSlot = { slot: number, thing: string, count: number, made_of: string?, hp: number }
type StoreInfo = { level: number, slots: number, contents: { StoreSlot } }
type StockQuery = { thing: string?, tag: string?, category: string? }
type ItemCategoryInfo = { id: string, label: string, parent: string?, order: number, children: { string }, items: { string } }
type Date = { year: number, season: string, season_index: number, day: number, day_of_year: number, year_days: number, year_fraction: number }
type Room = { id: number, cells: number, enclosed: boolean, uncovered: number, role: string?, role_label: string? }
type PriorityPart = { kind: "default" | "role" | "pin" | "rule", label: string, delta: number }
type WorkRoleInfo = { index: number, id: string?, label: string, edited: boolean }
type BoardWork = { id: string, label: string, skill: string?, waiting: number, per_person: number, weight: number, default: number }
type BoardColonist = { id: number, name: string, role: number?, member: boolean, skills: { [string]: number }, pins: { [string]: number }, base: { [string]: number } }
type WorkBoard = { levels: number, role: number?, work: { BoardWork }, colonists: { BoardColonist } }
type PlanCell = { level: number, reason: string? }
type WorkWhy = { work: string, level: number, why: string, dist: number?, urgent: boolean }
type Taker = { id: number, ticks: number, urgent: boolean }
type Part = { label: string, value: number }
type ItemFilter = { allows: { string }?, refuses: { string }?, hp: { number }? }
type OrderNeed = { thing: string?, tag: string?, count: number, filter: ItemFilter?, alike: boolean? }
type OrderSpec = { label: string, needs: { OrderNeed }, work: number, work_type: string, requires: { string }? }
type OrderInput = { thing: string?, tag: string?, count: number, have: number, match: string?, coming: boolean }
type OrderInfo = { owner: string, label: string, needs: { OrderInput }, work: number, done: number, total: number, requires: { string } }
type ItemQuery = { thing: string?, tag: string? }
type ThingAt = { id: number, thing: string, x: number, y: number, z: number, count: number, blueprint: boolean, hp: number }
```
