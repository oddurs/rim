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

There is no difficulty slider and no scenario picker, so the game needs one
rule that produces its own difficulty curve. That rule is **Wealth is gravity.**
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

There are no scenarios, so the shape of a run has to come from inside the game.

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
- **Against:** a fixed goal is a scenario in disguise, and you said no scenarios.
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
  cut off from the map edge, and at most `MAX_ROOM_CELLS` (400, i.e. 20×20)
  in size. Enclosed rooms count as indoors. The size cap is what an automatic
  roof would do: a valley ringed by mountains is not a house.
  - Doors are a def flag (`door = true`): passable for pathing, but they bound
    rooms like a wall does.
  - Rooms rebuild only when a wall, door or terrain changes, not every tick.
  - Scripts ask with `rim.indoors(x, y)` and `rim.room_at(x, y)`.
  - Explicit roofs, if ever wanted, are a plugin that marks cells roofed and
    hooks the same question.
  - **Revised in §6c:** the size cap becomes a roof span. A room is indoors
    when every cell is within reach of a wall or a pillar.

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
| `mods/weather` | Seasons (by patching core's terms), weather types, the forecast, weather incidents, the weather HUD |

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
  and cold rain, read where the colonist stands. Terrain inputs come with
  stock fields and farming.
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
  lightmap, so campfires glow and storms darken the map. The sky tint is a
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
  (the lightmap only rebuilds when emitters or rooms change).
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
- **`[[priority_scale]]`:** `levels = 4` in core. A mod that wants 9 changes
  one line and the UI follows. 0 means never.
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

### The Work Board

A panel in core's UI mod, so a mod can patch or replace it.

- **Painted, not typed.** Drag across cells to paint a value, scroll a cell
  to nudge it, press a number while hovering, shift-scroll a column. A cell
  shows the priority by brightness, the skill as a bar and passion as a
  flame.
- **Columns show demand:** jobs waiting, the backlog's trend, and coverage.
  A column with work waiting and no one on it at a high priority is marked.
- **Rows show now:** the current job, a 24-hour schedule strip, time idle.
- **Effective values are visible:** a cell reads `2→1` when a rule or stance
  moves it, and hovering explains why.
- **The why panel** on a colonist: what they picked and its score, and each
  work type they passed over with the reason, linked to the map.
- **On the map:** hovering a column lights its waiting jobs; hovering a job
  shows who would take it and when ("Bo in ~20s, then Cyd"). A right-click
  order still forces it.

### Cost

Choosing work costs work posted and pools checked, not map size. At 200 pawns
the budget is under 0.2 ms a tick for work choice, measured with a stress map
of every cell designated.

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

## 5. What's in `core` and what isn't

`core` is the smallest complete game. Everything else is a plugin, including
things we build ourselves.

| In `core`                                     | Out (plugins, first-party or community) |
|-----------------------------------------------|-----------------------------------------|
| Terrain, strata, plants, map generation       | Seasons and weather (`mods/weather`)    |
| Needs: food, rest, warmth                     | Mood, mental breaks, relationships      |
| Calendar, day and night, the atmosphere names | Other biomes, rain runoff, fire spread  |
| Harvest, mine, dig, build, haul (delivery)    | The stone age (`mods/primitive`)        |
|                                               | Bills (`mods/crafting`), research       |
| Melee combat, health, death                   | Ranged weapons, armour, medicine        |
| Wild animals, predators, hunting              | Taming, farming animals                 |
| Storyteller, wealth, eras                     | Trade, factions, diplomacy              |
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
   weather plugin is what sets it.
3. **Depth goes out.** Anything that deepens a loop rather than completing it
   is a plugin: seasons, weather, mood, crafting chains.

The test: with every plugin removed the game is complete but shallow (a CI
run proves it); with core removed nothing works, because the names are gone.

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
- **Ruling:** stacked planes. A level is a `Map`, unchanged. Levels meet
  only at **portals** (stairs, ladders, anything whose def says `portal`),
  which cover two cells one above the other the way a multi-cell thing
  covers its footprint. There are tens of portals, not thousands, so every
  3D question becomes a 2D one plus a small graph:
  - Positions carry `z`; 0 is the surface, and a missing `z` means 0.
  - Regions, rooms, fields and chunk caches stay per level, with per-level
    dirty flags. A wall on −2 rebuilds −2 and nothing else.
  - Reachability is a union-find over `(z, region)` joined at portals,
    rebuilt in O(portals) when a level's regions or a portal change. Asking
    stays O(1).
  - A path is planned leg by leg: a route over the portal graph, then A* on
    one plane to the next portal with today's scratch buffers. The next leg
    is planned on arrival.
  - A level nobody has dug into is **untouched**: a function of the seed and
    `z`, generated from `[[stratum]]` defs the tick something breaks in, and
    never saved until then.

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

- **Ruling:** a terrain may be `solid`: it fills its cell, blocks movement
  and bounds rooms, which already treat impassable terrain as boundary. Its
  `mine` block gives work, required tags, yields and the floor it leaves.
  Progress lives on a thing (§6b), so designating a rock cell spawns a
  worksite entity that holds the `Work` and is gone with the rock.
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
- Rock becoming terrain breaks mods that patch `thing/granite`, and bumps
  the API.
- Basins simplify: no currents, no pressure, and water never climbs stairs.
- One cell of height per level: no ramps or slopes, and hilltops are flat.
- Cross-level pathing, drawing the level below and generating a level are
  not yet measured. Each ticket that adds one records its number here.

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
    without perspective.
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
- **Roofs still cost nothing to build.** Explicit roofs stay a plugin
  (§4), and this is the question it would hook.

### Tension: do rooms know what they are for?

- **Scripts only:** a mood plugin could classify rooms in Luau. But eras
  (Camp: "a shelter, a bed and a fire"), mood ("slept in a barracks") and
  the storyteller would each classify them again, and could disagree.
- **Ruling:** **room roles are data.** A `[[room_role]]` has `needs` (tag
  counts) and optional limits (size, indoors). In load order, the first
  role a room meets names it. The engine counts the tags inside each room
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
- Pixels have a budget too. The world can draw at a fraction of the
  screen's resolution (render scale, the command palette), the UI always
  at full; a high-DPI screen defaults to its logical resolution, a quarter
  of the pixels at 2x.
- Per-system and per-mod profiler overlay from day one.

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
  same tracked pipeline.

### Tension: should players set the load order?

- **For manual order:** RimWorld players expect it, and it's an escape hatch
  when two mods fight.
- **Against:** it's hidden state. Two players with the same mods get different
  games, co-op desyncs, and bug reports can't be reproduced. Ordering is a
  job for the loader, not for players.
- **Ruling:** **order is derived from manifests only** (§6). When patches
  conflict, the mod manager shows the conflict and the player picks a winner
  per field. That choice is written into the modlist lockfile, so it is
  explicit, shareable and reproducible. Mod authors resolve conflicts
  properly with `load_after` or a compatibility patch.

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
  mod needs no permission prompt. The WASM tier (§6) is the only one that
  declares capabilities.
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
    change. Breaks are found by us, before modders find them.

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
  entities or cells), docked, windows, menus and tooltips, modal, toasts.
  Input goes to the top layer first; whatever the UI doesn't handle falls
  through to the world.
- **Anchored UI avoids collisions:** labels near each other are nudged apart
  by priority (selected, colonists, hostiles, others), which fixes overlapping
  names once for everything.

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
  mods replacing the same id is reported as a conflict, like def patches.

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
