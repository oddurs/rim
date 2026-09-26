-- Generated from crates/rim_sim/src/script.rs: don't edit. Regenerate with
-- RIM_UPDATE_TYPES=1 cargo test -p rim_sim --test api_types

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

declare rim: {
    -- A field's outdoor value.
    ambient: (id: string) -> number,
    -- The engine's plugin API version, "MAJOR.MINOR".
    api_version: string,
    -- Take your work order off its site. What was brought is put back down there. False if it had none.
    cancel_order: (site: number) -> boolean,
    -- Ease a named contribution out and remove it.
    clear_ambient: (field: string, key: string, ease_hours: number?) -> (),
    -- How many colonists are alive.
    colonists: () -> number,
    -- The colonists' average cell, or nil if there are none.
    colony_center: () -> (number?, number?),
    -- Rough melee output of the colony, which raids are weighed against.
    colony_strength: () -> number,
    -- Items lying on the map, by thing ({ thing = "core:wood" }) or by tag ({ tag = "knappable" }).
    count_items: (what: ItemQuery) -> number,
    -- Living pawns of a faction.
    count_pawns: (faction: Faction) -> number,
    -- Every creature def.
    creature_defs: {CreatureInfo},
    -- The calendar date.
    date: () -> Date,
    -- Days since the game began, from 0.
    day: () -> number,
    -- Entries of a def kind a mod declared with [[kind]], in load order: "type" for your own kind, "weather:type" for another mod's.
    defs: (kind: string) -> { {[string]: any} },
    -- A random open cell on the map edge that can reach the colony.
    edge_cell: () -> (number?, number?),
    -- Send an event to rim.on handlers in any mod. Only under your own name: "your_mod:event".
    emit: (name: string, data: {[string]: any}?) -> (),
    -- Run fn every `interval` ticks (hooks are staggered). Register at load time.
    every: (interval: number, fn: () -> ()) -> (),
    -- Each part of a field's outdoor value: its terms, then pushes.
    explain: (field: string) -> {Part},
    -- A field's value at a cell (temperature, light, ...).
    field: (id: string, x: number, y: number) -> number,
    -- A copy of stored script data, or nil. A bare key is your mod's; "weather:forecast" reads another's.
    get_data: (key: string) -> any,
    -- Whether some tool in the colony, lying about or in a hand, has every one of these tool tags. False for a tag no tool has.
    has_tool: (tags: { string }) -> boolean,
    -- Hour of the day, 0 to 24 (tick 0 is 06:00).
    hour: () -> number,
    -- Whether a cell is inside an enclosed room.
    indoors: (x: number, y: number) -> boolean,
    -- Make a pawn give up and walk off the map after `ticks`.
    leave_after: (id: number, ticks: number) -> (),
    -- Print a line to the console, tagged with your mod.
    log: (message: string) -> (),
    -- Map width and height in cells.
    map_size: () -> (number, number),
    -- Post a message to the feed (default kind "info").
    message: (text: string, kind: MessageKind?) -> (),
    -- A random open cell within r of (x, y).
    near_cell: (x: number, y: number, r: number) -> (number?, number?),
    -- Handle an engine event (`pawn_died`, `season_changed`, ...) or a mod event (`weather:changed`).
    on: (event: string, fn: (event: {[string]: any}) -> ()) -> (),
    -- Upgrade your script data from a save made with a different version of your mod: fn gets that version and your data (bare keys) and returns the data to keep. It sees no world: only your data. Runs on load, before any hook. Register at load time.
    on_migrate: (fn: (from_version: string, data: {[string]: any}) -> {[string]: any}) -> (),
    -- The work order on a thing and how far it's got, or nil.
    order: (site: number) -> OrderInfo?,
    -- Post a work order on a thing (a station): bring what `needs` lists, by thing or by tag, then work `work` ticks there, holding a tool with every tag in `requires`. Colonists take it as `work_type` work. When it's done, `order_done` names what went in; make what it makes then. One order a site at a time.
    post_order: (site: number, order: OrderSpec) -> (),
    -- A colonist's priority for a work type, rules and stance included: 1 first, 0 never. Nil if it isn't a pawn.
    priority: (id: number, work: string) -> number?,
    -- How a colonist's priority came about: the base, then each rule that moved it. The deltas sum to rim.priority.
    priority_parts: (id: number, work: string) -> { PriorityPart }?,
    -- Add a named contribution to a field's outdoor value, easing in over ease_hours and expiring after hours (nil: until cleared).
    push_ambient: (field: string, key: string, value: number, hours: number?, ease_hours: number?) -> (),
    -- A number in [0, 1) from the world's random numbers: the same on every machine.
    random: () -> number,
    -- A whole number from lo to hi inclusive, from the world's random numbers.
    random_int: (lo: number, hi: number) -> number,
    -- The room at a cell, or nil on a wall or door.
    room_at: (x: number, y: number) -> Room?,
    -- The current season's name.
    season: () -> string,
    -- The calendar's season names, in order.
    seasons: {string},
    -- Pin a field's outdoor value, overriding its terms and pushes; nil unpins. For tests and tools: mods push instead.
    set_ambient: (id: string, value: number?) -> (),
    -- Keep plain data in the world (hashed, saved, readable by the UI as view.data). A bare key is your mod's ("state" is "your_mod:state"); you can't write another mod's.
    set_data: (key: string, value: any) -> (),
    -- Put the colony in a stance: its priority rules hold until another. For incidents; the player's comes as a command.
    set_stance: (stance: string) -> (),
    -- Drop items near a cell, merging into stacks; returns how many didn't fit. stuff is what they're made of (a flint axe): it sets their hp and quality, and they stack only with the same.
    spawn_item: (thing: string, x: number, y: number, count: number, stuff: string?) -> number,
    -- Spawn a creature; returns its id and name, or nil if the cell is blocked.
    spawn_pawn: (creature: string, faction: Faction, x: number, y: number, name: string?) -> (number?, string?),
    -- The colony's stance, or nil if no mod defines any.
    stance: () -> string?,
    -- A thing's stat by name: its def's base times its material's factor.
    stat: (id: number, name: string) -> number?,
    -- A thing by id: what it is and where, or nil if it's gone.
    thing: (id: number) -> ThingAt?,
    -- Every thing def.
    thing_defs: {ThingInfo},
    -- The current tick. A day is `rim.ticks_per_day` ticks.
    tick: () -> number,
    -- Ticks in a game day.
    ticks_per_day: number,
    -- The colony's wealth (recomputed every few hundred ticks).
    wealth: () -> number,
    -- The year, from 1.
    year: () -> number,
}
