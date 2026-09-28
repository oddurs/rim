# RIM — Design

A 2D colony sim. You begin as one naked warrior with nothing. You end as a town
that the world has noticed. The game is a shell, and everything you play is a
plugin, including the base game.

This document records decisions and the arguments behind them. Each section is
written as a tension: the strongest case on each side, then a ruling. When a
ruling turns out wrong, change the ruling and keep the argument.

---

## 1. The one-sentence game

> **Everything you build makes you visible, and visibility brings the world to you: settlers, traders, beasts and raiders.**

The default game has no difficulty slider, and a premise (§4g) sets only
how a run opens, so the game needs one rule that produces its own difficulty
curve. That rule is **Wealth is gravity.** No slider and wealth as gravity
are core's opinions, not the engine's; §6 says where that line is.
Every event in the game is something being pulled toward your colony.

### Tension: wealth-scaled threats punish building

- **For pure wealth scaling:** it's simple, readable and proven. RimWorld does it.
- **Against:** RimWorld players learn to *avoid* building nice things, burn
  wealth, and "wealth-manage". A core loop that punishes the thing the game is
  about is broken.
- **Ruling:** wealth pulls in the good *and* the bad, in the same currency.
  Richer colonies draw better settlers, more traders and rarer opportunities,
  alongside bigger raids. The raid budget also weighs *defensive strength*
  (armed colonists, walls, turrets) against *exposed wealth*. A rich, fortified
  colony reads as a hard target; a rich, naked one reads as prey. Building
  defences is how you "spend" your visibility safely. The scorer is a stat
  pipeline, so plugins can add to either side.

---

## 2. The arc: eras instead of scenarios

A premise (§4g) can choose how a run opens, but never its arc or its
difficulty, so the shape of a run has to come from inside the game.

| Era      | Reached by                          | What the world sends                         |
|----------|-------------------------------------|----------------------------------------------|
| Castaway | start: 1 pawn, nothing              | weather, hunger, predators                   |
| Camp     | shelter + a bed + a fire            | wanderers, small animal threats, lone raiders |
| Hamlet   | 3+ settlers, wealth threshold       | raiding bands, traders, sieges of hunger     |
| Village  | 8+ settlers, wealth threshold       | organised raids, caravans, diplomacy         |
| Town     | 15+ settlers, wealth threshold      | wars, rival towns, legendary events          |

- Eras are **data**: `[[era]]` defs with conditions. Plugins can add, remove or
  re-threshold eras and listen for `era_reached`.
- Eras only go forward. Losing wealth doesn't demote you, but the storyteller
  still reacts to current wealth *inside* an era.
- There is no win screen. The run ends when the last colonist dies, and it
  finishes with a chronicle of what happened.

### Tension: endless sandbox vs. a goal

- **For a goal:** runs without a goal fizzle out, and a goal gives decisions weight.
- **Against:** a fixed goal is a scenario in disguise. Premises (§4g) may set
  the opening, and nothing else.
- **Ruling:** eras give the run direction without a finish line. A plugin can
  add a win condition (build a ship, found a kingdom) as its own event chain.

---

## 3. The founder

You start as *the warrior*: a strong fighter with nothing on them.

- **For making the founder special (founder death = game over):** strong
  identity and real stakes.
- **Against:** one bad wolf fight on day two ends a 30-hour run. That's
  frustrating, not dramatic.
- **Ruling:** the founder has a **Founder** trait (a combat bonus, and settlers
  are more willing to join while they live). Their death is a major colony
  event (a mood blow once mood exists, recorded in the chronicle) but it isn't
  game over as long as anyone else is alive. Early on, before anyone joins, the
  founder *is* the colony, so the stakes are highest exactly when the game is
  simplest.

---

## 4. Why shelter matters

"Build shelter" is meaningless unless the world hurts you outdoors.

- **Ruling:** the core game has **Exposure**. Nights and bad weather drain
  `warmth` for pawns under open sky. An enclosed room (walls and doors, cut off
  from the map edge) protects them. Sleeping in a bed in an enclosed room
  restores rest fastest. This single mechanic gives the first hour its goal:
  *get four walls up before the second night.*
- Temperature, weather and seasons deepen this (§4a, §4c). Roofs, if ever
  wanted, are a plugin: the core only needs "enclosed or not".

### Tension: roofs, or is enclosure enough?

- **For explicit roofs:** a roof is the obvious thing shelter means, and it
  allows open-sided sheds, overhangs and caves.
- **Against:** it's a second thing to build, a second overlay to read, and a
  new player doesn't know about it on night one. The first-hour goal is "four
  walls before dark", and walls should be enough.
- **Ruling:** a **room** is an area bounded by walls, doors, rock or water,
  cut off from the map edge, and roofed everywhere: every cell within span
  of a wall, a door, a window, a pillar or rock (§6c). Enclosed rooms count
  as indoors. The span is what an automatic roof would do: a valley ringed
  by mountains is not a house. (It replaced a cap of 400 cells, which did the
  same job but couldn't be seen or explained.)
  - Doors are a def flag (`door = true`): passable for pathing, but they bound
    rooms like a wall does.
  - Rooms rebuild only when a wall, door or terrain changes, not every tick.
  - Scripts ask with `rim.indoors(x, y)` and `rim.room_at(x, y)`.
  - Explicit roofs, if ever wanted, are a plugin that marks cells roofed and
    hooks the same question.

### Tension: does a room know what it is made of?

- **The question:** `leak_per_hour` was a constant on the *field* def, so every
  room on the map loses heat at the same rate. A room ringed in stone behaves
  exactly like the same room in wood, and a window can only be a hard-coded
  special case. A window is not a special case; it is the first piece of wall
  whose material is the whole point of it.
- **Against deriving it:** rooms would have to walk their boundary, and the
  room rebuild is already one of the few things that touches the whole map.
- **Measured** (0211, 200×200 map, release, best of 6): a full pass over the
  wall cells adding each wall's contribution to the rooms it touches costs
  **0.08–0.09 ms** against a room rebuild of **0.19–0.22 ms** — about +40%,
  flat from 16 huts to 400, because the pass scales with map area rather than
  room count. It runs only when rooms rebuild, which is only when walls
  change, so it is not in the per-tick path.
- **Ruling:** a room's field behaviour **comes from its boundary**.
  - `[[thing]]` may declare what it does to the room it helps enclose
    (`leak`, `daylight`), scaled by its material's factors.
  - Contributions are summed per room during the room rebuild and cached.
  - A field def's constant is the default for a boundary that says nothing,
    so nothing changes for a mod that does not care.
  - This is what makes a window real, and it is the same mechanism wind
    shelter and drafty rooms will want.
  - The pass scans the map. Narrowing it to changed cells belongs with
    incremental region updates (0097), not here.

---

## 4a. Field layers

Temperature, light, beauty, noise, danger and fertility are all the same
thing: a scalar value over the grid that the world produces and pawns and
systems read. So there is one engine mechanism, declared in data, instead of
a special case for each.

- **Ruling:** a `[[field]]` def declares a layer. Its value at a cell is a
  *base* plus *stamped* emitter contributions.
  - **Base:** the outdoor `ambient` under open sky: a sum of labelled terms
    over the time of day and year, plus named contributions from plugins
    (§4c). Inside an enclosed room it
    depends on `indoor`: the room's own value (`room`, for temperature), zero
    (`none`, for light: indoors is dark unless something lights it), or the
    same as outdoors (`outdoor`).
  - **Emitters:** any thing def can `emit = [{ field, amount, radius, cap }]`.
    The contribution fades linearly with *walking* distance, so walls and
    doors block it. Each emitter remembers exactly which cells it touched, so
    adding or removing one costs only its own footprint, and a wall change
    re-stamps only the emitters within reach of it.
  - **Room state:** a `room` field holds one value per enclosed room. It leaks
    toward outdoors at `leak` (insulation; terms, so wind makes a hut
    draftier, or a constant `leak_per_hour`) and is pushed by emitters
    inside at `room_gain` (heating power), up to each emitter's `cap`. These
    are separate on purpose: how well a hut holds warmth and how fast a fire
    heats it are different questions. When walls change and rooms rebuild,
    each new room inherits the cell-weighted average of what its cells held,
    so a wall elsewhere changes nothing.
  - Inside an enclosed room a `room` field is the room's value only; the
    emitter's local stamp applies outdoors. (Adding both counted the same
    fire twice and made huts too hot to be comfortable.)
- **Cost:** O(1) to read a cell; O(footprint) per emitter change; O(rooms)
  per room update (every 60 ticks), not O(cells). Values are integers in
  hundredths, for determinism.
- **Needs** can be driven by a field: `satisfier = "field"` with a `comfort`
  range. The need drains in proportion to how far outside comfort the pawn
  stands and refills inside it. Warmth is the first; a mood plugin could
  drive comfort from beauty the same way.
- **AI** looks for the nearest comfortable cell by walking. Failing that it
  takes the least-uncomfortable cell in reach if it's clearly better than
  where the pawn stands (an unheated hut beats the night outside). Idle
  colonists wait somewhere comfortable instead of wandering in the cold.
- **Scripts:** `rim.field(id, x, y)`, `rim.ambient(id)`, `rim.push_ambient(...)`
  and `rim.explain(id)` (§4c).
  **Client:** `O` cycles an overlay through every field that exists, so a
  mod's new layer gets a map view for free.
- **Scripts must not use `math.sin`/`math.cos`** for simulation values. They
  come from each platform's maths library and can differ in the last bit,
  which would break lockstep. Climate curves are data, evaluated by the engine
  in fixed point (§4c).

### Tuning warmth (balance harness, 40 seeds, 5 days)

Core climate: mean 10°C, swinging 9° either way (about 1°C at 03:00).
Warmth: comfort 10–32°C, drains fully in 0.2 days at 10° outside comfort,
refills in 0.1 days, hypothermia 40 hp/day at zero. Insulation 6%/hour,
campfire 12° with radius 5, capped at 24° in a room.

| Founder's hours at zero warmth after night one | Runs | Hours per run |
|---|---|---|
| No shelter | 40/40 | 14.5 |
| Hut with a bed | 17/40 | 2.6 |
| Hut with a bed and a campfire | 12/40 | 2.8 |

The remaining hut cases are runs where the bot's hut wasn't finished by
day 1. Exposure hurts and shelter fixes it, but a cold night isn't a death
sentence: deaths stay at the pre-warmth baseline. The weather plugin (§4c)
keeps this curve for the first week and brings winter later.

---

## 4b. Fights end in retreat, not death spirals

The first balance pass (`examples/balance.rs`: a bot plays the opening on
many seeds) found that **9 of 20 colonies were wiped out by day 5** and 16
of 20 lost someone. Food was never the problem: nobody went below 23%. Threats
were. Two causes:

- **Every fight was to the death.** Raiders are the same human as colonists,
  so each fight was a coin flip with a corpse at the end.
- **Retreat only worked for one side.** Once raiders could flee, a duel
  harness (`examples/duel.rs`) showed 43 colonist deaths against 1 raider
  death in 100 even 1v1 fights. A wounded raider walks off the map for good;
  a wounded colonist stays nearby and gets picked as a target again.

- **Options considered:** a longer grace period (only delays the wipe);
  weaker raiders (hides the problem and makes combat meaningless); a downed
  state with rescue (right eventually, but it's the Defense milestone).
- **Ruling:** creatures retreat, and retreat is symmetric.
  - `retreat_below` on a creature def (humans 0.3, wolves 0.25): below that
    fraction of max hp, colonists run and heal, while hostiles and hunting
    predators leave the map.
  - **Mercy rule:** nobody picks a wounded or retreating creature as a *new*
    target, and nobody chases one down. Anyone adjacent can still land a blow,
    so being surrounded stays deadly.
  - Colonists defend each other within 20 cells, so they don't get picked off
    one at a time.
  - Raids end: the storyteller gives raiders `rim.leave_after` 0.6 days.
  - Raid size is 80 threat points per raider (was 55), about one colonist's
    worth, so a lone warrior faces one raider and raids grow with wealth.
- **Result**, 40 seeds to day 5: 1 colony lost, 5 runs with a death, and
  **28 near-misses** (someone below 35% hp). Close calls are the norm and
  deaths the exception, which is what we want. Even fights now kill about as
  many raiders as colonists.
- **Unchanged:** need rates (food is never critical, so hunger pressure waits
  for seasons and the Shelter milestone) and the 2-day grace period (first
  threat lands around day 3, earliest day 2.05).
- **Revisit** when downed/rescue arrives in Defense: retreat should become the
  fallback, and being downed the usual outcome of losing a fight.

Re-run `cargo run --release -p rim_sim --example balance -- --seeds 40` after
any change to combat, creatures or incidents.

---

## 4c. Climate and weather

Weather is the world pushing back on a schedule you can see coming. It gives
shelter a second test (storms, winter), gives farming a calendar, and gives
the map a mood.

### Tension: how much weather, and where does it live?

- **For a deep simulation now:** wetness, snow cover, wind shelter,
  feels-like temperature and plant growth all interlock, and farming will
  want them.
- **Against:** nothing reads most of them yet. Building machinery before it
  has a job is how plugin-first projects drown (§6, "costs we accept").
- **Ruling:** build what a player notices now: seasons, weather you can see
  coming, firelit nights, a winter that needs a heated hut. Per-cell ground
  state (wetness, snow cover) and plant growth arrive with farming, their
  first reader, on the same mechanisms.
- **And it's a plugin.** The engine gets general mechanisms; `core` gets the
  shared names; seasons and weather are the first-party plugin
  `mods/weather`, built only on the public API. Remove it and the game plays
  as it did before: day and night, one mild climate.

| Layer | Owns |
|---|---|
| Engine | Terms and curves, the calendar, named contributions to outdoor values, script data and events |
| `core` | The calendar, and the atmosphere fields as shared names: `temperature`, `daylight`, `light`, `cloud`, `precipitation`, `wind`, `wind_dir`, `fog`. The sun as data |
| `mods/weather` | Seasons (by patching core's terms), weather types, the forecast, weather incidents, the weather HUD, and the ground it leaves: wetness (0–100%) and snow (0–60 cm), stock fields in `defs/ground.toml` |

Core declares `precipitation` even though only the weather plugin sets it,
because two plugins must agree on the name: the renderer draws rain from it,
and farming will read it without depending on how it got there.

### One primitive: terms

An outdoor value is a **sum of labelled terms**. A term is a scale times a
product of inputs, each optionally through a piecewise-linear curve:

```toml
[[field]]
id = "temperature"
# ...
[field.ambient.mean]
of = [10.0]
[field.ambient.day]
scale = 9.0
of = [{ input = "hour", curve = [[3, -1.0], [9, 0.0], [15, 1.0], [21, 0.0], [27, -1.0]] }]
```

- **Inputs** (v1): `input = "year"` (0–1), `input = "hour"` (0–24),
  `ambient = "id"` (another field's outdoor value), `field = "id"` (another
  field's value at the cell being read, for a derived field; outdoors, its
  outdoor value), `noise = "key"` (smooth deterministic noise with a period
  of `hours`), and plain numbers. A `kind = "derived"` field is only its
  `value` terms, worked out when read: feels-like is the air less wind chill
  and cold rain, read where the colonist stands.
- **Terrain inputs:** `terrain = "prop"` reads a number the ground gives
  (`[[terrain]] props`: core's fertility, drainage and water table; any
  names a mod adds), and `near = "tag"` how many cells it is to the nearest
  terrain with a tag, up to 16. Only the tags some term reads keep a grid,
  one byte a cell, worked out by one breadth-first search from every tagged
  cell at load. A terrain change patches the cells within 16 of it, by a
  search of the box within 32, so a read is one array lookup and digging a
  pond costs its neighbourhood. Outdoors (no cell), both read as nowhere.
- **Stock fields** (`kind = "stock"`) remember: rain yesterday is wet ground
  today. A stock field stores a value per cell (an `i32` in 1/10000ths, one
  array per field) and changes it by `rate` terms, in its units per game
  hour, read at the cell. The rate may read `self`, the value its `base`
  terms settle to (`base`), and the gap (`above_base`); emitters on the field
  add to the rate; `init` terms give each cell's value when the map is made.
  Each tick works out one slice of the map, `cells × k / P` to
  `cells × (k+1) / P` for tick `k` of a `period_minutes` of `P` ticks, over
  the whole period: every cell exactly once a period, a tick's cost the map
  over the period whatever else happens, and no allocation. A tick's reads
  see the values from before it. Two stock fields on 250×250 cost 0.003 ms a
  tick on synthetic terms; real ones cost more, about 0.4 µs a cell for a
  dozen inputs, which is why a stock field keeps the surface only unless
  it says `levels = "all"` (ore does). Values are in the state hash and the save; `rim.field_add` and
  `rim.field_set` change them. `input = "sky"` (0 in an enclosed room, 1
  elsewhere) is for rain and snow: a cell within a wall's or a cliff's
  roof span is still open ground.
- **Tables keyed by label**, not arrays: a patch can change one term
  (`set = { ambient = { day = { scale = 11.0 } } }`) and conflicts are
  reported per term.
- **Fixed point:** curves compile to integer breakpoints; evaluation is
  integer interpolation, so lockstep holds. No `sin` or `cos` anywhere.
- **Explainable:** `rim.explain("temperature")` and the HUD show
  `−3.2°C = mean −6.0, day +2.1, weather +0.7`.

#### Tension: a term language or a real expression language?

- **For expressions:** more power, familiar infix maths.
- **Against:** a parser, precedence bugs, a tree-walking evaluator where the
  per-cell loop will eventually run, and structure the tooling can't show.
  Curves already cover min, max and thresholds.
- **Ruling:** sums of products of curves. Something it can't express becomes a
  new input kind, a small engine change.

### Named contributions

`rim.push_ambient(field, key, value, hours, ease_hours)` adds a named
contribution on top of a field's terms. It eases in over `ease_hours`, expires
after `hours`, and shows up by name in the breakdown. The weather plugin
pushes `"weather"`; a cold snap pushes `"cold_snap"`. Two mods add up instead
of overwriting each other. `rim.set_ambient` still exists as a pin for tests
and tools: it overrides everything until cleared.

### The sky

The sun is content. Core's one sun is a `daylight` term, and a mod can patch
it, replace it, or add a second sun and a green moon without touching the engine
or the weather plugin.

- **`daylight` holds the sky; `light` holds what reaches the ground.** Core's
  `light` term is `of = [{ ambient = "daylight" }, { ambient = "cloud", curve =
  ... }]`. Sky mods touch only `daylight` terms, and weather touches only
  `cloud`. So a mod that replaces the sun with two keeps the cloud dimming, and
  weather never has to name a sun.
- **Curves, not orbits.** A sky body is a brightness curve over the hour and
  year. An engine that only needs to know how bright it is doesn't need orbital
  mechanics.
- **Day length stays fixed.** A day is `TICKS_PER_DAY` and `hour` runs 0–24,
  because day length is pacing (needs, work, sleep), not astronomy. A planet
  with long days is a curve with 20 bright hours.
- **Later (0209):** `input = "cycle"` with its own period in days, for moon
  phases and eclipses, and a sky tint made of labelled colour terms, one per
  sky body, so a green moon mixes with dusk instead of replacing it. Light
  stays a scalar in the sim; colour is the renderer's business.

### Calendar

`[[calendar]]` in core: a 60-day year of four 15-day seasons, starting on day
9 of spring. The calendar is shared vocabulary (farming, the storyteller and
mood all speak of spring), so core owns it, and a `season_changed` event
fires as each begins. Seasons only *matter* once the weather plugin bends the
temperature around them.

### Script data and events

Plugins keep state in the world, not in Luau locals: `rim.set_data(key,
value)` stores plain data under a namespaced key. It is in the state hash, it
will be saved, and the UI reads it with `view.data(key)`. That's how the
forecast panel sees the weather plugin's queue. `rim.emit(name, table)` sends
a script event to `rim.on` handlers in any mod, named under the sender's
namespace (`weather:changed`).

### The weather plugin

- **Seasons:** patches core's temperature terms: the mean follows a year
  curve (late spring starts like today, around 10°C; midwinter around −6°C),
  and cloud damps the daily swing. Light dims under cloud through core's
  `light` term; the plugin sets `cloud` and never touches `daylight`.
- **Weather types:** clear, cloudy, rain, storm, fog. Each has a duration
  range, blend hours, a weight (a function of the season and the previous
  type) and channel settings. Precipitation below freezing falls as snow, so
  there's no separate snow type: winter rain *is* snow.
- **Forecast:** the current type and the next three. Each is picked with the
  world RNG when it joins the queue, so the forecast is the future that will
  happen unless an incident forces a change. Changes push the channels with
  easing, so rain starts as a drizzle.
- **Incidents:** cold snap, heat wave and storm, through core's storyteller.
- They register through Luau (`weather.register{...}`, from the weather
  module) until plugins can declare their own def kinds (0208), then move to
  data.

### Seeing the weather

- **Light:** the renderer lights the world from the sim's `light` field
  instead of its own curve: daylight, dark rooms, firelight. Brightness is
  the square root of light (an overcast day at half the light still reads as
  day), and firelight is the brighter of sky and fire rather than added, so a
  campfire glows at night and hardly shows at noon. Indoors gets a
  share of daylight, as if through windows. It's drawn as a multiplied
  lightmap, so campfires glow and storms darken the map. (§6e replaces
  this with shadows, flicker and exposure; the sim side is unchanged.) The sky tint is a
  colour curve over the day in data, keyed by label so sky mods can add to it.
- **Weather:** the renderer reads channels, never weather names:
  precipitation (rain, or snow below freezing), wind (slant and drift), fog,
  and lightning in heavy storms. A mod that sets `precipitation` gets rain.
  Nothing falls inside an enclosed room.

### Cost and determinism

- The engine evaluates terms for a handful of fields every 20 ticks: O(fields),
  well under 0.01 ms. The plugin runs a Luau hook every 20 ticks that returns
  at once until the current weather ends (0.2 µs a call; 30-80 µs before it
  cached that). With weather, a tick costs the same as core alone: mean
  0.004-0.005 ms over 5 days (seed 4).
- On screen: 50 µs of CPU for 1,500 raindrops, 3 µs for the lighting pass
  (the firelight texture only rebuilds when emitters change, and the
  occluders only when fixtures, terrain or roofs do).
- Fixed-point terms, the world RNG only for picking weather, and pushes and
  script data in the state hash: a year of weather hashes the same on every run.

### Tuning seasons (balance harness)

The bot builds a 5x5 hut with a bed; `--fire` adds a campfire inside. "Froze"
is the founder's hours at zero warmth after night one.

**The first week must play as §4a.** The first pass made it milder: cloudy
nights stayed warm (cloud damped the daily swing to 45%) and the first night
was sometimes overcast. So the first day is always clear (the first night is
the shelter test core is tuned around), cloud damps the swing only to 80%,
and rain and storms are colder. 5 days:

| Founder froze after night one | Core alone | With weather |
|---|---|---|
| No shelter | 40/40 runs, 12.9 h | 80/80, 10.4 h |
| Hut with a bed | 30/80, 3.1 h | 26/80, 2.0 h |
| Hut, bed and campfire | 7/40, 1.6 h | 23/80, 1.7 h |

Runs with a death: 10/80 either way (an early 40-seed run showed 8 against 2;
at 80 seeds it was noise).

**Winter needs a fire.** Starting on day 38 (late autumn) and playing 20 days
into midwinter, 40 seeds:

| | Colonies alive after winter | Founder alive |
|---|---|---|
| Hut with a bed | 0/40 | 0/40 |
| Hut, bed and campfire | 24/40 | 20/40 |

The deaths in heated runs are mostly wanderers the bot's single hut can't
hold, and raids. Over a whole year the bot, which never builds more than one
hut, loses most colonies with or without weather (core alone: 23/40), so
year-long survival is a question for a better bot, not for the climate.
**A cold snap asks for a fire.** Cold snaps start on day 5: -8°C for two days
outside winter, -12°C in winter, with a warning to light a fire. An unheated
hut leaks toward the outdoor temperature, so a hut alone doesn't carry you
through one; a campfire does. 80 seeds, 8 days, snap on day 5:

| | Colonies lost | Runs with a death |
|---|---|---|
| Hut, no snap | 9/80 | 29/80 |
| Hut, snap (-8°C) | 16/80 | 47/80 |
| Hut, snap at -5°C (tried) | 14/80 | 38/80 |
| Hut and campfire, no snap | 2/80 | 18/80 |
| Hut and campfire, snap (-8°C) | 1/80 | 24/80 |

---

## 4d. Work: who does what

With three colonists the player watches; with thirty they can only set policy.
RimWorld's priority grid shows that a policy can be the fun part: a number per
colonist per work type turns a crowd into something you tune. Its mods show
what players want added: finer levels and priorities that change with the
hour. What it hides is where the grid goes wrong: the leftmost column wins a
tie, nobody can see why a colonist isn't cooking, the grid doesn't change
with winter or a siege, and it doesn't show how much work is waiting.

### Tension: numbers or a ranked list?

- **For numbers:** a grid of 30 colonists by 12 work types is the densest
  control in the genre, and experienced players think in it.
- **For a ranked list:** a newcomer thinks "Ana: doctor, then cook, then
  build". Ordering is easier to reason about than numbers.
- **Ruling:** numbers in the sim, and both views in the UI. The ranked view
  of one colonist writes numbers underneath, so the two never disagree.

### Mechanisms

- **`[[work_type]]`** (core data): id, label, icon, skill, default priority,
  and `order`, the tie-break, which the UI shows and lets you drag.
- **`[[priority_scale]]`:** `levels = 4` in core, named
  `labels = ["First", "Soon", "Later", "Spare time"]`. A mod that wants 9
  patches `levels = 9, labels = []` and the UI follows with numbers; a
  label count that doesn't match fails to load. 0 means never.
- **`[[work_role]]`:** a partial set of levels that colonists belong to,
  one role each. What a role leaves out falls to the work type's default.
  Roles are seeded from defs into the save and edited there; one the player
  never touched follows its mod's updates. A mod ships roles or patches
  core's (`work_role/core:builder`). Named work roles so they never read as
  room roles (§6c). The player deletes only their own roles; a mod's would
  be seeded back on the next load. The roles after a deleted one move down,
  so a role is always its place in the list, and its members go to the
  default role with their pins.
- **Pins:** a colonist's own level for one work type beats their role. It
  exists only while it differs: set it back to the inherited value and it
  is gone.
- **Work finds its type from data.** A designation names its work type
  (`work_type = "chop"`); work the engine hands out itself, like raising
  blueprints, is claimed by the work type that lists it (`jobs = ["build"]`).
  Clearing a thing for a building planned over it (grass under a wall) is
  building, at the priority of the work type that claims `build`. It
  still trains and paces with its designation's skill, and a colonist with
  Chop at 0 will fell a tree a planned wall stands on.
  A colonist without a setting for a work type is at its default, so a
  mod's new work type shows up on every colonist.
- **Effective priority = base + rules.** A `[[priority_rule]]` shifts or
  sets work types while its `when` holds: an hour range, a season, an alert,
  a need, or a stance. Anything a curve can't say is a Luau predicate, cached
  and not run every tick. Each value explains itself, like
  `rim.explain`: `Hauling 2 = base 4, harvest_rush -1, stance Siege -1`.
- **Stances:** named sets of rules ("Normal", "Harvest", "Winter prep",
  "Siege") that change the whole colony with one click. Mods add them.
- **Work pools, not scans.** Work givers post work to a pool per work type
  when something changes (a designation, a blueprint, an item outside a
  stockpile). Pools are bucketed by reachability region and chunk, so
  unreachable work is rejected in O(1). A pawn walks its work types in
  effective-priority order, cached until the grid, a rule or a stance
  changes, and stops at the first level that has reachable work. Inside
  that level an integer score picks: distance, urgency (rot, fire,
  bleeding) and skill fit. That avoids walking across the map for one
  pebble without asking the player to manage it.
- **Luau work givers** post into the same pools, within a per-tick budget.
  Scripts say what work exists; the engine says who does it.
- **Why:** the engine keeps the top few candidates of a choice, with the
  reason each lost (unreachable, reserved, no materials, priority 0), only
  for pawns someone is inspecting.
- Priority changes are `Command`s, rules run in the sim, scores are
  integers: determinism holds.

### Tension: should priorities drive themselves?

- **For setting them by hand:** the grid is where players of the genre tune
  their colony. A system that sets it for them hides the game.
- **For automation:** a castaway player doesn't know what Chop at 2 means,
  flat defaults are wrong for everyone, and at 30 colonists nobody keeps
  200 cells current as the work changes.
- **Ruling:** colonists start on **Auto**, a work role whose levels a
  planner sets each in-game hour from skill and what is waiting, around what
  the rest of the colony already covers. Taking control is a ladder, and
  each rung leaves the ones below it running:

| Rung | The player says | Costs |
|---|---|---|
| Auto | nothing; the colony plans itself | 0 |
| Focus | the colony's situation: a stance, standing orders | 1 click |
| Urgent | this one job, now | 1 click on the map |
| Pin | this colonist, this work type | 1 click per cell |
| Work roles | these colonists work alike | 1 edit per group |

A pin never takes a colonist off Auto: the planner plans their other cells
around it. Joining a fixed role does.

### Tension: is Auto a mode or a role?

- **For a colony-wide switch:** one toggle is easy to explain.
- **For a role:** a switch is all or nothing. Players want a fixed crew of
  builders with everyone else planned, and a new settler to slot in.
- **Ruling:** a role (`planned = true`), the first by `order`, so it is where
  colonists start. Every level resolves the same way:

```
start = Auto's plan | the role's level | the work type's default
base  = the pin, if there is one, else start
level = base, then each rule that holds: set, then shift
```

Two verbs: plans, roles and pins **set**; stances and standing orders
**shift**. `explain` names each part:
`Hunt First = Auto Later (melee 6, covered) · Ana's pin Soon · Food is low −1`.

### What Auto promises

- **It never says never.** Only the player or a rule sets 0.
- **It never moves a pin,** and it counts pins as coverage.
- **It changes slowly.** A planned level changes only when two plans in a
  row agree, so colonists don't swap jobs every hour.
- **It explains itself.** Every planned level carries its reason.
- **Focus has the last word.** Rules shift the plan like any other base.

### How the planner decides

The Auto role names its planner (`planner = "core:auto"`), a Luau function
the engine calls once an in-game hour with the board and whose plan it
checks: a 0 or a pinned cell is an error. Core's is
`mods/core/scripts/auto.luau`; a mod replaces it by patching the role, so
two mods doing it is a loader conflict. The engine stores planned levels and
applies the two-plan rule, and knows nothing of the policy.

1. **Need:** a work type's waiting jobs over its `auto.per_person`, rounded
   up, and at most the colony's size. Nothing waiting, nobody needed.
2. **Gap:** minus the colonists at First or Soon whom the planner doesn't
   move: role members and pins.
3. **Fill:** every gap gets its first person before any gets a second.
   Larger `waiting × auto.weight` goes first. Best skill wins, less a
   penalty for each First already given. At most two First each, then Soon.
4. **Rest:** Later if skilled or if the work needs no skill, Spare time if
   unskilled.

### Standing orders

A standing order is a `[[priority_rule]]` whose `when` reads a colony
**reading**, with a band so it doesn't flap:
`when = { reading = "core:food_days", below = 5, until = 8 }`. Readings are
numbers scripts publish on the sim's clock (`rim.set_reading`), so the
engine knows no content. A reading re-evaluates the rules only when it
crosses a band. Starting and stopping emit events for the news, and a
colony can switch any order off (`SetRuleEnabled`). Core ships three, on by
default: food is low (under 5 days, until 8), loose items piling up (over
30, until 10), and wood before winter (autumn, under 60 wood, until 120;
core's fires burn nothing yet, so wood to build with is winter's want).
`scripts/readings.luau` publishes the readings hourly from the stock
ledger, and each order posts to the news when it starts and stops. A
standing order only shifts; the stance stays the player's choice.

The balance harness (40 seeds, 8 days, the bot's opening) with the orders
on and switched off:

| | Colonies lost | Runs with a death | Near-misses (hp < 35%) |
|---|---|---|---|
| Orders on | 3/40 | 10/40 | 32/40 |
| Orders off | 3/40 | 10/40 | 34/40 |

They cost nothing in the opening and help at the margin; their real work
is later, when stores run down and loose items pile up.

### Urgent marks

`MarkUrgent` flags one job: a blueprint, a designation, an order. Every
colonist takes it one level sooner than its work type, never from 0, and it
wins ties inside its level. Pools keep urgent work in its own bucket, so the
walk stays a walk over levels. The mark clears when the work is done.
The why panel says when a pick came from a mark ("urgent: a level
sooner"), and the map's hover forecast leads with "Urgent" on a marked job.

### The Work Board

A panel in core's UI mod, so a mod can patch or replace it.

- **Painted, not typed.** Drag across cells to paint a value, scroll a cell
  to nudge it, press a number while hovering, shift-scroll a column. A cell
  shows the priority by brightness, the skill as a bar and passion as a
  flame.
- **A cell says who set it:** a ring for Auto, a dot for a pin, nothing for
  the role. Clicking a cell back to the value it would inherit removes the
  pin, and `A` hands it back.
- **Columns show demand:** jobs waiting, the backlog's trend, and coverage.
  A column with work waiting and no one on it at a high priority is marked.
- **Rows show now,** grouped by role with Auto first: what each colonist
  would pick, a 24-hour schedule strip, time idle.
- **Effective values are visible:** a cell reads `2→1` when a rule or stance
  moves it, and hovering explains why.
- **The why panel** on a colonist: what they picked and its score, and each
  work type they passed over with the reason, linked to the map.
- **On the map:** hovering a column lights its waiting jobs; hovering a job
  shows who would take it and when ("Bo in ~20s, then Cyd"). A right-click
  order still forces it, and "Mark urgent" sits beside it.
- **Lenses:** the board, roles, and one person as shelves, registered like
  screens so a mod adds its own (a headcount view, a labour list) that
  writes the same commands.
- **One colonist is a plan, not a grid:** their levels as shelves, each with
  Auto's reason, and a way to pin any of them.

### Does Auto play well?

The balance harness (80 seeds, 8 days, the bot's opening) with colonists on
Auto, in Hand at the defaults ("flat"), and in Hand with a hand-written grid
("tuned": Build First, Harvest and Chop Soon, Hunt and Haul Later, Mine
Spare time). Churn is planned levels that changed, per colonist-day.

| Start | Priorities | Colonies lost | Runs with a death | Churn |
|---|---|---|---|---|
| Castaway | Auto | 4/80 | 23/80 | 0.91 |
| Castaway | Flat | 5/80 | 22/80 | 0 |
| Castaway | Tuned | 4/80 | 21/80 | 0 |
| Six colonists | Auto | 0/80 | 29/80 | 0.58 |
| Six colonists | Flat | 0/80 | 24/80 | 0 |
| Six colonists | Tuned | 0/80 | 23/80 | 0 |

The first run had the castaway's churn at 1.10: a single colonist's rest
levels flipped between Later and Spare time as one job came and went. Two
changes brought it under one a day: the rest is set by skill alone, not by
whether anything waits, and a colonist already at First or Soon for a job
keeps it against someone a level and a half better.

- **Ruling:** Auto ships as the default. It loses no more colonies than
  flat defaults or a tuned grid, and changes a level less than once a day.
- **Deaths, measured again:** with six colonists at 200 seeds, runs with
  a death were Auto 63, flat 57, tuned 60: under one standard deviation
  (about 6.5 runs). The causes match mode for mode (wolves 51, 46 and 47;
  cold 26, 24 and 36; raiders and boars the rest), and colonists died the
  same distance from home (22 to 23 cells) on the same jobs, mostly idle
  or harvesting. Auto doesn't spread colonists into danger; the 80-seed gap
  was noise, and the planner stays as it is.

### Disclosure

Controls appear by colony size and by what the player does, never by an era
id. One colonist: Auto, Focus on the HUD, Urgent in the right-click menu.
Two to four: the board and pins. Five or more: roles, offered when two
colonists have been pinned into the same shape. Fifteen or more: rows fold
by role, and repeated alerts offer new standing orders.

### Cost

Choosing work costs work posted and pools checked, not map size. At 200 pawns
the budget is under 0.2 ms a tick for work choice, measured with a stress map
of every cell designated. The planner runs once an in-game hour over
colonists × work types; its budget is 0.5 ms a run at 30 × 12. Core's
planner takes 147 µs a run at 32 colonists × 7 work types (release build,
`auto_planner` test, `--ignored`), and the profiler shows it as its own
line, `planner:core:auto`.

---

## 4e. Hands first: gathering, tools and crafting

The colonist wakes naked with nothing (§1), yet nothing in the game asks what
they are holding: a bare-handed warrior fells an oak and quarries granite on
the first morning. The first days should be a climb. You gather with your
hands, knap your first edge from flint, and fell your first tree with an axe
you made. The pacing comes from materials and tools you can see in the world,
not from a research screen.

### Tension: a research tree or a tool gate?

- **For research:** a screen of goals is easy to read, and it's the genre's
  habit.
- **For a tool gate:** the gate is in the world. You can see the flint on
  the riverbank, you hold the axe, and losing it matters. It explains itself
  with no extra UI: "needs a chopping tool".
- **Ruling:** tools gate the stone age, and research (a plugin) gates later
  tiers. The engine knows neither: it matches tags.
- **How research gates:** through the stat pipeline (§6). A `[[modifier]]`
  adds a value to a thing's stat while it is on; a buildable starts with
  `buildable = 1`, and one at 0 or less can't be placed (the build order
  is refused, and the dock says the modifier's `reason`). The research
  plugin declares a modifier group per project that takes its unlocks to
  0, and `rim.set_modifiers(group, false)` lets them go when the project
  is studied. The switch is world state, saved by id. The engine knows
  modifiers and stats; research is data and a script (`mods/research`,
  docs/modding/research.md).

### Tension: tools in hand, or tools anywhere in the colony?

- **Anywhere:** one check, no fetching. But one axe then lets ten colonists
  chop, and the axe stops mattering.
- **In hand:** each pawn holds at most one tool and fetches it before the
  work. Three axes fell three trees at once, and wear turns tools into a loop
  of flint, knapping and replacing.
- **Ruling:** in hand. The fetch is a stage of the job. Tags are bits, and
  the tags the colony's tools cover are gathered once per search from the
  few tools there are, so work nobody could do costs a mask test.

### Mechanisms

- **Several harvests on one thing.** An oak can be gathered (branches,
  regrows) and chopped (wood, gone). A patch can add an entry to another
  mod's thing.
- **Tools.** An item with a `tool` block: tags, speed and wear. Its material
  scales speed and hp, so one def makes a flint axe and a bronze one. A
  harvest or an order `requires` tags. Held tools are a component, which is
  the hand slot equipment reuses.
- **Work orders.** "Bring these things, by def or by tag, to this place, then
  work there." A mod posts one from Luau on a site (a station), under a work
  type it names, and hears `order_done` with the inputs and their material.
  Recipes and bills are not an engine concept (§10). Orders are the job they
  need. Blueprints do the same job for building and could become one kind of
  order; they stay as they are until orders have proven themselves.
- **Selecting things.** Any thing can be selected and inspected, and the
  inspector has a slot a mod fills: a station's bills, a tool's wear.

### Content

- **Core** owns the shared names (§5 rule 2): the `gather` designation and the
  tool tags `cutting`, `chopping`, `pounding`, `digging` and `piercing`. Core
  alone gates nothing, so it plays as it did.
- **`mods/crafting`** declares the `recipe` kind, stations (a tag on a thing)
  and bills. Its cheapest station is a free spot on the ground, so no recipe
  needs a "no station" case.
- **`mods/primitive`** is the stone age:
  - branches, fibre, stones, flint, bone and clay;
  - the flake, the hand axe, the hafted axe, the maul and the digging stick;
  - the patches that gate chopping and mining behind tools.

  It lands in tiers. The first is gathering by hand: the materials, the wild
  things they come from, oak branches, and a shelter and campfire of
  branches. The second is tools: a hammerstone, a flint flake and hand
  axe, cordage, and the hafted axe, maul and digging stick, all made at a
  crafting spot. It gates the oak's chop behind `chopping` and granite
  behind `pounding`. The third is clay: banks on the marsh, dug with a
  digging stick and dug out for a few days at a time, build cob walls
  that hold heat better than branches and don't burn, and fire into pots
  at the campfire, which is a crafting station too. The fourth is bone,
  from butchering: it knaps like flint, only worse, so a bone hand axe is
  the same def, slower and weaker.

### The first days

A target for one naked colonist, measured by a seed sweep rather than
assumed.

- **Day one:** gather branches and berries, and build a branch shelter, a
  bed and a campfire. Branches are weak, draughty structural stuff, and
  enough for night one.
- **Day two:** find flint, knap a flake and a hand axe, and cut a digging
  stick.
- **Day three:** fell the first oak and quarry the first stone with a maul.
  Dig clay for warm cob walls, and fire a pot at the campfire.

Measured (`cargo run --release -p rim_sim --example stone_age`, a bot
playing the opening through commands, seeds 1 to 20; 1 to 40 in brackets):

| Step | Share of seeds | Target |
|---|---|---|
| A campfire and an enclosed bed before the first night | 90% (92%) | 90% |
| A flint tool by the end of day 2 | 100% (100%) | 80% |
| A felled tree by the end of day 3 | 100% (98%) | 80% |
| Cob walls by the end of day 4 | 90% (85%) | 60% |

It holds only with a player's sense:
- building first (at equal priorities the nearest work wins, and some grass
  is always nearer than the hut's materials);
- a first hut of 3x3 on ground that can be cleared by hand, not over a tree,
  with a grass pallet (6 fibre) for a bed rather than a timber one (25
  branches);
- flint marked where it's seen, since it's rarely within 20 cells.

What set the numbers is the content: an oak gives 5 branches for 45 work,
deadfall gives 6 and grows on grass, dirt and rich soil at 3%, and the
grass pallet takes the bed off the branch bill. With the old content the
first target was 55%. The misses left are the sparsest maps
(60 to 120 branches by nightfall against a hut's 55 to 60); lifting them
floods the median map, at about 240. The target passes by one seed, so the
next change to wild spawns can move it.

Colonies don't yet last. Raids and wolves end most lone colonies within
days 3 to 45, and storyteller pressure on a stone-age colony is still to tune.

### Cost

A gate is a bitset test. An idle order costs nothing per tick. A scripted
order costs one script call when it completes. Nothing here scans the map.

---

## 4f. Stores and sorting

Mining is the first work that makes things faster than the colony uses
them: one 10×10 room dug out of granite is 1,000 stone, 14 stacks at the
75 limit. Storage has three jobs: **keep** more stacks per cell than the
ground does, **sort** each stack to the best place that takes it, and
**show** what is where.

### Tension: containers as a second item layer, or slots on a fixture?

- **For more stacks per cell:** it's the obvious model, and every system
  that finds items would find a crate's contents.
- **Against:** pathing, rooms, the renderer, `room_for` and the save's
  rebuild of `map.item` all assume one stack per cell (§6a).
- **Ruling:** a **store** is anything that owns slots, a filter and a
  level. A stockpile zone is a store whose slots are its cells. A container
  is a fixture with a `store` block, and its slots live in it; its cell
  has nothing in the item layer. Pawns reach it from its `spots`, so
  pathing learns nothing new.

### Tension: stored stacks as entities, or values?

- **For entities:** scripts can hold an id, and reservations and `MadeOf`
  already work on them.
- **Against:** every stored stack would land in every ECS scan, and
  merging two partial stacks despawns one. Slots would fragment the ECS
  for data that never moves by itself.
- **First ruling:** a slot holds a `Lot`, the value carries and
  deliveries already use, addressed as `(store, slot)`.
- **Changed, building it:** the argument against entities was ECS scans,
  and the stock ledger and store index took those away before containers
  arrived. Measured by the change each asks for, values lose: every job
  that takes a thing (supply, deliver, eat, haul, fetch a tool) would need
  a second kind of source, and reservations a second kind of claim. As
  entities, a stored stack is a stack with a `Contained { store, slot }`
  component at its container's cell, kept off the item layer, so the grid
  still holds one stack per cell, and the jobs need only one helper that
  says where to stand to reach it. Merging still happens in place.

### Sorting only climbs

- A store takes a stack if its filter passes the thing, material and
  condition, and it has room: a free slot, or a partial stack of the same
  thing and material.
- The highest level wins; within a level, the nearest, ties by id.
- A stored stack moves only to a **strictly higher** level. Two stores at
  one level never trade, whatever the distance, so nothing thrashes. To
  pull berries into the kitchen, raise the kitchen's level.
- A store that stops taking something lets it go, to any level.
- Room is reserved by count: a hauler bound for a store holds the room it
  will fill, so nobody scans other pawns' jobs.
- Nothing is lost. A hauler that finds no room goes to the next best
  store, then the nearest open cell.
- Levels are data: `[[store_priority]]` in core, five levels with Normal in
  the middle, like `[[priority_scale]]`.

### One filter

Stores, zones and bills ask the same question, so there is one filter:
the things it takes, the materials it refuses, and a condition range. It
is authored in `[[item_category]]` defs (a tree; an item joins by tag, by
id, or by what it is, and anything unclaimed is under Other); toggling a
category sets the things under it once, so the sim never walks the tree.
Refusing materials rather than listing them means a material a mod adds
later is taken. The filter is small sorted sets, not bitsets: the store
index lists stores by thing, so no search tests a filter against every
thing, and a store's filter is only read to check a stack's material and
condition.

A bill's filter rides on its order's need (`Need::filter`): the engine
checks it on each stack a fetch looks at, beside the need's thing or tag.
An `alike` need is filled in one material: after the first piece the
rest must be its thing made of its material, and the first is only
fetched from a thing the map has enough of. Crafting asks for alike
inputs unless a recipe says `mix = true`, so a knapped axe is two flint
or two bone and the output's material stays one answer, while a stew can
still take any meat.

### What goes where

- **Core** owns the mechanism's shared names: the categories, the level
  scale and the `bulky` tag. Core alone stores in zones.
- **Bulky** things (wood, stone, ore) never go in baskets, crates or
  shelves: the ground, zones, and bulk stores at 2–3× the stack. Mining
  fills yards and bins; crafting fills shelves.
- **Plugins** own every container: `primitive` (basket, pots, woodpile,
  stone bin), `timber` (planks, plank walls, crates, shelves, racks,
  granary) and `iron` (bog iron, charcoal, the bloomery and forge, nails,
  fittings, the saw and the pick). Timber plays without iron: planks are
  hewn with an axe, slowly, and nothing needs nails. Iron depends on
  timber (its saw pit makes planks) and patches timber's builds to add
  nails and fittings. Deep ore is §6d's: iron sits
  at −3 behind `mining`, and bog iron is the surface's poor early source.
- A build may take a material **and** parts (`stuff` and `cost`), and may
  require a tool, so a plank wall is planks and nails.

### Cost

Every question storage asks is answered from an index kept by the change
that affects it, through one choke point for stack changes:

| Index | Answers | Updated when |
|---|---|---|
| ledger, by thing and material | how much the colony has (`rim.stock`) | a stack changes count |
| holdings, by thing and chunk | the nearest flint for a bill | a stack changes count |
| accepts, by thing and level | which stores take this | a filter or level changes |
| room | which stores have space | a slot fills or empties |
| unsorted, by chunk | which stacks have a better place | a stack is dropped, a store changes |
| waiting, by thing | stacks with nowhere to go | a store gains room for that thing |

An idle store costs nothing. A hauler with nowhere to go waits under the
thing's id and wakes only when a store gains room for it. The indexes are
derived and rebuilt on load (§7a). Before this, the haul search cost
0.33 ms a step on 250 × 250 with 30 colonists and 300 loose stacks, over
the 0.2 ms work-choice budget (§4d).

### Showing contents

One **item token** draws every item everywhere: the def's own look, the
count bottom right, a notch for a full stack, a condition bar when
damaged. Containers show their slots; zones show totals by thing and
material, because cells are their limit. The inspector gets Contents and
Accepts tabs, the map a hover card and a storage overlay, and a Stores
sheet shows the colony's totals. Sort orders are registered in the UI VM,
so a mod's order can never desync a game.

---

## 4g. Story: facts, feelings, telling

Every run should leave a story worth retelling, and the story is something
to take apart. Every feeling names its reason, every line of text comes
from a pack anyone can edit, and a story package can be run headless across
a hundred seeds to see what it does.

A story has three layers, and the line between them decides the rest:

| Layer    | Example                                        | Lives in           |
|----------|------------------------------------------------|--------------------|
| Facts    | Arn died on day 14 holding the door            | the sim            |
| Feelings | Sefa grieves; Ode is ashamed, because Ode ran  | the sim, plugins   |
| Telling  | *"Someone should fix his door."*               | the client         |

**Feelings change behaviour; words never do.** A grieving medic works
slower, so grief is simulated. A line of dialogue changes nothing, so text
is presentation: it never enters the sim, costs nothing when nobody looks,
and can come from a template, an author or a model without touching
determinism. A *speech act* (comfort, insult, confess) is a sim event with
effects on opinion; only its wording is telling.

### Tension: scenarios after all?

- **For:** a seed gives a run a reason to exist. "A survey crew whose lander
  came down in the wrong valley, and one of them sabotaged it" is a story
  before day one, and players remember openings.
- **Against:** §1 and §2 ruled scenarios out. They usually bring difficulty
  and goals with them, which wealth-as-gravity and eras replaced.
- **Ruling:** a **premise** sets the opening and never the difficulty.
  `[[start]]` becomes a choice of premises: map constraints, the cast (names,
  traits, skills, who is founder), what they carry, their history with each
  other, and an opening. The schema has no field for the raid budget, the
  wealth scorer or era thresholds. Core's castaway is the default premise,
  so a game with no story mod plays as it does today. Core has no
  difficulty picker. That's core's opinion, not an engine rule (§6): the
  storyteller is a singleton a mod can patch, so a mod can bring its own,
  and the engine never learns the word.

### Tension: can you load more than one?

- **For stacking:** players will want a premise from one author and a feud
  from another, and mods that can't combine split the community.
- **Against:** two openings can't both happen, and two story mods that each
  pace raids would double them.
- **Ruling:** one premise, any number of **threads**, one **writer**. A
  thread is a chain of beats gated by conditions (an era, a day, an event, a
  bond), optionally bound to one premise; with another premise it is skipped
  with a reason. A thread never fires anything itself: a beat is an *offer* to
  the storyteller, which spends one tension budget on offers and its own
  incidents alike. Five threads make a run more varied, not harder.
- A def may **claim** a tag (`claims = ["early_raids"]`). Two enabled defs
  claiming one tag are a contested slot (§10): a compatibility mod settles
  it, else the player's pick, else load order, labelled. A claim never
  blocks a new game. The pick is kept in the save until the modlist
  lockfile lands, then in the lockfile. Claims are generic: any kind can use
  them.

### Who owns what

The engine gets mechanisms that three or more of these plugins share:

- **Perception.** An event def may ask for witnesses: pawns within its
  radius, on its level, in its room or with a clear line to it. Candidates
  come from a **pawn index by chunk**, kept by movement, so nothing scans
  every pawn. Witnesses are computed once, in id order, into the payload.
- **Memories.** The chronicle's store (§2) is the one memory mechanism: a log
  of compact records (kind, tick, place, actors, witnesses, a small data
  table), each with a notability from its event def, indexed by entity. "What
  happened to Arn", "what does this knife remember" and "what happened in
  this room" are lookups. Routine events live in a short ring; notable ones
  are kept, and retention is bounded by size.
- **Relations.** Sparse values between two entities, per kind a mod
  declares (opinion first), each the sum of remembered reasons that decay.
  A pair exists only while it has a reason. Decay runs in one daily batch.
- **Interactions.** Each awake, unbusy pawn considers one on a staggered
  cadence, about once an in-game hour. The partner comes from the pawn
  index, the act is a weighted pick over `[[interaction]]` defs whose weights
  are data (opinion, emotion, bond, traits) evaluated in Rust, and the
  effects are data: a relation reason, a thought. Scripts listen to the
  event, and one budgeted hook may veto.
- **Gatherings.** People at a place for a time, facing a focus: funerals
  first, shared meals and parties later.
- **Bodies.** A death leaves a corpse that remembers who it was, and
  `pawn_died` names the killer.
- **Traits.** A pawn carries trait ids. Core declares the kind and the
  founder; plugins add traits, and traits modify stats through the pipeline.
- **Premises**, and a script hook at game start.

Core owns the shared names: the premise and trait kinds, the castaway, the
corpse, the storyteller's offer queue, notability for its own events, and
English text for them. Four first-party plugins do the rest, each reading
the others as `optional`:

| Plugin     | Owns                                                              |
|------------|-------------------------------------------------------------------|
| `mood`     | thoughts (the mood milestone), emotions, appraisal, contrast, contagion |
| `social`   | opinion, bonds, speech acts, shared experience, secrets           |
| `mourning` | grief, graves, burial, funerals, reminders, grief styles, epitaphs |
| `story`    | threads, beats, procedural premises                               |

### Feelings

- **Emotions are kinds.** A thought carries an emotion (joy, grief, fear,
  anger, disgust, pride, shame, relief) declared as data. Mood is still the
  sum, but a person at −20 from grief acts differently from one at −20 from
  disgust, and speaks differently.
- **Appraisal.** A thought's strength is its base times factors from the
  person's traits: raw meat is nothing to a hunter and an insult to a cook.
- **Contrast.** A thought family can measure against the person's recent
  average, so the first hot meal after a hungry week matters and the
  thirtieth doesn't.
- **Contagion.** Strong emotions spread a little between people in a room,
  per room per in-game hour.
- **Mixed feelings and delayed reactions** need no mechanism: one event can
  raise two thoughts, and a thought can start late.

### Mourning

Grief is a thought with phases (shock, acute, a long tail), scaled by the
bond, not by the death. Rites (burial, a marker, a funeral) shorten the
tail. An unburied body blocks them and gives dread and disgust to whoever
sees it; a body never recovered keeps grief open. **Reminders**: a grieving
person near something the log links to the dead (their bed, their knife,
the grave) gets the grief back, checked on the grief's own cadence and only
for grieving pawns. **Grief styles** (withdraw, keep vigil, overwork, take
over their job, keep a memento, blame someone) are data choosing jobs and
modifiers; blame is a relation reason. Shared rites raise opinion among
those present. The chronicle writes an epitaph from facts.

### Tension: rich simulation or legible simulation?

- **For rich:** more interaction kinds, hidden values, gossip. The surprises
  are the point.
- **Against:** if the player can't see why Hild hates Ode, it reads as
  random, and random isn't a story.
- **Ruling:** legible. Every thought, opinion and memory names its reason and
  the event it came from, and the inspector shows them. Few interaction
  kinds, each with a clear effect.

### Telling

- **Intent in, line out.** The sim records intents: act, speaker, listener,
  topic, facts, feelings. A writer turns an intent into words. Writers live in
  the client, and `rim_text` is shared with the CLI.
- **Templates are the default writer,** and must be good on their own.
  Packs are data per locale (`text/en/*.toml`), keyed by act, emotion and
  bond, with nested grammars and slots filled from the intent's facts, most
  specific first. Voices are data: register, words, and what a voice never
  says, by trait and background. Hand-written lines override templates for
  an exact moment.
- **Same words, nothing saved.** A line's template is picked with a seed from
  the world seed and the event's sequence number, avoiding what the speaker
  said recently. Picks are made as events arrive, which is cheap; text is
  expanded only when a line is seen. A replay shows the same words.
- **A connected writer** is an external process the player points the game
  at, speaking JSON lines over stdio: intent in, line out; prompt in, premise
  package out. It never runs in the sim, the sim never waits, and a slow
  answer falls back to templates. Its lines go in the save's presentation
  sidecar. No model ships with the game.
- **Craft rules**, linted by `rim check` where a linter can: every template
  uses a fact slot; lines have a length cap; slots only offer facts the
  speaker witnessed or was told; a deny-list catches therapy phrasing. The
  rest is for authors: subtext, talking past the event, a voice per person,
  callbacks, silence as a line, rarity.
- Each client renders its own words. In co-op the facts are shared and the
  words may differ.

### Hack it

- `rim story <premise> --seeds 100 --days 120` runs a story headless and
  prints a chronicle and beat statistics: how often each beat fired, and when.
- Text packs reload instantly. Words are presentation, so no replay.
- The inspector shows why a person feels, likes and remembers, and what a
  thing remembers.
- A dev console fires a beat, adds a memory or sets a relation, as commands,
  so replays hold.
- Players act in the story: name people, things and places, hold a funeral,
  pin a moment, rewrite an entry's words.

### Cost

All story systems together stay under **0.15 ms a tick** on the §8 target
map, measured by the bench with the story plugins on. Nothing runs per pawn
per tick.

| System | Cadence | Bounded by |
|---|---|---|
| Perception | per event that asks | pawns in nearby chunks |
| Memories | per event | fixed-size records, bounded retention |
| Interactions | each pawn about hourly, staggered | defs × one partner |
| Relations | a daily decay batch | pairs with a reason |
| Contagion | per room, hourly | occupied rooms |
| Reminders | grieving pawns, hourly | remembered things in their room |
| Threads | daily, and on the events they name | enabled threads |
| Text | client, lines on screen only | lines seen |

Event dispatch gets cheaper first. Today every event builds a Luau table
even with no listener, handlers are found by a linear scan, and each call
allocates its profiler label. Handlers are indexed by name, unheard events
are skipped, and labels are interned.

---

## 5. What's in `core` and what isn't

`core` is the smallest complete game. Everything else is a plugin, including
things we build ourselves.

| In `core`                                     | Out (plugins, first-party or community) |
|-----------------------------------------------|-----------------------------------------|
| Terrain, strata, plants, map generation       | Seasons and weather (`mods/weather`)    |
| Needs: food, rest, warmth                     | Mood, mental breaks, relationships, mourning |
| Calendar, day and night, the atmosphere names | Other biomes, rain runoff, fire spread  |
| Harvest, mine, dig, build, haul (delivery)    | The stone age (`mods/primitive`)        |
|                                               | Bills (`mods/crafting`), research       |
| Melee combat, health, death                   | Ranged weapons, armour, medicine        |
| Wild animals, predators, hunting              | Taming, farming animals                 |
| Storyteller, wealth, eras                     | Trade, factions, diplomacy              |
| The castaway premise, traits, bodies          | Other premises, story threads (§4g)     |
| Incidents: raid, wanderer, herd, predators    | Everything else                         |

`core` is itself a plugin: it loads from `mods/core` like any other mod, and
the engine has no special case for it. What makes it core is that it's the one
plugin everything else builds on. Three rules decide what goes in:

1. **The smallest complete game.** Core alone must play: a map, a castaway,
   hunger, rest, cold nights, building, animals and raids.
2. **Core owns the shared vocabulary.** When two plugins must agree on a
   name, core defines it: `temperature`, `precipitation`, `human`, `wood`,
   the calendar. Plugins meet through core's names instead of depending on
   each other, so farming can read `precipitation` whether or not the
   weather plugin is what sets it. Tags too: a bare tag (`bed`, `chopping`,
   `ore`) is core's, and core declares it; a plugin's own tags carry its
   name (`primitive:knappable`).
3. **Depth goes out.** Anything that deepens a loop rather than completing it
   is a plugin: seasons, weather, mood, crafting chains.

The test: with every plugin removed the game is complete but shallow (a CI
run proves it); with core removed nothing works, because the names are gone.
A total conversion replaces core by declaring those names itself, and a CI
fixture with no core keeps that path real.

- **Tension:** mood is RimWorld's soul, so leaving it out makes the core feel thin.
- **Ruling:** ship mood as the **first first-party plugin** (`rim.mood`). If
  mood can't be built as a plugin, the API is wrong, and that's the test we
  want to run early.

---

## 6. Architecture: engine verbs, plugin nouns

The engine knows *mechanisms*. It never knows *content*.

```
rim_sim (engine)                        mods/core (content)
────────────────                        ───────────────────
harvestable, buildable, edible, bed  ←  tree_oak, wall, berries, bed
need w/ decay + satisfier kind       ←  food, rest
creature w/ stats                    ←  human, deer, wolf
incident registry, scheduler, events ←  storyteller.luau, raid.luau, ...
stat pipeline (wealth, threat)       ←  modifiers from every mod
```

### Three plugin tiers

1. **Data (TOML defs + patches).** Most mods live here. Patches are declarative
   (`target = "thing/wood"`, `set = {...}`). Lists are edited, not owned
   (`append`, `remove`, and `[[patch.edit]]` by a matching key), so two mods
   can both add to one list. The loader **detects conflicts** (two mods setting
   the same field, or one replacing a list another edited) and reports them
   rather than letting the last one win silently.
2. **Script (Luau).** Sandboxed, typed, fast. Behaviour lives here: incidents,
   the storyteller, custom events. Scripts get a deterministic RNG and no I/O.
   Every hook call is timed **per mod**, and the timings are visible in-game (F3).
3. **Native (WASM, later).** For heavy systems such as new pathing or fluid
   simulation. Same API surface, compiled.

### Tension: typed extension points vs. "patch anything"

- **For patch-anything (Harmony-style):** unlimited power. Modders never wait on
  you to add a hook.
- **Against:** it's the main cause of mod conflicts, it destroys performance on
  hot paths, and every engine update breaks mods.
- **Ruling:** named extension points plus a stat pipeline plus events. When
  modders need a hook that doesn't exist, that's a feature request against the
  engine, and it's cheap to add because the engine is small. The API is semver'd
  (`api = "0.1"` in `mod.toml`).

### Tension: which opinions does the engine enforce?

- **For enforcing the game's taste in the engine:** rim has opinions (no
  difficulty slider, wealth as gravity, eras that only go forward), and
  they are the game. A platform that lets any mod undo them has no identity.
- **Against:** a modder can get round an opinion the engine enforces only by
  forking, or by the patch-anything workaround the engine exists to
  prevent. And core is a plugin (§5). If its opinions were engine rules,
  that would stop being true.
- **Ruling:** **the engine enforces only what someone else relies on.** Ask
  of any rule: if a mod broke it, who else would pay?
  - **Someone else** (co-op peers, a replay, a bug report, other mods,
    trust in the index): it's a **wall**, and the engine holds it. The grid
    and the tick (§6a), determinism (§7), no native code and no I/O (§10),
    load order from manifests (§10), named extension points instead of
    patch-anything (above), declarative defs (§10), and a UI that never
    writes the sim (§11). A mod that needs to cross one is a fork, and we
    say so.
  - **Only the player who installed the mod:** it's an **opinion**, and it
    lives in core or the default game (§10), where a mod can change it. No
    difficulty slider (§1), wealth as gravity (§1), eras that only go
    forward (§2), a plain look (§6a), a HUD that holds still (§11). The
    default game ships them and is balanced around them. A game that
    changes them is labelled, never blocked.
- **The storyteller is the worked example.** Core's storyteller splits in
  two. The **incident registry** is shared vocabulary that every mod
  registers into. The **pacer** decides when an incident fires, and which.
  Core declares one `[[storyteller]]`, a **singleton kind** like the
  calendar and the sky, and it names core's pacer. A replacement patches
  that one field, so installing one replacement simply works, and two
  replacements are a contested slot (§10). A difficulty mod is just a
  pacer.
  - A claim was considered and rejected: core's own storyteller would
    always claim too, so every replacement would become a question.
- The argument in full, with a mock of starting a game from a set of mods:
  <https://claude.ai/artifact/FoavEVM94kqHnQmiXBkhDn>. Every modding
  detail argued against Factorio, RimWorld, Content Patcher, Luanti,
  Minecraft and Paradox: <https://claude.ai/artifact/NXiPKN7pnSTWFDRCJPgtFg>.

### Costs we accept

Plugin-first isn't free. These are the costs, and what keeps each one small:

- **Every mechanism is a promise.** Engine internals can be refactored
  freely; a mechanism mods rely on is a contract. The API is versioned, and
  we only expose what a second user needs.
- **Generalising takes longer, and can over-reach.** Rule of three: a
  mechanism earns its generality with two or three real users. Until then
  it's a plain engine function (wind shelter, §4c).
- **Performance has a floor.** Data-driven maths is slower than a hand-written
  special case. Per-cell and per-tick work stays in Rust as mechanisms, data
  compiles to flat programs, and every system has a measured budget.
- **Behaviour is spread out.** A value may come from core's data, a plugin's
  push, a patch and a script. So explanation tooling is mandatory: value
  breakdowns, devtools showing which mod did what, conflict reports and
  load-time validation.
- **More combinations to balance.** First-party plugins live in this repo and
  are tested together; the default install is what gets balanced; core alone
  gets a smoke test in CI.
- **Generic means less bespoke polish.** A renderer that reacts to channels
  can't special-case a blizzard. Visuals become data too, so mods get the
  same power we have.
- **Plugins can tangle.** Plugins connect through core's shared names (§5),
  not through each other, wherever they can.

### Load order

The loader discovers `mods/*/mod.toml`, topologically sorts on
`depends`/`load_after`, and breaks ties by id so the order is deterministic.
Then it merges defs, applies patches and runs scripts in that order.

---

## 6a. The grid is a contract, not a look

The map is a flat lattice of integer cells (`crates/rim_sim/src/map.rs`).
What the engine promises about it is short:

- One terrain, one fixture and one item stack per cell, plus a movement cost.
- The map is a stack of such planes, one per level, joined only at portals
  (§6d).
- Passability is 4-connected. A* may step diagonally only when both
  orthogonal neighbours are open, so the two never disagree.
- Regions (reachability), rooms (§4) and fields (§4a) are derived from the
  lattice and rebuilt only when it changes.
- The map edge is a game concept: a room that touches it is outdoors, and
  raiders arrive and leave across it.

Everything the player *sees* on that lattice, and everything that *lives* on
it, is data.

### Tension: should mods be able to replace the grid?

- **For:** the pitch is "write a mod that challenges the base game". Hex
  maps, height, streamed infinite worlds and free-form placement are all
  things people will want to try.
- **Against:** determinism, A*, region and room flood-fills, field stamping
  and lockstep co-op all assume integer cells on a lattice with a fixed
  neighbourhood. Making the topology pluggable would put a trait call on
  every hot path and make every one of those systems generic over a shape
  nobody has asked for yet.
- **Ruling:** the **lattice and the tick are engine**. A mod that wants a
  different topology is a fork, and the design says so out loud. In
  exchange, every other property of the map is open:
  - **Layers, not cells.** The map is a set of named per-cell layers, stored
    as flat arrays. Terrain, fixture and item are engine layers because
    pathing and rooms read them; every other layer (pollution, roads,
    elevation, ownership) is a `[[field]]` a mod declares, and it gets a map
    overlay for free. Nothing about a new layer touches `map.rs`.
  - **Size is a parameter.** The map's dimensions come from world creation
    (a biome or plugin may set them), not a constant. The edge stays a hard
    boundary whatever the size.
  - **One cell, one pawn, one wall.** Sub-cell movement is render
    interpolation and never feeds back (§7). A thing that covers several
    cells occupies each of them in the fixture layer, all pointing at one
    entity, so pathing and rooms never learn about footprints.
  - **Generation is a stage, not a secret.** Terrain bands, spawns and
    wildlife are data today. A script may take over a stage of map
    generation through a hook, with the engine's noise exposed in fixed
    point so a Luau-generated map is as deterministic as a Rust one.

### Tension: pretty out of the box, or plain and overridable?

- **For shipping art:** first impressions. A colony sim of coloured squares
  looks like a prototype.
- **Against:** every sprite core ships is a sprite a mod has to match or
  replace, and the renderer that draws it grows a special case per kind of
  thing. Until API 0.4 `Shape` named content: `tree`, `bed`, `stove`,
  `chair`, and the renderer decided what joins to a wall by matching on
  `wall`, `window` and `door`. A mod that adds a loom picks the least wrong of those, a mod
  that adds a fence couldn't make it join, and a mod that ships a tileset
  couldn't use it at all.
- **Ruling:** **core is plain, and the look is data.**
  - A def declares a `look`: layers of **primitives** from a small fixed
    set (fill, outline, disc, edges) or **sprite keys** into the world
    atlas every mod's sprites pack into ([docs/modding/looks.md](docs/modding/looks.md)). Primitives stay in code because they are mechanisms
    (§10); the named content shapes go.
  - Core ships no sprites. Out of the box the game is coloured rectangles
    and discs, which is the fastest thing to draw and the easiest
    thing to read.
  - The renderer never matches on a def id. It reads the look, the material
    tint, and the cell's neighbours. **Autotiling is a data rule**: a def
    says which neighbours count as joined and which variant each neighbour
    mask picks. Then walls, fences, hedges and marble all join without a
    renderer change.
  - The renderer reacts to the map's `revision` and to field channels. It
    cannot special-case a blizzard, and that is the cost §6 accepted.

### Speed follows from the ruling

- Flat arrays and small integer costs are already the shape of the data;
  keep it structure-of-arrays.
- **Chunks are the one structural addition.** Incremental region updates,
  render caching and field re-stamping all want the same unit: a fixed-size
  chunk with a dirty bit. One chunk grid serves all three rather than each
  system inventing its own.
- Regions are one layer per faction, indexed by an enum. When factions
  become defs, the layers become one per door key, and the enum goes with
  the rest of the content that leaked into the engine.

---

## 6b. Worksites: work shows on the thing

A tree looks the same at the first swing and the last, a plan is the same
blue box for every building, and a wall at 10 hp looks like one at 200.
Chop and take-down progress lived in the pawn's job, so an interrupted
chop started over and nothing could draw it. Every plan was drawn every
frame whether or not anyone was building it.

Three rules, in this order:

- **Idle costs nothing.** A worked thing is drawn live only while a worker
  is on it. Idle, half-done or planned, it sits in its chunk's cached mesh
  at one of 8 stages. The sim touches the map when a site starts or stops
  being worked, and at no other time, so progress never rebuilds a chunk.
- **Progress lives on the thing.** `Work { done, total, .. }` is on the
  tree, rock, plan or wall. It survives the worker leaving, a second
  worker and a save. The renderer reads one number, the stage from 0 to 8:
  the larger of the work done and the hp lost, so a raider's club wears a
  wall the way a hammer does, and a repair runs it back.
- **The look never lies about the grid.** A thing looks solid exactly when
  it blocks. A rising wall is see-through until it stands, and a wall
  being taken down keeps its outline until it is gone.

What is drawn falls into three channels on the cell: the **stage** (baked
when idle), the **strike** (each blow, derived by the client when `done`
crosses a multiple of the style's `every`, so a skilled worker looks
faster and a replay throws the same chips) and the **exit** (a tree falls,
a rock crumbles, a wall settles, once). How a job looks is a
`[[work_style]]` def from a small fixed vocabulary, like the look
primitives: a mod picks and colours effects and never draws per frame.

### Tension: when does a worked thing leave the chunk cache?

- **On every stage change:** 8 rebuilds of a 32×32 chunk per job, for a
  picture that only changes in steps.
- **Always, as plans were:** a big build plan costs frame time while
  nobody builds it.
- **Ruling:** while a worker is on it, and nothing else. Two touches per
  work session; a chop rebuilds its chunk twice, once when the cutter
  arrives and once when the tree is gone.

### Tension: should a half-dismantled wall look lower?

- **For:** it is the obvious picture of "half gone".
- **Against:** it still blocks, and the player would plan a path through a
  wall that stops every pawn.
- **Ruling:** no. Wear on a blocking thing cracks and darkens it and never
  shrinks it, and a load check holds mods to the same rule.

---

## 6c. Houses: drawn as their plan, built as orders

A house today is a ring of flat brown cells with a 1.5-point edge. Its
furniture is a few small rectangles, and its rooms can't be seen at all.
The sim knows far more than that. It knows what each wall is made of,
whether the ring encloses a room, how fast that room loses heat, what lets
daylight in, and what the room holds. A house should show all of it, and
§6a already rules out sprites for doing so. So the look has to carry the
mechanics.

A working prototype of this section is
[docs/engineering/houses-prototype.html](docs/engineering/houses-prototype.html):
one page of canvas code with no build step. Open it in a browser, paint
walls, and watch the rooms change. Its join, pattern and roof code is the
reference for the Rust renderer.

### What the other games teach

- **RimWorld:** things made of stuff, rooms found from walls, a roof held
  up for 6 cells by a wall or a column, and rooms that take a role and a
  score from what is in them.
- **Minecraft:** a block is a shape times a material, and the material is
  what you see. Working a material changes both its look and its stats
  (cobblestone into stone bricks). The material decides which tool it
  needs. Fences and panes join their neighbours by themselves: a post, and
  an arm toward each neighbour. Structures are data.
- **Prison Architect:** a top-down plan anyone can read, rooms defined by
  what they must contain, and a room you draw with one drag.
- **Terraria:** a house is a checklist the game can explain: walls, a
  door, a light, a table, a chair.
- **Townscaper** and the dual grid: a join is decided at a cell's corners,
  not at the cell.
- **Architects' drawings:** solid walls (poché), a few line weights, a
  hatch for each material, and a door drawn as its leaf and its swing.

### Tension: a picture of a house, or a plan of one?

- **For a picture (sprites, three-quarter view):** it's warmer, and it is
  what the genre looks like.
- **Against:** every sprite is a binary that a mod must match or replace
  (§6a). A three-quarter view hides the cell behind a wall. And a picture
  shows what a house is like, where a plan shows how it works.
- **Ruling:** **a house is drawn as its plan.** The view stays top-down and
  orthographic. Walls are solid masses, openings are gaps with symbols,
  furniture is outlined plan symbols, and rooms are labelled. Five rules
  hold it together:
  - **Four line weights.** Wall contours are the heaviest, openings are
    medium (a door leaf, a pane), furniture is thin, and material patterns
    and floor seams are hairlines. Hierarchy comes from weight, not colour.
  - **Material is a pattern, not a picture.** Each material has a tint and
    a pattern from a fixed vocabulary (`weave`, `stipple`, `logs`, `rubble`,
    `courses`, `bond`, `crag`). The pattern is laid out in world space and
    along the wall's run, so it flows from cell to cell. It fades out below
    a zoom level, which keeps the worst case (the whole map zoomed out)
    cheap.
  - **One light.** Every mass casts the same short shadow down and to the
    right, and its top and left edges catch a highlight. That gives height
    without perspective. (§6e: the shadow is a contact shadow drawn with the
    light, and it yields to the sun's.)
  - **The look never lies (§6b), extended to rooms.** A window facing an
    enclosed room throws a fan of daylight into it. A fire fills its room
    with a warm wash that stops at the walls. The one gap that keeps a ring
    of walls from being a room is marked. Floor beyond the roof's reach is
    hatched as sky.
  - **Zoom tells three stories.** Up close you see the plan, with patterns
    and labels. Further out you see masses and symbols. Furthest out you
    see roofs, and the colony reads as a village. Hovering a house lifts
    its roof.

### Tension: how do walls join?

- **Per-cell edges (today):** an `edges` layer leaves out the sides facing
  a joined neighbour. It's cheap, and it's why a run already reads as one
  wall. But corners are square notches, every cell is a flat box, and a
  door is a rectangle that happens to sit in a wall.
- **Hand-drawn tile sets (the 47-tile blob):** they look beautiful, but
  they are 47 pictures per material, which is the opposite of §6a.
- **Ruling:** **joins are a topology, drawn in quarters.** A joined piece
  reads its 8 neighbours. Each quarter of the cell looks at its two sides
  and the corner between them, which picks one of five shapes: outer
  corner, either edge, inner corner, or solid. That is the blob tile set
  worked out from the mask with primitives, instead of drawn by hand.
  - `look.join = { group = "wall", style = "mass", round = 0.22 }`. A
    `mass` fills the whole cell, which is honest because a wall blocks the
    whole cell. It is rounded where a run ends and square where it meets
    another. A material change along a run shows as a hairline seam, never
    an outline.
  - `style = "pipe"` is Minecraft's fence: a post in the middle and a pair
    of rails toward each joined neighbour. `connects = ["wall"]` lets a
    fence meet a wall without the wall bulging toward it.
  - **Run orientation.** A layer can be written along the run
    (`orient = "run"`) and the renderer turns it to fit the neighbours. So
    a door or window placed in a north–south wall needs no rotation.
    `into = "room"` mirrors a layer toward the enclosed side, which is how
    a door swings into the room and a window's light falls inward.
  - Rock joins as `rock` the same way, so a cliff is a mass too.
- **New primitives, still a fixed set:** `mass`, `pipe`, `pattern` (the
  material's) and `arc` (a door's swing). `edges` stays for mods that use
  it.
- **Cost:** a mask is 8 reads and is built with the chunk mesh. Neighbours
  already dirty a chunk's border (§6b, `map.rs`). Patterns cost vertices
  only at zooms where the chunk count is small.

### Tension: facing, or looks that work it out?

- **For explicit rotation everywhere:** it's simple, and it's what every
  builder does.
- **Against:** a door rotated by hand can face the wrong way, and a chair
  turned away from its table is a mistake the game could have avoided.
- **Ruling:** both, each where it fits. Things get a **facing** (four
  directions). Their footprint, spots and look turn with it, so a 1×2 bed
  or a workbench with a place to stand in front can face any way. Placing
  one takes a facing, `R` in the client. **Joined pieces derive** their
  facing from the run and the room, and a chair turns to face the table
  it's beside. A look can say `face = "beside:table"`.

### Tension: when is a room indoors?

§4 ruled that a room is indoors when it is enclosed and at most 400 cells.
The cap stands in for a roof.

- **For the cap:** it's one number, and a new player never meets it.
- **Against:** it can't be seen, and it can't be explained ("why is my
  hall outdoors?" "it's 401 cells"). It treats a 20×20 hall the same as a
  2×200 corridor. It gives materials no say, and pillars no job.
- **Ruling (revises §4):** **the roof follows from the walls.** Every
  support (wall, door, window, pillar, and rock, since overhead rock makes a
  cave) holds the roof up for `span` cells in each direction. The span is a
  material factor: 2 for wattle, 3 for cob and dry stone, 4 for logs and
  brick, 5 for ashlar and granite. A room is indoors when it is enclosed
  and every cell in it is within reach of a support. A 3×3 hut and the
  bot's 5×5 hut don't change. A great hall needs pillars, and a valley
  ringed by cliffs still isn't a house. The engine works the span out at
  room rebuild, which already runs only when walls change: O(room cells).
- **The same field draws the roof.** A house is the set of indoor rooms
  that share walls. Its roof covers their cells and walls. Each cell's
  height is its Chebyshev distance to the eaves, which gives a hipped roof
  on any room shape. The material comes from the walls (thatch on wattle
  and cob, shingle on logs, turf on dry stone, slate on ashlar, tile on
  brick). A hearth gets a chimney. This is client-only and derived, and is
  rebuilt only when rooms are.
- **Roofs still cost nothing to build.** An implicit roof is free. A built
  floor on the level above is an explicit one (below).

### Storeys: one span rule for roofs and floors

Depth (§6d) makes the map a stack of levels joined at stairs, and building
up puts a floor over air. Holding up a floor and holding up a roof are the
same question, so they share one field, worked out per level (per `Map`):

```
covered(z, c)  = some support on z within its span of c
roofed(z, c)   = covered(z, c)  or  the cell at (z+1, c) is solid or has a floor
floor at (z+1, c) may be built  when  covered(z, c)  or  a support stands at (z, c)
```

- **Underground is roofed by rock.** A cell below a solid cell is roofed
  whatever its distance from a wall. A pit dug to the surface isn't.
- **The implicit roof marks where a storey can go.** Every cell a room's
  walls roof is a cell that can carry the next floor. A floor built there
  becomes that room's explicit roof, and the storey above has rooms of its
  own, found on its own level.
- **A house spans levels.** It is the set of indoor rooms that share walls
  on a level, or that stand directly above one another. Its roof is drawn
  over its topmost storey, and hovering it lifts the whole house.
- **Seeing storeys.** One level is drawn at a time (§6d): the levels above
  are cut away, so a plan is always a true floor plan. Walls on the level
  below show through air as a dim ghost of their contour, which is how a
  gallery or a stairwell reads. Stairs are a plan symbol: treads, a break
  line, and an arrow marked UP or DN.
- **Rooms and roles stay per level.** Span, roles and gaps are pure
  functions of one `Map`, so the level stack wraps them unchanged.

### Tension: do rooms know what they are for?

- **Scripts only:** a mood plugin could classify rooms in Luau. But eras
  (Camp: "a shelter, a bed and a fire"), mood ("slept in a barracks") and
  the storyteller would each classify them again, and could disagree.
- **Ruling:** **room roles are data.** A `[[room_role]]` has `needs` (tag
  counts) and optional limits (size, indoors). The most specific role a
  room meets names it (the most needed tags), then a role's `priority`; a
  tie is a contested slot (§10). Load order never decides what a room is. The engine counts the tags inside each room
  at room rebuild and when furniture changes, which is the same bitset
  match as tools (§4e). Core declares `bedroom`, `dormitory`, `home` and
  `hall`. Crafting adds `workshop`. Scripts read `rim.room_at(x, y).role`,
  and the client labels the room on the plan with its role, size and
  whether it's heated. Room scores (beauty from the boundary's material
  factor, space, light) are stats on the §10 pipeline, for mood to weigh.

### Tension: two ways to make things, or one?

Building has Blueprint, Deliver and Construct. Crafting has orders. They
are the same job (§4e).

- **Ruling:** **a build is an order** (e7c4a3f6). A blueprint becomes an
  order whose completion is the engine's own: it becomes the building. This
  gives building what orders already have: inputs by tag, `requires` for
  tools, a work type and the "why can't this run" reasons. Two things
  follow:
  - **A material names its tool.** `stuff.requires = ["pounding"]` means
    anything built of ashlar waits on a maul in hand, the way a Minecraft
    block names its pickaxe. Building then climbs the same ladder as
    gathering: wattle and dry stone by hand, logs with an axe, ashlar with
    a maul, brick from a kiln.
  - **Replace in place.** Planning cob over a wattle wall, or a door over a
    wall, keeps the old piece standing until the new material is
    delivered, then swaps it in one work session. The room stays enclosed
    through a winter upgrade.
- **Deferred: two materials in one piece.** A timber frame with a cob
  infill would be beautiful (half-timbering), and a look could colour
  layers by slot. But it's the first user of a mechanism with no second
  one (§6, rule of three). One material per piece, until a mod asks for it.

### Plans as data

A house plan is text: an ASCII grid and a legend, placed as blueprints
with one command and turned by a facing.

```toml
[[plan]]
id = "cob_house"
label = "cob house with a hearth"
grid = """
#######
#b.#.h#
#..+..#
#..#t.#
###+###
"""
legend = { "#" = "core:wall", "+" = "core:door", b = "core:bed", h = "core:stove", t = "core:table" }
```

Mods ship plans, players save their own (client files, text again), and
the balance bots build from them instead of hand-coding huts.

### What this costs, and what it replaces

- **Frame:** masks and patterns are built with the chunk mesh, and the roof
  field is rebuilt only with rooms. Nothing new runs every frame. The
  render bench (§8) gates it, and pattern LOD is the lever if it goes over.
- **Tick:** role counting and span coverage run at room rebuild. No new
  per-tick work.
- **API:** joins, primitives, material looks, facing, roles and plans are
  additive. Replacing `MAX_ROOM_CELLS` with a span changes what counts as
  indoors, so it is breaking and needs an api bump.
- **Content:** core's looks move to the new primitives. "Every core def has
  a sprite" (f4e97005) becomes "every core def has a finished plan look".

---

## 6d. Depth: stacked planes

Mining and building gain a z axis: dig down for clay, metal and shelter,
build up for second storeys. Depth has to feel big and cost nothing where
nobody has dug.

### Tension: true 3D or stacked planes?

- **For 3D:** ramps, slopes, free vertical movement, water under pressure.
  It is Dwarf Fortress's model and the richest one.
- **Against:** every flood fill, A* and field stamp goes from 4 or 8
  neighbours to 6 or 26, and regions and rooms span levels. §6a makes the
  lattice engine because every hot path depends on its shape; 3D changes
  that shape in all of them at once.
- **Ruling:** stacked planes. Levels meet only at **portals** (stairs,
  ladders, anything whose def says `portal`), which cover two cells one
  above the other the way a multi-cell thing covers its footprint. There
  are tens of portals, not thousands, so every 3D question becomes a 2D one
  plus a small graph:
  - Positions carry `z`; 0 is the surface. A position without `z` is on the
    surface, and `z` 0 is never written, so saves, commands and scripts
    from before levels read the same.
  - **One map, planes in a row.** Every per-cell array holds each level's
    plane in turn: the surface first, then down, then up. A cell's index
    includes its level, `offset` stays on it, and code that only knows the
    surface indexes the first plane unchanged. One allocation, no lookup of
    a level on a hot path.
  - Regions rebuild per level: a wall on −2 renumbers −2 and nothing else,
    and a level's ids start at its plane's number times `1 << 20`, so they
    never collide. Rooms are one rebuild over every level, keeping the
    contiguous ids the fields' room values are indexed by; a level of solid
    rock is passed over at the cost of a scan. Chunks are per level.
  - Reachability is a union-find over `(z, region)` joined at portals,
    rebuilt in O(portals) when a level's regions or a portal change. Asking
    stays O(1).
  - A path is planned leg by leg: a route over the portal graph, then A* on
    one plane to the next portal with today's scratch buffers. The next leg
    is planned on arrival.
  - Every level is generated with the map, from `[[stratum]]` defs. An
    earlier draft generated a level only when something first dug into
    it; with the planes in one allocation that saves no memory, and
    generating everything up front keeps determinism and saves plain. A
    level of rock is one zstd-compressed array in the save. Measured at
    192 × 192, core only, best of 8: a new game takes 5.9 ms for the
    surface and 15.3 ms with three strata, about 3 ms a level.
  - A mod may make a level itself: `rim.on_generate_level(z, fn)` runs
    after the stratum has filled it, with `rim.set_terrain`, `terrain_at`
    and the engine's seeded `noise`, so a Luau level is as deterministic as
    a Rust one. Solid terrain with no `thing` is bedrock, and every stratum
    rings its level with some: only the surface has a map edge.

### Tension: how deep?

- **For many levels:** Dwarf Fortress has a hundred-odd, and depth is the
  point.
- **Against:** a fort there lives in ten or so, and what players remember
  is the aquifer, the magma and the cavern, not the count. Each level here
  is a whole 192 × 192 plane of rock that nobody mines out. A level that
  asks no new question is a screen to scroll past.
- **Ruling:** the range is a world-creation parameter, like size. Core
  sets three down, and two up once building up lands. Each level below
  earns its place with a new material, tool tag and danger: −1 soil and
  clay (`digging`: cellars, wells, pits, the water table), −2 limestone
  (`pounding`: copper, tin, aquifers), −3 granite (`mining`, a new core
  tag: iron, coal). A caverns
  plugin opens −4 and what lives there. The default map shrinks to
  **192 × 192**, six chunks a side, because dug levels add the area of
  whole maps; 250 × 250 stays the §8 stress case.

### Rock is terrain

Granite was a thing spawned on every `rock_floor` cell: one entity per rock
cell. Four levels of that would be 147,456 entities that never act.

- **Ruling:** a terrain may be `solid = { thing, leaves }`: it fills its
  cell, blocks movement and bounds rooms, which already treat impassable
  terrain as boundary. `thing` is what stands in the cell once someone
  works it. Marking, planning over or ordering work on a rock cell stands
  that thing up, and mining it leaves the `leaves` terrain. Unmarked with
  no work done, it goes back to being terrain.
  - Everything about working rock (harvests and their tool tags, work,
    wear, the look, wind, room boundaries) comes from the thing, so the
    whole harvest pipeline is reused, and a patch to `thing/core:granite`
    still reaches every rock cell.
  - Measured on seeds 1–3 at 200 × 200 with the default mods: 3,630,
    6,524 and 7,206 entities after generation became 1,383, 2,973 and
    2,060.
- A cell's terrain is **solid**, **floor** or **air**. A floor belongs to
  the cell that stands on it, so mining the rock under a room leaves the
  room's floor. **Dig down** turns an open cell's floor to air and mines
  the cell below: a pit one level deep, or a stair pair if you ask for
  stairs. It always goes exactly one level. Air is not walkable; items and
  water fall through it.

### Tension: can a pit stop a raid?

- **For:** digging costs real labour, and for a stone-age colony that can't
  afford stone walls a trench *is* the wall.
- **Against:** a closed ring of air makes raids pointless. That is §1's
  failure again: players learn to exploit the system instead of building.
- **Ruling:** a pit is impassable at its level and nothing climbs out, so
  it can't be broken, only **bridged**. A raider who can't reach anything
  lays planks over air a cell at a time, as raiders break doors today, and
  defenders can knock a bridge down. The colony's own way out is a
  drawbridge: an owned floor over air, which is the owned-door rule laid
  flat. Trenches count as defence in the raid budget. Climbing is a mod: a
  movement class whose region layer treats a one-level drop as passable.

What was built (ba8253df):

- Footing is one flag per cell: walkable terrain, a floor, or a fixture
  that spans air. Passability reads it and nothing else, so a floor over a
  pit is ground to the pathfinder and the regions with no special case.
- `build.spans` marks what stands over air: `core:bridge` (a floor) and
  `core:drawbridge` (a door). They are planned over open pits only, and
  anyone's bridge can be deconstructed.
- A raider shut out with no door to break looks at the open air beside its
  side and bridges the cell nearest its target, with the quickest spanning
  floor any mod has; each cell joins its side, so the next is one further
  across. With no spanning floor loaded, a trench holds.
- What stands on a bridge that goes falls to the level below, hurt a tenth
  of its health a level; an item dropped over a pit lands below.
- The drawbridge is the owned-door rule on a spanning door rather than a
  floor with an owner, since doors already carry one.
- Trenches don't count in the raid budget yet, because walls don't either:
  `colony_strength` is melee alone. Defences in the budget is its own item.

### Tension: water per cell or per basin?

"Dig into water and it floods" needs moving water. Two prototypes, 192 × 192
× 5, a mine of 13,276 dug cells breached into a river (release, Apple
Silicon, one run each; the reference machine is slower):

| Model | Fill the mine | Mean per tick | At rest |
|---|---|---|---|
| Per cell: fall, then halve differences | not full (64%) after 200,000 ticks | 0.0089 ms | 0.0056 ms |
| Per basin: volume per connected open area | 332 ticks (24 game minutes) | 0.0014 ms | 0.000009 ms |

Per cell is cheap per step (13.6 ns a cell) but diffusion through a tunnel
takes time in the square of its length. On a whole flooded level it costs
0.40 ms a tick for 18,532 ticks and settles 62% full, in a slope, because a
one-unit difference never moves. Dwarf Fortress fixes that with pressure,
which is where its cost and strangeness come from.

- **Ruling:** basins. A basin is a connected open area on one level,
  derived like regions and rebuilt with them; only its volume is saved.
  - Surface water terrain is the **water table**: static, infinite, never
    simulated. Open space dug next to it floods. Aquifer rock seeps at its
    def's rate once a face is exposed.
  - Water falls first, through air and stairwells, into the basin below
    until that one is full. It never climbs: no pressure, and a U-bend
    doesn't level out.
  - A wet front grows one ring a tick from where the water entered. The
    depth is uniform across wet cells.
  - Wading, swimming and drowning depths are data on a `[[fluid]]` def.
    Passability changes only when a basin crosses one, and then that
    level's regions rebuild at most once every 60 ticks.
  - Doors pass water unless their def `holds_water`. Core has no drains;
    pumps are crafting content. Runoff and puddles from rain stay a
    stock-field plugin on the surface (§5).

### Seeing it

One level is drawn at a time: levels above are cut away, and the level
below shows through air, dimmed. Rock nobody has stood beside is drawn
plain, so a vein is found by looking. `[` goes down and `]` goes up. Chunk
meshes are keyed by `(z, chunk)`. The depth ruler that shows who is on
which level is a UI mod.

### Underground

Rock is a roof. Solid terrain is a roof support (§6c), and a cell is
roofed when a support's span reaches it or the cell above is solid or has
a floor, so every underground room is roofed with no special case.
Temperature below comes from terms: −1 follows the year's mean with a
damped swing, and deeper is steady and warmer. Light is zero until
something emits it.

### Building up comes second

Digging removes material from a solid world; building up adds floors to an
empty one, which needs support, collapse and coverage. It reuses §6c's
roof span rather than adding a second one, computed per level:

    roofed(z, c) = span_covered(z, c) || solid_or_floor(z + 1, c)

A built floor at `(z + 1, c)` is allowed only where `span_covered(z, c)`
holds or a support stands directly below it. So the implicit roof over a
room is exactly where a second storey can go, and the floor laid there
becomes its explicit roof. A floor that loses its span falls.

### Costs we accept

- Positions gaining `z` touches nearly every file, the commands, the save
  and the Luau surface: one large mechanical change.
- Rock becoming terrain changes what map generation places: a mod that
  spawned things on `rock_floor`, or looks for granite entities, finds
  none until a cell is worked.
- Basins simplify: no currents, no pressure, and water never climbs stairs.
- One cell of height per level: no ramps or slopes, and hilltops are flat.
- Drawing the level below is not yet measured; the view records it here.
  Generating a level is about 3 ms at 192 × 192 (the strata above).
  Crossing levels, measured with portals (acd85584): joining reach over 61
  portals on four levels costs 0.0017 ms. With every colonist mining two
  dug levels below the colony (`bench --levels`, 250 × 250), a tick is
  0.41–0.52 ms on average and p99 5.6–11 ms, with searches still about 12
  nodes. The largest part is rooms, about 0.2 ms, since every dug cell
  rebuilds rooms over all four levels until rooms update incrementally.
- Digging down is a build: stairs, ladders and pits are things whose
  `build.dig` takes out the cell below first, with the work and the tool
  its rock asks. The stairs stay as the portal; a pit's cell becomes `air`
  terrain. Wind is the surface's alone, and cover is never worked out below
  it: rock is the roof there.

---

## 6e. Light: at the rate it changes

Today the renderer multiplies the world by one lightmap: firelight stamps,
an indoors bit, and the sky as a uniform (§4c, "Seeing the weather"). It
costs 3 µs, and it has no shadows, no flicker, and no difference between a
torch in a hut and a torch in a field. A concept with a live WebGL demo of
everything below is at
<https://claude.ai/artifact/SC5Coj3UKKjxVCSxLKEq1t>.

### Tension: whose light is it?

- **For moving light into the sim:** gameplay would see the same shadows the
  player does.
- **Against:** shadows depend on sun angle, window glass and flame size,
  none of which the sim should pay for or hash.
- **Ruling:** **the sim keeps its scalar `light` field; everything here is
  the renderer's.** Gameplay (plants, mood, sight in 713009ac) reads the
  field. The picture adds direction, colour, softness and flicker. The two
  agree wherever walls block both and differ at windows, doors and tree
  shade, so any overlay that informs a decision shows the field, never the
  picture.

### Tension: what does a frame pay for?

Most light in a colony barely changes. Walls stand for days, torches stay
where they were built, and the sun moves a fraction of a degree a second.

- **Ruling:** **each kind of light is computed at the rate it changes**, into
  a light buffer at 1, 2 or 4 texels per cell, and a frame is a sum of cached
  textures and a few uniforms.
  - **Occluders**, one texel per cell (height, roofed, opening, sky opacity,
    light opacity): rebuilt when walls, terrain or rooms change.
  - **Sky shadows:** each light texel steps toward a sky body through the
    height map, rising `tan(elevation)` per cell, and stops at the first thing
    taller than the ray. Rebuilt when a body moves past a threshold (0.25° by
    default), otherwise free. The penumbra widens with distance from the
    occluder, and cloud widens it further.
  - **Static lights** (anything that emits `light` and doesn't move): soft
    shadows baked once, with 8 rays, into one of four **flicker channels**.
    Flicker is then four colours a frame, for one torch or a thousand.
    Rebaked only for lights within reach of a change, the rule the sim
    already uses to re-stamp emitters.
  - **Dynamic lights** (carried, burning, moving): the same march every frame,
    up to a cap per preset; past it they glow without shadows.
  - **Compose** at light resolution, then **multiply** the world, as today.
- **Falloff** is `I · (1 − (d/r)⁴)² / (1 + 0.08·d²)`: close to inverse-square
  near the flame and exactly zero at the radius, so a light's quad is tight.
  **Flicker** is smooth noise with rare gusts, redder when dimmer, never a
  fresh random value per frame.
- **Exposure** follows the sky (0.8 at noon, up to 2.6 on a moonless night,
  eased), which replaces `max(sky, fire)`: a torch reads strong at night and
  weak at noon because the eye adapts, not because of a special case.

### Indoors

- A cell under a roof sees the sky only along a ray that leaves through a
  window between its sill and lintel. That one rule gives §6c's daylight fan:
  a strip under the window at noon, a bar across the floor at dusk, nothing
  through a north wall. "Under a roof" is the per-cell `roofed(z, c)` of §6c,
  so a hall beyond its span gets sky where the plan hatches it.
- A closed room returns its lights' flux from its walls:
  `fill = 0.3 · Σ(I·r²) / area`, flat over the room, in each light's channel.
  That is §6c's "warm wash that stops at the walls". Outdoors there is nothing
  to bounce off, so a campfire falls off into the dark.
- A room's diffuse sky share is `[[sky]]`'s `indoor_share` (what gets in
  through walls and a door; core 0.2) plus the light `pass` of its windows,
  which room rebuild already sums (§4, 0211). A windowless hut is dim by
  day; every window shows.

### Tension: the plan's one light, or the sun?

§6c gives every mass the same short shadow down and to the right (ae5c3807),
baked into the chunk mesh. Sky shadows point wherever the sun is.

- **For the fixed shadow:** it is a drawing convention; it reads the same at
  every hour and on every machine, and it costs nothing per frame.
- **For the sun's:** it is the atmosphere this section exists for, and two
  shadows on one wall at dusk contradict each other.
- **Ruling (0779def9):** the fixed shadow becomes a short **contact
  shadow**, down and to the right, that fades as direct sky light reaches
  the cell. By day the sun's shadow does the work; at night, indoors,
  underground and on `low` the plan convention remains, so a wall never
  has two shadows and never has none.
  - It is drawn by the lighting compose pass, not baked into the chunk
    mesh, because the fade needs the per-texel sky visibility that only
    compose has. It costs one occluder read. The top-and-left highlight
    stays in the mesh (ae5c3807).
  - Roofs (df049dac) shade their facets by the real sun direction instead
    of a fixed one.

### Depth

Light is per level, like everything else in §6d, and it has to be
**continuous across levels**: no edge where one level's light stops and the
next begins, and no pop when the view changes level.

- Buffers are keyed by `z`. The viewed level and its neighbours above and
  below stay cached; the rest are evicted with their chunk meshes.
- **Light crosses openings.** Every air cell, stairwell and ladder is an
  opening. Each level's compose adds the light of the level above through
  its openings, and the level below's through its own air cells, from a
  blurred, half-resolution copy of that level's composed light, attenuated
  per level (0.5 by default). So a torch at the top of a stairwell lights
  the steps below and fades out around them, and a fire in a pit glows on
  its rim. The cost is one read of a small texture per texel, and it is
  skipped on a level with no openings.
- **Sky down a shaft falls off with depth.** A column open to the sky gets
  direct sun only while the sun is inside the shaft's cone, which the same
  height-map march gives when the levels above count as solid height. The
  diffuse sky share falls with the solid angle of open sky seen from the
  bottom (`width / (width + 2·depth)`), so light fades smoothly down a
  shaft instead of stopping at the surface.
- **The level below, through air, is lit by its own light** and dimmed by a
  depth tint per level, the same curve at every step down, so looking down
  a shaft three levels deep reads as one gradient.
- **Exposure is one continuous value.** It blends between sky-driven and
  firelight-driven by how much sky reaches the view, and eases over about a
  second, including when the view changes level. Changing level crossfades
  the two cached buffers for 150 ms.

### Presets

`low`, `medium` (the default), `high` and `ultra`, from `[lighting]` in the
player's settings file:

| | low | medium | high | ultra |
|---|---|---|---|---|
| Light texels per cell, at most | 1 | 2 | 2 | 4 |
| Sun steps (0.4 cells each) | 16 | 28 | 40 | 56 |
| Soft sun shadows | no | yes | yes | yes |
| Sun worked out again after | 1° | 0.25° | 0.1° | 0.02° |

Any setting overrides its preset (`texels_per_cell`, `sun_steps`,
`soft_shadows`, `sun_rebuild_degrees`). There is no `auto` yet (24bad102):
the frame's time mixes the sim and the UI with the light, and a GPU timer
works only off Apple, so it would have turned the lights down for the wrong
reason. Light is upsampled bilinearly in every preset; bicubic was tried and
changed nothing visible at 2 texels a cell. Static lights bake at 8 rays in every preset, because the bake runs only on
edits. A light texel never gets smaller than 4 screen pixels: at the minimum
zoom (4 points a cell on a 1x screen) the buffer drops to 1 texel per cell,
so zooming out doesn't raise the cost, and it grows back only with a fifth
to spare, so it doesn't flicker between sizes at the edge.

### Cost and constraints

- macroquad already has what this needs: a `render_target` per pass, additive
  and multiply `BlendState`s, and GLSL 100 with constant loop bounds and a
  uniform `break`. Targets are RGBA8 with square-root encoding and dither.
- Modelled at `medium` for a 1080p view of 120 × 68 cells: about 0.4 M
  texture reads a frame beyond today's multiply, and 1.9 M more on a frame
  where the sky rebuilds. Unmeasured; the first ticket puts every pass in
  `rim --bench-render` with GPU time, and each later ticket records its
  number here.
- **Baseline** (`rim --bench-render`, 250 × 250, before any of this; Apple
  silicon under load, so read the CPU numbers as a range): `Sky::light` is
  two passes. The lightmap rebuild runs only when emitters or rooms change
  and costs 0.45–1.7 ms of CPU. The multiply is one draw call and 0.005 ms
  of CPU. The bench's per-pass GPU column comes from timer queries, only
  where they can time one pass: not on a tile-based GPU (Apple silicon),
  where a query times the whole tile pass. CI's runner draws with llvmpipe,
  so its GPU column is the CPU rasterising, useful only relative to itself;
  GPU budgets are read on a real immediate-mode GPU.
- **Sun shadows** (8f4f1de8, same machine): the sun pass costs 0.15 ms of
  CPU when it runs, on a 500 × 500 target, and nothing on the frames it
  doesn't; with the sun moving 0.25° between runs, that is none of a
  steady frame. The multiply is one draw call and 0.008 ms.
- **Costs we accept:** height-map shadows are 2.5D (a canopy shades like a
  column); four flicker channels share colours; a static torch's shadow never
  sways; 8-bit buffers need dither in the dark.

---

## 6f. Overlays: the grid, selection and highlights

Everything the client draws over the world answers three questions: what
is under the pointer, what have I picked, and what will this order do.
Today each answer has its own look. Selection is a yellow box. A drag fills
its whole rectangle in the tool's colour. A tool's cursor is a 2-point
square. Stockpiles and blueprints are both sky blue. And there is no grid,
although every order is a promise about cells. The concept, with a live map
and every state drawn, is at
<https://claude.ai/artifact/N2buE4YpoduQdxpE1zZTHv>. Its working name is
Chalkline.

### Tension: always show the grid, or never?

- **For always:** every order is a promise about cells (§6a). A wall run of
  eighteen cells is easier to lay when you can count them.
- **Against:** a lattice laid over the whole map turns a village into graph
  paper and fights the plan's own line weights and patterns (§6c). A player
  watching the colony has no use for it.
- **Ruling:** **the grid shows while a tool is in hand.** It has four
  levels:
  - **Rest:** no grid, with the select tool and nothing held.
  - **Lens:** a tool is armed. Corner ticks appear around the pointer and
    fade out over 5.5 cells.
  - **Plan:** during a drag, lines run across the view, strongest near the
    pointer.
  - **Measure:** held with G. Every fifth line is heavier, counted from
    the map's origin. Rulers count cells, and the pointer's row and column
    are lit.

  A grid line is a groove in the ground, not a line over it. It is a dark
  seam, with a faint lit edge on its lower-right side from the same
  top-left light as the walls (§6c). It is drawn after the terrain and
  before the things, so a tree or a wall hides it. Below 10 points a cell
  there are no lines, and below 20 they are at 70%. Measure's heavier lines
  stay at every zoom.

### One colour, one job

- **Chalk** (a warm off-white) means attention. It marks hover, selection,
  the drag box, the grid's lit edge and the ring an order leaves. It never
  changes colour, whether the thing selected is a raider or a tree.
- **Hue** means kind. Designation colours stay in the defs. The theme's
  `accent` blue marks the player's plans and people. Zones move to violet,
  so that a stockpile never reads as a plan. `bad` (amber) means allowed but
  costly, and `threat` (red) means it can't be done, or it's hostile.
- **Form** carries state. An edge is hover, brackets are selection, a
  wash is extent, a cross is blocked, a triangle is caution. A player who
  can't tell red from green reads every state from its shape.
- **Every overlay stroke has a keyline**, a dark line 2 points wider drawn
  under it. This is what keeps chalk readable on sand and blue readable on
  water. The black disc under a designation dot is the same idea.

The colours and sizes are theme tokens in core's `ui/theme.toml`, and the
world renderer reads them from the theme. A mod restyles the overlays the
same way it restyles a panel. Durations and zoom bands are client
constants, since they are measured in cells and time.

### Out, edge, in

When states stack on one thing, each sits at its own distance from the
footprint. Selection is 3 points outside it: brackets on a thing, a ring on
a pawn. Hover is on the footprint's edge. Marks are inside it, each at a
fixed corner: a designation top-right, urgent top-left, unreachable
bottom-left. Nothing covers the thing itself, and no state hides another.
Which jobs are unreachable is the sim's answer, read-only:
`ai::unreachable_jobs` asks the map's regions, one colonist per region,
for each job in view. The work choice can't say, since it stops at the
nearest job it can reach.

### Tension: who decides what a preview shows?

- **The client:** it already knows the tool, the drag and the map.
- **Against:** `command::apply` decides which trees a chop drag marks and
  which cells get a wall. A second copy of those rules in the client will
  drift from them. A preview that doesn't match its order is worse than
  having no preview.
- **Ruling:** **the sim answers, read-only, and `apply` uses the same
  answer.** A designate preview names the targets an order would mark. A
  build preview gives each cell an outcome: open, clears a natural thing
  first, or blocked by something named. Both are mechanisms and name no
  content. The client draws only what would change. New targets get a ring
  in the designation's hue, and marks already there stay as they are. A
  cell that has to be cleared first gets an amber triangle, and a blocked
  cell gets a red cross and a reason.

### Motion reports a change

Hover fades in over 60 ms and out over 140 ms. Brackets close in from 7
points to 3 over 120 ms. The grid fades in over 160 ms. It fades out over
280 ms, but only after a 400 ms hold, so swapping tools doesn't make it
blink. An order leaves one chalk ring for 240 ms. A refused placement
shakes twice. Urgent is the only mark that keeps moving: a ring breathes
out every 1.8 s. The reduce-motion setting makes all of this instant and
stills the urgent ring.

### Cost

The grid costs one quad per line in view and nothing below 10 points a
cell, so the whole-map views in `rim --bench-render` pay nothing. The lens
and previews touch only the cells near the pointer or inside the drag.
Everything else is a handful of strokes per selected or hovered thing.

---

## 6g. Rock face: what mining looks like

Rock is terrain (§6d) and has a look, but not a picture. A hill is a grey
plate with a per-cell tint, so it reads as tiles. It has no height, so the
one light of §6c never reaches it. `crag` puts two random squiggles in every
cell. The mine designation is `#b0a89e`, grey on grey. Strata adds six rock
kinds that differ only in tint, and nothing shows a vein, a tool gate or the
face a miner can reach. The concept, with a live mining scene and every state
drawn, is at <https://claude.ai/artifact/Je86GRjsT2dTPN6ZS1bqMt>. Its working
name is Rock face.

### Tension: a mass like a wall, or a picture of rock?

- **For the wall's mass:** it is already built, joins by rule and carries the
  lit edge. A rounded rectangle per cell is honest about the grid.
- **Against:** a run of rounded squares reads as masonry. Nobody sees a hill.
- **Ruling:** **rock is a mass with a broken outline.** The join is the same
  and the fill still covers the cell, so the look never lies about what
  blocks (§6b). Where a cell faces open ground its edge is broken into a few
  facets and its outer corners are bevelled, by hashes of the world edge and
  corner, so neighbours agree and a cell always looks the same. Where rock
  joins rock the edge is straight. Scree (a few broken stones in the rock's
  colour) lies at the foot of each face. Per-cell tint goes; tone drifts with
  low-frequency noise. `look.join = { group = "rock", style = "rough" }`.

### Height

One number drives the plan and the light: a solid cell's distance to open
ground, capped at 4, flood-filled per chunk with a border of 4 when terrain
changes.

- The plan multiplies a smooth relief from it (100, 84, 73, 65%) and draws a
  hairline contour where it steps, as a topographic map does.
- The occluder texture (§6e) takes it as height: `1 + 0.4 · min(d − 1, 3)`
  storeys. A hill casts a longer shadow than a wall, and a deep adit is dark
  at noon. Walls stay at 1.

### Tension: whose pattern does rock use?

- **The walls':** `rubble` and `courses` exist and are cheap.
- **Against:** bedded limestone drawn as courses is a wall. Material patterns
  follow a run (§6c); rock has no run.
- **Ruling:** **built things follow their run; rock follows its bed.** Each
  rock kind has a pattern after the lithology symbols on geological maps:
  `igneous` (crosses, ticks and joint sets), `bedded` (beds with staggered
  joints), `nodular` (faint beds with flint nodules strung along them),
  `laminated` (short dashes), `pebbly` (pebbles and grit), `hatch` (bedrock)
  and `seep` (bedded, with a damp band on an exposed face). They are laid in
  world space along one dip per stratum, `bed = { dip = 6 }`, wobble a little,
  and never turn with a facing. They share the walls' fade band.

### Finding it

- **The seen bit (§6d) gates the kind's colour, its pattern and its ore
  together.** Unseen rock is one neutral per stratum (`unseen`), with neutral
  fractures and grit so it still reads as stone. A seen cell draws in over half
  a second. A kind that showed through its colour alone would give prospecting
  away.
- **Ore is set into the stone, as Minecraft does it.** A veined cell keeps its
  kind's fill and pattern, takes a faint cast of the ore's colour, and has one
  to four nuggets set in it, or a raw lump when it is richest. A vein is the
  set of cells carrying ore, never a line drawn over rock, and it depletes a
  nugget at a time. The `ore` layer reads a stock field (§4a). Ore is the one
  mark that outlasts the pattern fade: far out, a veined cell keeps a tint.

### The work

- **An order to dig is a cut line.** Plans draw what is to be removed as a
  dashed outline. A mine order is a dashed line around its region, on a
  keyline, over a light wash, in survey yellow (`#e9d44a`). At each cell's
  designation corner (§6f), a solid dot means a miner can reach it now, a
  hollow one that it waits behind the face, and an amber triangle that no one
  holds the tool. Bedrock takes no order: the preview crosses it out.
- **The stage (§6b) gains two layers and the strike one effect.** Fresh break
  is pale flakes on the worked side, since newly broken rock is lighter than a
  weathered face. From 6/8 a fracture crosses the cell. `sparks` fly when the
  worker's tool has the style's tag (`sparks = "metal"`) and the rock is
  `hard`. The cell never shrinks.
- **What is left:** the yield where the rock stood, and `hewn` floor, short
  pick strokes on the `leaves` terrain, so a dug gallery reads as dug.

### Holes

An air cell is a plan's floor opening: an outline, an X across it, and the
level below showing through with §6e's depth tint. The near rim (top, left)
is shaded and the far wall lit, which is the mass's light turned inside out.
Stairs keep §6c's symbol. Underground, light is §6e's: zero ambient, torches
baked into flicker channels, and sky down a shaft by its width and depth.

### Cost

Outline, relief, contours, patterns and ore are built with the chunk mesh.
Nothing new runs per frame except a reveal fading and the strike effects §6b
already pays for. Rock height needs no new texel format. Patterns share the
walls' fade band, and ore draws one fill per cell below 10 points a cell, so
the whole-map view pays what it does today. The render bench gains a quarry
view to hold that.

---

## 7. Determinism is non-negotiable

- All player input becomes a `Command` that is applied at a tick boundary.
- Integer or fixed-point simulation math, a seeded RNG owned by the world, and
  ordered iteration.
- Rendering interpolates. It never feeds back into the simulation.
- **Floating point is the same everywhere or not used.** Rust never fuses
  multiply-adds or enables fast-math, but clang does fuse them in C++ on
  arm64, so Luau is built with `-ffp-contract=off` and CI checks the binary.
  Library transcendentals (`sin`, `exp`, `pow`...) differ between platforms
  in the last bit, so scripts get rim's own (the `libm` crate, bit-identical
  everywhere, checked by test vectors in CI), and Rust sim code avoids the
  platform's.
- **Scripts are sandboxed for it:** only deterministic libraries, no memory
  or clock queries, the world RNG instead of `math.random`, and a runaway
  script is stopped after a *counted* number of steps, never a timeout, so
  every peer stops it at the same point. Rules for modders:
  [docs/modding/scripting.md](docs/modding/scripting.md); configuration:
  [docs/engineering/dependencies.md](docs/engineering/dependencies.md).

This buys us replays, reproducible bug reports ("seed + mod list + command
log"), desync-checkable **co-op lockstep multiplayer** later, and a headless
simulation for tests and CI.

## 7a. Saves

### Tension: a snapshot, or the seed and the command log?

- **For a snapshot:** loading ten in-game years must not mean simulating
  them, and a save must still load after a mod update, which changes what
  a replay of the old commands would do.
- **For seed + commands:** determinism (§7) makes it tiny and exact. It's
  what replays, bug reports, hot reload and co-op already are, and a log
  appended as you play loses nothing when the game crashes.
- **Ruling:** both, in one file. **The log is the save; snapshots are a
  cache.** A game is a pure function of a root state, the code that runs
  it, and the commands it's given. The save records the root and appends
  every command as it happens, so there's no moment when the game is
  "unsaved". Snapshots memoise the function so loading is fast, and any of
  them can be deleted and rebuilt.
- **When the code changes, an epoch starts.** An engine update, a mod
  update, or a mod added or removed changes the function, so the old log no
  longer replays. On load the newest snapshot is migrated once and becomes
  the root of a new epoch with an empty log. The first epoch's root is the
  seed.

```
save   = [epoch, …]
epoch  = { lock, engine version, root: seed | snapshot, log: [(tick, command)] }
cache  = snapshots by (epoch, tick), their sections stored by hash
```

Everything else is an operation on that: load is the newest snapshot plus
the log after it; a replay or a crash report is one epoch; co-op join and
resync send a snapshot and the log after it; rewinding truncates the log; a
save edited by hand is a new epoch whose root is the edited snapshot, the
same operation as a mod update. Migration happens only at an epoch
boundary, so it's the only place the format has to be read by a different
version of the code.

### Tension: what counts as a change of code?

- **A version per mod (what epochs recorded first):** simple, and authors
  already bump versions.
- **Against:** versions are typed by hand and cover the whole mod. Adding a
  theme mod starts an epoch and drops the unreplayed tail, for a mod the
  sim never reads. Editing a def without a bump keeps the epoch, and the
  log replays under different code until a checkpoint hash disagrees.
- **Ruling:** **an epoch keys on what the sim runs, by hash.** Each
  sim-side mod (§10) contributes a hash of its `mod.toml` sim fields,
  `defs/` and `scripts/`; the version is recorded but not compared. Colony
  options and sim picks count too. A mod with only `ui/` never starts an
  epoch. The load report names the input that differs.
  - The lockfile hashes every file of every mod, for integrity. That is a
    different job.
  - Presentation fields in defs (`look`, `color`, `label`) stay in the
    hash until no sim path reads text (§4g).

### Tension: say what changed before opening, or after?

- **After (today):** open the save under whatever mods are installed, then
  report what went: the ticks after the last snapshot, and the things whose
  defs are gone ("dropped 3 × boars:boar").
- **Against:** the player learns what they lost once it's gone. The new
  epoch has begun and the tail can't replay. And when the player has only
  *added* a mod since saving, nothing needed to go: every mod the save used
  is still there, and the loader already takes an `enabled` filter.
- **Ruling:** **a save says what changed before it opens.** The last epoch
  lists its mods and versions, so comparing them with what's installed
  needs no world. The title screen marks a save whose mods differ.
  Opening one first shows what was added, removed and updated, what the
  removed mods take with them in the player's words ("3 boars, from
  Wildlife+"), and how many ticks can't replay. When every mod the save
  used is still installed at its version, the first choice is **Open as
  saved**: load under the save's own mods, with no new epoch and nothing
  lost.
- **Old versions stay.** Installs live in a store keyed by id, version and
  hash. A version stays while a save uses it, and a missing one is fetched
  from the index, the same path co-op join takes. Bug bundles embed their
  mods' sim sides, so a report reproduces anywhere.
- **Forks inherit.** A mod may declare `migrates_from = ["old_id"]` and
  rename defs with `[[rename]]`, so a fork adopts a removed mod's things
  instead of dropping them. Each rename applies once, recorded in the
  epoch.

### What a snapshot holds

- **A header:** save format version, engine and API version, seed, tick,
  and the mod lockfile: every mod's id and version, in load order (0152).
- **Sections, by owner and name:** `engine:rng`, `engine:map`,
  `engine:field/temperature`, `core:data`, `weather:data`. The engine owns
  the `engine:` sections and versions them with the save format; each mod
  owns its own and versions them with the mod.
- **Components, one section each.** `engine:pawn`, `engine:thing`,
  `mood:thoughts`: a section holds every entity's value of that component,
  keyed by entity id. Grouping by component, not by entity, is what lets
  a removed mod's data be parked whole, lets a migration see exactly one
  mod's data, and lets an unchanged section be shared between snapshots.
- **Stable entity ids.** The world hands out entity ids from a counter it
  owns and saves, and never reuses one (hecs's `spawn_at`), so a hecs
  handle *is* the stable id: commands, the log, scripts and the save all
  use it, with nothing to translate. Where the sim picks between equals
  (the nearest food, the weakest door), it breaks the tie by id, never by
  the order hecs happens to iterate in, and sums floats in id order.
- **Def references are qualified ids** (`core:wall`, 0138), never `DefId`
  indices, which change whenever the mod list does. A string table keeps
  the repeated ids small: the snapshot stores ids as indices into its own
  table of qualified ids, and a load under different defs maps each by
  name. What no longer exists is dropped and listed in the load report: an
  entity whose def is gone, a material, a need.
- **A removed mod's data waits for it.** Its script data stays in the world
  and in every later snapshot, and is there again when the mod returns.
- **Nothing derived.** Paths, reachability regions, rooms, the wealth cache
  and def indices are rebuilt on load. If it can be computed, it isn't saved.

### Script state

The Luau VM isn't saved. On load, scripts run again and register their hooks
in the same order. Anything a mod needs to remember lives in **script data**
(`rim.set_data`), which is saved and in the state hash. The engine puts
each key in the calling mod's namespace, so script data is one section per
mod and no mod can write another's. A value kept in a Luau local is a
cache: it's lost on load, and invisible to the desync check.

### Encoding

- **Self-describing:** serde into MessagePack with named fields (it decodes
  2.6× faster than CBOR at the same size; numbers in
  [dependencies.md](docs/engineering/dependencies.md)), each section
  compressed with zstd. Self-describing because two jobs need to read data
  without its type:
  - components of a removed mod ride along untouched until it comes back
    (0063);
  - migrations work on plain data (0139).
- **Content-addressed:** a section's hash, over its canonical bytes, is its
  identity. The same hash checks the file, shares unchanged sections
  between snapshots (the terrain rarely changes, so it's stored once), and
  names the mod whose state diverged in a desync.
- **Append-only:** the file only grows: epochs, log chunks and snapshot
  sections, each with a checksum. A crash can only cut the tail, which
  costs the last few commands. Compaction rewrites it without old snapshots.
- **Versioned twice:** the format has a version, and so does each mod. A mod
  whose recorded version differs from the installed one gets its
  `rim.on_migrate` function called with that version and its script data,
  when the next epoch begins and before any hook runs. If any mod's
  migration fails, the load fails and the save is left as it was.

### Readable on request

`rim save unpack` writes a save as a directory, one text file per section,
and `rim save pack` turns it back into a save losslessly. `rim save diff`
compares two saves section by section and names the first entity that
differs. The game only ever reads the binary; the text form is for people,
bug reports and `rim test` fixtures.

### The guarantee, tested

- Save, load and save again gives the same bytes.
- Save at a tick, load, and carry on: every later snapshot equals the one
  from the game that never saved. The test loads entities in reverse order,
  so any hidden dependence on iteration order fails here, not in co-op.
- Load replays the log after the snapshot and checks the state hash at
  every checkpoint. If they disagree, the snapshot wins and the mismatch is
  reported: a determinism bug costs the tail, never the colony.

CI runs these on the crosscheck scenario on every platform. A save that
doesn't round-trip is a desync that hasn't happened yet.

---

## 7b. Seeds: every run is named, every failure comes back

The world owns one RNG, and 29 draw sites in seven files share it: terms,
systems, scripts, AI, world, mapgen and the map. A new draw anywhere
reshuffles every roll after it, so an unrelated change moves where wolves
spawn, which colonist a raid picks, and what `rim.random()` returns to every
mod. Tests written against what seed 1 does break for no reason (#207, #211),
and a balance number measured on one seed means little. The plan, with the
testing and CI around it, is at
<https://claude.ai/artifact/KUQcdtWkNG4u63T4S8uPpt>.

### Tension: one stream, or one per purpose?

- **One stream:** simplest, and it's what the save stores today.
- **Against:** every system and every mod perturbs every other. Installing
  mod B changes mod A's dice; a new AI roll changes the map's animals.
- **Ruling:** **a stream per purpose, derived from the world seed.**
  - Each stream is `Rng::new(mix(seed ^ hash_str(name)))`: `spawns`, `sim`,
    `ai`, `weather`, `story`, and one `mod:<id>` per mod behind
    `rim.random()`. Terrain already uses stateless hashes of position and
    stays that way.
  - A per-entity decision uses a counter-based draw, a hash of the stream,
    the entity, the tick and a draw index. It doesn't depend on the order
    systems run in, so it stays correct if they ever run in parallel.
  - Every stream's state is saved with the snapshot. The switch changes every
    map once, so it lands with a save-format bump, and saves from before it
    replay under their own engine version (§7a).

### Tests and seeds

A seed names a run, never a map. A test may use a seed to make its randomness
reproducible, never to rely on what that seed's map contains.

- **Scene tests build their own ground** and are the default for behaviour.
  **Seeded properties** assert invariants over a corpus of seeds.
  **Determinism tests** compare a run with itself (twice, after a save and
  load, on four platforms) and never with a stored hash, which would churn
  with every sim change.
- A test that doesn't name a seed gets `hash(test name) ^ RIM_SEED_SHIFT`.
  The shift is 0 on a PR and the date at night, so a test that secretly
  depends on its map fails at night, is quarantined, and is rewritten as a
  scene. Luau's `t.world()` does the same instead of defaulting to seed 1.
- Every failure prints the line that brings it back: the seed, how it was
  made, the tick, and the command to rerun it.
- `tests/seeds.toml` keeps maps worth keeping, each with a `why`, chosen with
  `rim seeds find`, which runs mapgen alone. A nightly sweep runs 200 seeds
  derived from the date; a failure files a cairn item with its repro line,
  and once fixed its seed joins the corpus, which every PR runs.
- Balance changes report distributions over 200 seeds, not one map.

---

## 8. Performance budget

Target: **250×250 map, 30 colonists, 200 total pawns, 6× speed, 60 fps** on a
little old laptop with mods loaded. The reference machine is a 2017
ultrabook: a four-core mobile CPU and integrated graphics (Intel UHD 620
class) at 1080p. That means ≤ 2 ms per sim tick at 6× (≈ 360 ticks/sec).

The frame's 16.7 ms is split: the sim's 2 ms a tick, the UI's 1 ms (§11),
and **4 ms of CPU for the world renderer** on the whole map at the lowest
zoom, the worst case. What is left is headroom for the GPU, the driver and
a mod or two that isn't careful. `rim --bench-render` measures it on a
dense colony, per pass, with draw calls; CI fails over budget.

- Each system declares a tick interval (every tick, every N ticks, or
  event-only), and work is **staggered** by entity id.
- Pawns think only when idle, on a staggered cadence. Jobs run as small state
  machines.
- **Reachability regions** (flood-fill ids, rebuilt only when passability
  changes) reject impossible targets in O(1) before A* runs.
- A* uses generation-stamped scratch buffers with no allocation per search.
  Hierarchical pathing and flow fields for raids come later, behind the same
  interface.
- Wealth and other aggregates are cached and recomputed on an interval.
- Rendering culls to the viewport. What doesn't move is drawn from
  per-chunk meshes, rebuilt when the chunk changes; every mod's sprites
  share one atlas, so the number of mods doesn't change the number of
  draw calls.
- Pixels have a budget too. The world draws into a texture of its own,
  at full resolution by default or a fraction of it (render scale, the
  command palette), then onto the screen in one copy; the UI always at
  full. One target, not several passes on the window: on macOS's GL each
  pass on the window's framebuffer costs more than all the drawing.
- Per-system and per-mod profiler overlay from day one.

---

## 8a. Proving: tests, CI and the merge queue

Rim lands about forty PRs a day, mostly from agents, on a public repo. Actions
minutes are free there; runner slots aren't (20 jobs at once, 5 of them
macOS), and time isn't. Until September a PR push ran every platform and
cancelled its previous run, a fifth of runs were thrown away, and a merge
needed a person-shaped queue that asked each PR to rebase and wait again.

- **The gate is one command, and CI runs that command.** `scripts/task check`
  runs what CI's jobs run; each CI job calls a `scripts/task` verb, and a
  pre-push hook runs `check`.
- **Tests count work; benches measure time.** No wall-clock assertion in the
  suite: a test asserts steps, cells visited or draw calls. Time lives in the
  benches, compared with main's last numbers on the same runner type.
- **A flake is fixed, not retried.** nextest keeps `retries = 0`. A test that
  fails once on main is quarantined the same day (it runs, and doesn't gate)
  with a cairn item, and leaves quarantine by counting work instead of time.
- **The queue proves; a push only checks.** A PR push runs Linux only. The
  full proof runs in the merge queue on the exact result that will land: the
  crosscheck and save round trip on Linux x64, Linux ARM, Windows and macOS,
  and the platforms' hashes compared. Full macOS and Windows suites, fuzzing
  and the 200-seed sweep run nightly. A push to main re-runs nothing heavy,
  and queue and main runs are never cancelled.
- **The queue is configured in the repo.** Mergify, from `.mergify.yml`,
  with a docs lane and a code lane. An agent adds the `queue` label; the
  queue tests the PR on top of main and the PRs ahead of it and merges in
  order. Nobody rebases by hand to chase main.

---

## 9. Milestones

1. **Castaway** (vertical slice): map gen, one warrior, harvest, mine, build
   walls/doors/beds, food and rest needs, animals, melee, the Luau storyteller
   with raids and wanderers, wealth, F3 profiler. Two mods loaded: `core` and
   an example plugin that adds content, patches core and scripts an incident.
2. **Shelter:** warmth, enclosed rooms, eras.
   **Weather** (a sprint inside it): seasons, weather and a forecast as the
   first-party plugin `mods/weather`, lighting and weather visuals (§4c).
3. **Save/load:** the log is the save and snapshots are a cache (§7a);
   unknown mod data is preserved.
4. **`rim.mood`:** the first first-party plugin, and the test of the API.
   **Story** follows it (§4g): perception, memories, relations, mourning,
   and text written from intents.
5. **Stockpiles and hauling, work priorities (§4d), skills.**
6. **WASM tier, mod browser, co-op lockstep.**

---

## 10. Modding as a platform

§6 says how plugins plug in. This section covers who writes them and how the
work gets to players. The first modders will be developers who already live on
GitHub, and the platform should feel like publishing a small open-source
library: `rim new`, write typed code, `rim test` in CI, tag a release, open a
PR to the index. If that loop is good, content follows.

### Tension: where do mods live?

- **For Steam Workshop:** it's where players already look, and installing takes
  one click.
- **Against:** it's closed and tied to one store. You can't fork a mod, send it
  a pull request, review a diff or run CI on it. A mod that is abandoned there
  stays abandoned. It also doesn't exist for players who got the game any
  other way.
- **Ruling:** **a mod is a git repo**, and a release is a tag. Discovery goes
  through a **mod index**: a public git repo with one small TOML file per mod
  (id to repo URL). Adding a mod is a pull request, and the index's CI runs
  `rim check` and `rim test` on it. Players never need git: the game and the
  `rim` CLI download release archives over HTTPS. A Workshop mirror can come
  later as a second front door onto the same index.
- Every install is recorded in a **modlist lockfile** (exact versions plus
  content hashes). The same file pins replays, bug reports and co-op sessions.
  A modpack is just a lockfile someone shared.
- The index requires an SPDX `license` in `mod.toml`, so modpacks and forks
  know what they're allowed to do.
- The index pins each release by content hash, so a moved tag can't swap
  code under a modpack. It also keeps a `compat/` directory where anyone
  can file a compatibility rule by pull request, and the crater's results
  land there. The loader names the source of each rule it applies.
- A dependency names a version range. `breaks = [{ id, versions, why }]`
  refuses a pair and shows why. There's no warn-only incompatibility:
  warnings get ignored, and contested slots have their own rule.

### Tension: global ids or namespaced ids?

- **For global ids (`wolf`):** they're short, and mods today can refer to core
  content without ceremony.
- **Against:** two mods that both add `iron` will collide, and with a hundred
  mods they will. Save files key on def ids, so renaming later breaks saves.
- **Ruling:** every def id is **namespaced by its mod**: `core:wolf`,
  `wildlife_plus:boar`. Inside a mod, a bare id means that mod's own def; any
  other mod's def needs the prefix. This is a breaking change, so it lands
  **before the save format** does, while breaking things is still cheap.

### Tension: how do mods talk to each other?

- **For a shared `rim` table (what we had first):** it's simple. Core exposed
  `rim.register_incident` just by assigning it.
- **Against:** any mod can overwrite any function for everyone. That is
  patch-anything through the back door, which §6 ruled out. Names collide
  silently, and nothing records who depends on whom.
- **Ruling:** the `rim` engine table is **read-only**. Mods share code as
  **modules**: `require("@core/scripts/storyteller")` returns what that
  script exports. A mod can only require mods listed in its `depends` or
  `optional`, so the
  dependency graph is real rather than hoped for. For loose coupling there are
  **namespaced events**: `rim.emit("wildlife_plus:stampede", data)` and
  `rim.on("wildlife_plus:stampede", fn)`. The rule of thumb: hard
  dependencies use `require`, soft ones use events.

### Tension: a fixed schema or one mods can extend?

- **For a fixed set of def kinds:** the engine validates everything and tools
  know the whole schema.
- **Against:** mood needs `thought` defs and crafting needs `recipe` defs.
  Neither is an engine concept, so the rule "if mood can't be a plugin, the
  API is wrong" fails immediately.
- **Ruling:** mods can **declare new def kinds** with a field schema. The
  loader validates, merges and patches them like built-in kinds and exposes
  them read-only to scripts. To attach data to *another* mod's def, a mod uses
  a table named after itself (`[creature.core:human.mood]` becomes
  `def.mood` in scripts). It can't collide with anyone else's data, and its
  owner is obvious.
  Declared kinds landed with 0208: `[[kind]]` with typed fields and
  defaults, `[[weather.type]]` from other mods, and `rim.defs(kind)` in
  scripts. The weather plugin's types are the first. Extension tables and
  references to other kinds come with 0143.
- Content enums in the engine (`Faction`, `Satisfier`) become registries fed
  by defs. Draw primitives (a look's fill, outline, disc and edges, §6a)
  and broad categories stay in code: those are mechanisms, not content.

### Tension: declarative patches or scripted defs?

- **For letting scripts generate and edit defs:** it's what power users want,
  e.g. twenty ore variants from one loop.
- **Against:** conflict detection, compatibility reports and index checks
  only work if changes are declarative.
- **Ruling:** stay declarative, and fill the actual gap, which is lists.
  Patches gain `append`, `remove` and match-by-key for arrays; today setting
  a list replaces all of it. Load-time def generation is **deferred** until a
  real mod needs it. If it comes, it must emit defs and patches through the
  same tracked pipeline. Until then, twenty ore variants are generated at
  authoring time by any script, committed and reviewed like code.

### Tension: a mod list, or a game?

- **For showing the list:** it's honest. rim is a set of mods, and hiding
  that hides the platform.
- **Against:** the first-party plugins are split for modders, not players.
  Iron needs four mods. A new player meets six switches that mean nothing
  to them, and each one is a way to leave the balanced game without
  knowing it.
- **Ruling:** **players choose a game, and the list is one click away.**
  - The default game is a **set**: a named list of mods (`sets/rim.toml`)
    that the default install is balanced as (§6, "Costs we accept").
    `wildlife_plus` is an example, so it stays installed but out of the
    set.
  - New colony shows the game as one line, "rim", with Start ready.
    Change opens the mods. Leaving the set changes one label, "Untested
    combination: rim +Wildlife+". It never blocks and never warns.
  - A save remembers the set it started from, so the title screen and the
    top bar can name the game.
  - A set lists sim-side mods only, may pin colony option values, and
    carries `aliases` so a renamed set still names old saves. When the
    lockfile lands, a set is a lockfile that ships with rim, and a modpack
    is one someone shared.
  - Dev-only plugins (`mods/devtools`, §11a) are never in a set and never
    listed outside a `--dev` game.
  - Games started from the command line, like the autotests, still load
    every installed mod unless they name a set.

### Tension: does every mod belong to the colony?

- **For one list:** simpler, and a mod is a mod.
- **Against:** §11 promises each co-op player their own UI mods. With one
  list, a theme mod starts an epoch in every save, and co-op refuses a
  player over a colour. Factorio requires every mod to match in
  multiplayer, so players install a friend's minimap to join.
- **Ruling:** **a mod's side comes from its folders.** `defs/` or
  `scripts/` put it on the sim side, and its `ui/` still loads in the
  client. A mod with only `ui/` is client-side: the player's own. Sets,
  epochs, seed codes, "Open as saved" and the co-op handshake compare sim
  sides only. A sim-side mod may not depend on a client-side one. There's
  no `side` field: a declared side can be wrong, and a derived one can't.

### Tension: do mods get settings?

- **For:** players want to tune a mod (wolf density, winter length)
  without editing TOML. A modder with no settings fakes them with patches.
- **Against:** a setting that changes the simulation is input (§7). If it
  lived in a per-player config file, two machines would run different
  games from one save, and a replay or a bug report couldn't say which.
- **Ruling:** **mods declare typed options in three scopes, and what the
  sim reads belongs to the colony.**
  - A mod lists `[[option]]`s in `mod.toml`: an id, a name, a kind (bool,
    an integer in a range, or a choice), a default and a scope. Values are
    never floats, and the engine types and defaults every one.
  - **colony** options are chosen on New colony and recorded in the epoch,
    then the lockfile. They may **gate patches** (`when = { option = ...,
    is = ... }`) and scripts read them. Changing one later is an epoch
    boundary, like a mod update.
  - **live** options are read by scripts only. A change is a `Command`,
    logged and replayed; in co-op the host decides.
  - **player** options live beside the client's settings and are read by
    the UI VM only. The sim can never read them.
  - Gated patches resolve before patches merge, so conflict detection sees
    exactly the patches that apply. Factorio's startup settings and
    Content Patcher's `When` do the same; RimWorld's per-user settings
    shaping the sim are the case to avoid.

### Tension: should players set the load order?

- **For manual order:** RimWorld players expect it, and it's an escape hatch
  when two mods fight.
- **Against:** it's hidden state. Two players with the same mods get different
  games, co-op desyncs, and bug reports can't be reproduced. Ordering is a
  job for the loader, not for players.
- **Ruling:** **order is derived from manifests only** (§6). When mods
  conflict, the next tension's ladder settles it, and a player's pick is
  written into the modlist lockfile, so it is explicit, shareable and
  reproducible. Mod authors resolve conflicts properly with `load_after` or
  a compatibility patch.

### Tension: when two mods want one slot

- **For a rule per mechanism:** each mechanism knows its own stakes. A
  missing calendar is fatal; a replaced button is cosmetic.
- **Against:** the rules drifted into five answers across nine mechanisms:
  a load error (a second built-in singleton, a second level generator),
  the last one silently winning (`[[start]]`), the first one silently
  winning (room roles), a warning with load order deciding (patched fields,
  replaced lists, `ui.replace`, theme tokens), and a planned pick that
  blocks the game (claims). Nine rules is nine things to learn, and two of
  them are silent.
- **Ruling:** **one ladder for every contested slot.** A slot is one field
  of one def, one UI id, one theme token, one level generator or one claim
  tag. It resolves by the first rung that applies:
  1. **A compatibility mod.** A mod that depends on every contender and
     sets the slot itself wins, and no one is asked.
  2. **The player's pick.** Sim picks go in the lockfile, client picks in
     the player's settings. A pick is keyed by the slot and its
     contenders' versions, so an update asks again instead of reusing a
     stale answer.
  3. **Load order**, labelled "decided by load order" on New colony and in
     F3.
  - A contest is never silent and never blocks a game. Redefining a
    singleton stays a load error, because that's an authoring mistake;
    two patches to its fields are the contest.
  - No ecosystem we found lets players pick field winners, so the first and
    last rungs should settle almost everything. Content Patcher's
    `Exclusive` loads, where two claimants cancel each other out, are the
    failure to avoid.

### Tension: what can a mod from a random repo do?

- **For native code (DLLs, like RimWorld's C# assemblies):** unlimited power
  and speed.
- **Against:** "download code from a stranger's GitHub and run it" is only
  safe if the sandbox is real. One malicious mod would poison trust in the
  whole index.
- **Ruling:** **there is no native tier, ever.** Data and Luau mods can't do
  I/O. The Luau VM runs in sandbox mode, the engine tables are frozen, and
  each mod gets hard instruction and memory limits, so a runaway script is
  stopped and named instead of hanging the game. Installing a data or Luau
  mod needs no permission prompt. A capability comes from how the game was
  started, never from a prompt a player clicks through: `dev` works only in
  a `--dev` game (§11a). The WASM tier (§6) declares the rest.
- Scripts share the sim's determinism rules. Library math that can differ
  across platforms (`math.sin` and friends from the C library) is replaced
  with deterministic implementations, so co-op and replays hold across
  machines.

### Tension: a moving API or mods that rot?

- **For breaking freely before 1.0:** the API is young, and freezing it early
  locks in mistakes.
- **Against:** every break silently kills mods whose authors have moved on.
  Mod ecosystems die this way.
- **Ruling:** break freely, **but never silently**:
  - The API is declared once in Rust. The `.d.luau` type definitions and the
    reference docs are generated from that declaration, so they can't drift.
  - A deprecated call keeps working for one minor version, logs a warning
    with its replacement, and names the version it will be removed in.
  - **Mod crater:** engine CI runs every indexed mod's tests against the
    change. Breaks are found by us, before modders find them, and the
    results are published to the index, so the game can say "known broken
    on this engine" before a colony starts.
  - **Data outlives code.** Declarative data (`mod.toml`, defs, patches,
    themes) is read through a shim for each past API minor, so a data-only
    mod keeps loading across engine updates, as Content Patcher's `Format`
    keeps old packs working. Only scripts follow the deprecation window. A
    data format change lands with its shim and a fixture mod.

### Tension: how do modders know their mod works?

- **For "just play it":** it's how most games work.
- **Against:** the engine is deterministic and headless (§7). Not letting
  modders test with that wastes our biggest advantage.
- **Ruling:** **`rim test`** runs `tests/*.luau` against seeded headless
  worlds: set up a scene, advance ticks, assert. The GitHub Action that
  `rim new` generates runs it on every push, against the engine versions
  the mod supports.
- **Hot reload is deterministic replay.** When a file changes, reload the
  defs and scripts, rebuild the world from its epoch's root (the seed, or
  the snapshot a save was loaded from) and replay the log to the current
  tick. You see what your change *would have done* in this exact game.
  Later snapshots can't shorten this: the change applies from the root.

### Tension: Luau, or a language more developers know?

- **For TypeScript or JavaScript:** far more developers know it.
- **Against:** a JavaScript engine is heavy, hard to sandbox deterministically
  and slow to embed. Luau is built for exactly this job: sandboxed, gradually
  typed, fast, and with a solid LSP (luau-lsp).
- **Ruling:** Luau for scripts. People who want other languages get them
  through the WASM tier, which compiles from Rust, AssemblyScript, Zig and
  others.

---

## 11. Interface

The UI follows the same split as the game: the engine provides a few UI
mechanisms, and **the interface you see is a mod** (`mods/core/ui/`). Any
panel, bar, menu or bubble can be extended, replaced, wrapped or removed by
another mod.

### Tension: immediate-mode widgets or a retained tree?

- **For immediate mode (egui, Dear ImGui, today's HUD):** quick to write, and
  there's no tree to keep in sync.
- **Against:** the only way to change it is to edit Rust. There's nothing to
  address, so mods can't reach in, restyle it or test it.
- **Ruling:** a **retained tree described in Luau**, built React-style: a
  component is a function from a read-only view of the game to a tree of
  plain nodes. The engine does layout (a flexbox subset via `taffy`),
  drawing, input and caching. Because a tree is data, mods can find a node by
  id and change it, tests can compare trees as text, and devtools can show
  which mod put what on screen.

### Tension: one Luau VM or two?

- **For sharing the sim's VM:** one runtime, and mods can call their own sim
  code directly.
- **Against:** the sim VM is deterministic and synchronised in co-op. UI code
  wants the wall clock, animation and per-player choices, and a UI bug must
  never be able to desync a game.
- **Ruling:** the UI runs in its **own client-only Luau VM**, sandboxed like
  the sim's (§10). It can only *read* the world through `view` and *act*
  through `act`, which queues the same Commands mouse and keyboard produce.
  Each co-op player can run different UI mods.

### Layout: a shell of regions and layers

- **Regions:** `top`, `bottom`, `left`, `right` dock panels at the screen
  edges; panels declare a region, an order and size limits, and regions
  stack them. The world stays visible under translucent panels.
- **Layers**, bottom to top: world, anchored (labels, bars, bubbles tied to
  entities or cells), docked, float, windows, menus and tooltips, modal,
  toasts. Input goes to the top layer first; whatever the UI doesn't handle
  falls through to the world.
- **Anchored UI avoids collisions:** labels near each other are nudged apart
  by priority (selected, colonists, hostiles, others), which fixes overlapping
  names once for everything.

### Tension: should a region grow with what's in it?

- **For:** a region sized to its content never wastes space, and a tray that
  pushes the map up keeps every cell visible.
- **Against:** the bottom region held the undo toast, the Build tray and the
  placing pill, so every order, every pick and every Escape changed its
  height, and the inspector and the hover card, which sit on the band's
  floor, jumped by tens to hundreds of pixels. A panel that moves under the
  pointer is a misclick waiting to happen.
- **Ruling:** the HUD holds still.
  1. **The frame is fixed.** The top bar and the dock bar have fixed heights,
     so the map band between them never changes size.
  2. **What comes and goes floats.** The `float` layer places panels over
     the map, from the left column's edge, above the dock; it never reflows
     the shell. Trays live there. One too wide for the room slides left to
     stay on screen.
  3. **Status lives in the bar.** What the next click does (the placing
     pill) and what can be taken back (Undo) sit in the dock bar's
     `core:dock.status` slot, not above it.
  4. **Headers don't move.** A panel docked at a column's end reserves a
     `slot`, a minimum height, and hangs from its top, so content that
     changes height grows down into the slot and the header stays put.
  5. **A test holds it.** `rim_ui`'s `steady` tests play select, tabs, an
     order, a tray, a pick, Escape and a busier hover, and fail if the dock,
     the colonist list or a docked header moves.
- The right column's start stack is not held yet: an alert arriving still
  pushes the news down.

### Looks: plain, minimal, tokens

- Every visual value is a **token** in `ui/theme.toml` (spacing on a 4 px
  grid, three text sizes, colours, radius, borders). Mods patch tokens like
  defs, with the same conflict reporting.
- Translucent dark surfaces, 1 px hairlines, one accent colour, no textures.
  Emphasis comes from weight and colour, not size.
- **Inter, shipped by core** (SIL OFL 1.1, `mods/core/ui/fonts`): the same
  letters and measurements on every platform, CI included, so a layout that
  fits on one machine fits on all. The system UI font drew badly (the text
  stack applies a variable font's weight but not San Francisco's optical
  size, so small text came out in its narrow Display cut) and differed by
  machine. System fonts remain the fallback for other scripts, and a mod
  can ship fonts under `ui/fonts/` for a theme to name.
- One **UI scale** multiplies every token and follows the display's DPI.

### Tension: stable ids or free-form trees?

- **For free-form:** less ceremony for small mods.
- **Against:** a mod can only change what it can name.
- **Ruling:** every component has a **namespaced id** (`core:clock`,
  `core:inspector.tabs`). Mods get four operations, applied in load order:
  `ui.extend` (add children), `ui.replace`, `ui.wrap` and `ui.remove`. Two
  mods replacing the same id is a contested slot (§10), like def patches;
  the player's pick lives in their own settings.

### Built for hacking

- **Hot reload:** saving a UI script or theme updates the running game
  without touching the simulation.
- **Devtools (F12):** hover any element to see its id, owning mod, layout box
  and tokens.
- **Per-mod UI time** shows in the profiler; a component that errors shows an
  error box in its place and the rest of the UI keeps running.
- Budget: **under 1 ms per frame** for the whole UI.

### How it's built, and what it costs

- **Stack** (decided by spike 0162): `rim_ui` is a crate with no renderer
  dependency. It takes Luau component trees in and puts a draw list and a
  glyph atlas out, so layout, text, routing and the VM are tested headless in
  CI. Layout is taffy's flexbox; text is shaped by cosmic-text and
  rasterised into an atlas the client uploads only when it changes. The UI
  works in physical pixels, so text lands 1:1 on the screen at any DPI.
- **Budget**, 30 colonists, profiler open, 349 nodes: **0.12 ms median
  frame**; 0.66 ms on the frames that rebuild trees. Trees rebuild on input or
  client change and otherwise at 20 Hz: hover and press restyle at paint
  time and anchored labels are re-placed every frame, so nothing visible
  goes stale in between. Layout is cached by tree hash, and small trees
  (labels, tooltips) by content.
- **Numbers shown to scripts refresh at 4 Hz.** A readout that changes every
  frame would otherwise re-lay out the whole shell every frame (it did: 220
  layouts in 230 frames before this rule).
- **Lesson:** a layout measure function must return the size flex decided
  when it's given one. Ignoring it collapsed every spacer to zero and
  stacked docked panels at the top; the shell test now checks that panels sit
  *against* their edges, not merely inside the screen.

---

## 11a. The workbench: acting on the sim from inside the game

The UI has its tools (§11: F12, hot reload, error boxes). The sim has none:
there is no console, no single-tick step, no way to see an entity's raw
state, and defs and sim scripts don't reload. Agents, who write most of rim,
can't ask a running game a question at all.

### Tension: a debug side door, or commands?

- **A side door** (poke the ECS directly) is quick to build.
- **Against:** it breaks the one rule everything rests on (§7). A save
  touched by it no longer replays, and a bug report no longer explains itself.
- **Ruling:** **a dev tool is a command.** Spawn, set a stat, heal, kill,
  teleport, fire an incident, force weather and finish a build are a
  `Command::Dev` family: logged, replayed, and marking the save's epoch
  `dev_touched`, so achievements and balance statistics can ignore the run.
  The epoch's engine version becomes the git commit, so a replay across
  commits reports a version change instead of a divergence.

### Tension: an engine feature, or a plugin?

- **In the engine:** one place, no API to design.
- **Against:** §6's rule. A console, an inspector and a timeline are UI and
  opinions, and every one of them is something a modder will want to extend
  or replace, as the weather mod already adds its own F12 panel.
- **Ruling:** **mechanisms in the engine, tools in a plugin.** The engine
  provides dev commands, a dev read API (an entity's raw components, job,
  reservations and path; the session's snapshots; stepping), and the dev
  port. Everything you see is `mods/devtools`, a first-party plugin that
  loads only with `--dev`. A mod gets the dev API by declaring the `dev`
  capability, and only in a `--dev` game.

### What the plugin holds

- **Console:** a Luau REPL over the read API and a `dev.*` table.
- **Time:** a single tick, N ticks, run until a condition, fast-forward on a
  worker thread, and this session's snapshots as a timeline to scrub and
  branch from (§7a's log truncation).
- **Inspector:** any entity's raw components, job step, reservations and
  path, what touched it, and watch expressions that pause the game.
- **Profiler:** per-system history with p99 against §8, and a spike trap that
  pauses and snapshots on a tick over budget.
- **Log:** one structured log with per-mod targets, in a file and a panel.
- **Bug capture:** F8 writes one bundle (save, mod lock, commit, seed, the
  last day of commands, a screenshot, the log); `rim repro` opens it at its
  tick.

### Tension: only for people?

- **Ruling:** **what a person can do in the workbench, an agent can do over
  the wire.** `rim --dev-port` serves the console's API as JSON lines on a
  local socket, headless or windowed, and `rim mcp` exposes it as an MCP
  server. Everything an agent does is a logged command.
- `mods/devtools` ships in release builds and loads only with `--dev` or a
  setting, since a modder needs it (§10) and a player shouldn't stumble into
  it.
