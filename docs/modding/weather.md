# Modding climate and weather

rim's weather is built from a few general engine mechanisms, and the weather
itself is a plugin: [`mods/weather`](../../mods/weather/). It uses only the
API described here, so anything it does, your mod can do too, or do instead.

The design and its reasoning are in [DESIGN.md §4c](../../DESIGN.md).

## Who owns what

| Layer | Owns |
|---|---|
| Engine | Terms and curves, the calendar, named contributions to outdoor values, script data and events |
| `core` | The calendar, the sun, and the *names* of the atmosphere: `temperature`, `daylight`, `light`, `cloud`, `precipitation`, `wind`, `wind_dir`, `fog`. Day and night as data. No weather |
| `mods/weather` | Seasons, weather types, the forecast, cold snaps and storms, the weather readout, and what the weather leaves on the ground: `weather:wetness` and `weather:snow` |

Core declares `precipitation` even though only the weather plugin sets it.
That's deliberate: the renderer draws rain from it, and a farming mod can read
it, without either depending on the weather plugin. Remove `mods/weather` and
the game plays with core's mild, unchanging climate.

## Outdoor values are terms

Every field has an outdoor value (`ambient`). It can be a constant, or a sum
of **labelled terms**. A term is a `scale` times a product of inputs, and any
input can go through a piecewise-linear `curve`. Core's temperature:

<!-- not a sample -->
```toml
[[field]]
id = "temperature"
# ...
[field.ambient.mean]
of = [10.0]

[field.ambient.day]
scale = 9.0
of = [{ input = "hour", curve = [[0, -0.6875], [3, -1.0], [15, 1.0], [24, -0.6875]] }]
```

That reads: 10°C, plus 9° times a curve over the hour of the day.

| Input | Value |
|---|---|
| `{ input = "hour" }` | Hour of the day, 0 to 24 |
| `{ input = "year" }` | Fraction of the year, 0 to 1 |
| `{ ambient = "cloud" }` | Another field's outdoor value |
| `{ field = "temperature" }` | Another field's value at the cell being read (a derived field's terms); elsewhere its outdoor value |
| `{ noise = "gusts", hours = 2 }` | Smooth noise from -1 to 1, changing over about `hours` game hours |
| `{ terrain = "fertility" }` | A property of the terrain at the cell being read (`props` on `[[terrain]]`); 0 where it doesn't give one, or there is no cell |
| `{ near = "water" }` | Cells to the nearest terrain with that tag (`tags` on `[[terrain]]`), counting diagonal steps, up to 16; 16 where there is no cell |
| `4.5` | A constant |

- A field with `kind = "derived"` has `value` terms instead of `ambient`,
  worked out wherever it's read and never stored. Core's `feels_like` is
  `temperature` less wind chill and cold rain, and the warmth need reads it.
- Terrain **props** and **tags** are any names a mod likes. Core gives every
  terrain `fertility` (1 on grass), `drainage` (0 to 1) and `water_table` (0
  to 1), and tags water `water`. A mod adds a prop by giving it to its own
  terrain, or by patching core's (`set = { props = { salinity = 0.4 } }`).
  Reading a prop or tag no terrain has is a load warning, and it reads 0
  (or nothing near). `near` is kept as a grid for each tag some term reads,
  and a terrain change patches it within 32 cells of the change.
  `rim.terrain_prop(x, y, name)` reads a prop from a script.
- A **curve** is up to 16 `[x, y]` points with increasing `x`. Between points
  it's a straight line; beyond the ends it holds the end value.
- Fields are evaluated in dependency order: `temperature` can read `cloud`.
  A cycle is an error when the game loads, and so is a typo in a field or
  input name. The error names your mod, the field and the term.
- Evaluation is fixed point, so every machine gets exactly the same numbers.
  Don't compute simulation values in Luau with `math.sin` or `math.cos`: they
  can differ in the last bit between platforms.

### Fields that remember

A field with `kind = "stock"` keeps a value in every cell and changes it
over time, so the ground can stay wet after the rain stops. Instead of
`ambient` it has:

| Key | What it is |
|---|---|
| `rate` | Terms for how fast the value changes, in its units per game hour |
| `base` | Terms for what it settles to, which `rate` reads as `base` and `above_base` |
| `init` | Terms for each cell's value when the map is made (0 without) |
| `period_minutes` | How often each cell is worked out, 60 by default |
| `levels` | `"surface"` (the default) or `"all"`, for what lives underground too, such as ore |
| `move_cost` | A curve from the value to extra percent move cost: snow `[[0, 0], [20, 50], [50, 150]]` |
| `move_cost_by` | A terrain prop that scales `move_cost` per cell: the weather plugin's wetness uses core's `mud`, so soaked soil slows a walk and wet sand doesn't |
| `range` | The value is kept within it |

```toml
[[field]]
id = "wetness"
label = "wetness"
kind = "stock"
range = [0.0, 1.0]
color_low = "#cdbb85"
color_high = "#3d7ab8"

[field.base.ground]
of = [{ terrain = "water_table" }]

# Half the way back to the ground's own dampness each hour.
[field.rate.settle]
scale = -0.5
of = [{ input = "above_base" }]

# Rain soaks in where the sky is open.
[field.rate.rain]
scale = 0.1
of = [{ field = "core:precipitation" }, { input = "sky" }]
```

Rate terms may read `{ input = "self" }` (the value here), `"base"` and
`"above_base"`; every term at a cell may read `{ input = "sky" }`, 0 in an
enclosed room and 1 elsewhere. Emitters on a stock field add to its rate, so
`emit = [{ field = "wetness", amount = 2.0, radius = 2 }]` is a sprinkler.
Scripts change it with `rim.field_add(id, x, y, amount)` and
`rim.field_set(id, x, y, value)`. The values are saved and in the state hash.

### Veins: what the rock holds

A stock field with `levels = "all"` and no rate terms holds what map
generation lays into it, and costs nothing a tick. A `[[vein]]` lays blobs
of whole units into it through the terrains it names; a harvest with
`draw` takes its yield from the field where it's worked, and isn't offered
once the cell has less than `per` left. Primitive's flint lies in chalk:

<!-- not a sample -->
```toml
[[vein]]
id = "flint"
field = "flint_vein"          # a stock field, levels = "all"
in = ["core:chalk"]
chance = 0.04                 # of the chalk cells, where a vein starts
size = [3, 7]                 # cells in one vein
amount = [4.0, 10.0]          # flint in each of them

# The chalk's second harvest: up to three flint a time, the rock left standing.
draw = { field = "primitive:flint_vein", per = 1.0, most = 3 }
```

Veins are placed by hashing the seed, the vein's id and the cell, after
every level's scripts have run: the same seed lays the same veins.

### Plants that grow with the weather

A plant grows from a seedling to grown by terms read where it stands:

```toml
[[thing]]
id = "reed"
label = "reed"
color = "#7a9a50"
category = "plant"
natural = true
spawn = { terrain = ["core:marsh"], density = 0.05, spread = true }

[thing.grow]
days = 3            # seedling to grown at a rate of 1
after_harvest = 0.5 # a harvest it survives cuts it back this far

# How fast: 1 is `days`, 0 or less holds it (dormant, drawn faded).
[thing.grow.rate.growth]
of = [
  { field = "core:temperature", curve = [[4, 0.0], [18, 1.0]] },
  { terrain = "fertility" },
]

# Health lost a day; at none it dies. A tender plant fears frost.
[thing.grow.harm.frost]
of = [{ field = "core:temperature", curve = [[-6, 2.0], [-1, 0.0]] }]
```

- Every term reads at the plant's cell, like a derived field's: `field`,
  `terrain`, `near` and `{ input = "sky" }` too. Terms add, so a factor that
  should multiply goes into the same term's `of`.
- A harvest with `destroy = false` regrows with the plant: it's ready when
  the plant is grown again, so nothing regrows in a winter. Give the
  harvest `regrow_days` to keep to days instead.
- A plant felled young yields by how grown it is. Wild spread prefers cells
  where the plant grows fast, and a spread plant starts as a seedling.
- The weather plugin adds `weather:wetness` to core's oak and berry bush by
  patching their `growth` term (`mods/weather/defs/plants.toml`).

### Growing zones and crops

A crop is a plant that grows (`grow`) and that a work of its own raises
(`build.by`): the farming plugin's potatoes are sown, not built. With the
farming plugin loaded, a mod adds one like this:

<!-- not a sample -->
```toml
[[thing]]
id = "turnip_plant"
label = "turnips"
color = "#6a9a48"
category = "plant"
look.layers = [{ draw = "disc", r = 0.28 }]
harvest = { designation = "core:harvest", work = 80, yields = [{ thing = "farming:potatoes", count = 6 }] }
build = { by = "farming:sow", work = 50, free = true }

[thing.grow]
days = 6

[thing.grow.rate.growth]
of = [{ field = "core:temperature", curve = [[3, 0.0], [15, 1.0]] }, { terrain = "fertility" }]
```

- The player paints a growing zone with a crop's Grow tool. Every plant
  pass, each empty cell of it gets the crop's plan while the crop would
  grow there (its rate above 0), and a plan nobody has started is taken
  back once it wouldn't: nothing is sown into a frost.
- The plan is raised by its `by` work type, with that work's skill, and
  the crop starts as a seedling. Grown, it is marked for its first harvest
  that destroys it; harvested, the cell is sown again.
- A growing zone takes no items, so nothing is hauled to it and what falls
  there is hauled away. It keeps its crop across a save; one whose crop is
  gone after a change of mods is cleared, with a note.

### Changing a term

Terms are keyed by label, so a patch changes one term and leaves the rest.
Two mods patching the same term is reported as a conflict. A harsher winter
is one patch to the weather plugin's `mean`:

```toml
[[patch]]
target = "field/core:temperature"

[patch.set.ambient.mean]
of = [{ input = "year", curve = [
  [0.0, 4.0], [0.1333, 10.0], [0.375, 17.0], [0.625, 6.0], [0.875, -14.0], [1.0, 4.0],
] }]
```

To switch a term off, set its `scale = 0.0`. To add one, patch in a new label.

## Named contributions

Scripts don't overwrite outdoor values. They add a **named contribution** on
top of the terms:

<!-- not a sample -->
```lua
rim.push_ambient(field, key, value, hours, ease_hours)
rim.clear_ambient(field, key, ease_hours)
rim.explain(field)   -- { { label, value }, ... }: each term, then each push
```

- The contribution eases in from its current value over `ease_hours`, and
  after `hours` (nil: until cleared) it eases back out and disappears.
- Pushing the same key again replaces it. Different keys add up, so two mods
  never fight over a value.
- Each shows by name in `rim.explain` and in the weather panel's breakdown.

A volcanic winter, from a script:

```lua
rim.on("season_changed", function(e)
	if e.season == "autumn" and e.year == 2 then
		rim.push_ambient("core:temperature", "ashfall", -6, 24 * 20, 12)
		rim.push_ambient("core:cloud", "ashfall", 40, 24 * 20, 12)
		rim.message("Ash darkens the sky. It will be a hard winter.", "threat")
	end
end)
```

`rim.set_ambient(field, value)` still exists, but it *pins* the value,
overriding terms and every push, until `rim.set_ambient(field, nil)`. It's for
tests and tools, not mods.

## The calendar

Core's `[[calendar]]` sets a 60-day year of four seasons, starting on day 9 of
spring. Patch `calendar/core` for a longer year or different seasons.

<!-- not a sample -->
```lua
local d = rim.date()   -- { year, season, season_index, day, day_of_year, year_days, year_fraction }
rim.season()           -- "spring"
rim.seasons            -- { "spring", "summer", "autumn", "winter" }
rim.on("season_changed", function(e) end)   -- e.season, e.index, e.year
```

## Script data and events

Keep your plugin's state in the world, not in Luau locals. It's part of the
state hash, it will be saved, and the UI can read it:

<!-- not a sample -->
```lua
rim.set_data("state", { spells = 3, last = "rain" })   -- stored as "my_mod:state"
local s = rim.get_data("state")          -- a fresh copy, or nil
local f = rim.get_data("weather:forecast")  -- another mod's, read-only
```

Only plain data: nil, booleans, numbers, strings and tables of those. A bare
key is your mod's, and you can only write your own: `"my_mod:state"` works
too, `"weather:forecast"` is an error. That keeps each mod's data apart in a save, so
a migration or a removed mod's data is handled as a whole. In a UI script, `view.data("my_mod:state")` reads it.

Send your own events to any mod's `rim.on` handlers:

<!-- not a sample -->
```lua
rim.emit("my_mod:flood", { x = 10, y = 20 })
rim.on("my_mod:flood", function(e)
	rim.log("flood at " .. e.x .. ", " .. e.y)
end)
```

## The weather plugin's API

Weather is a queue: the current weather and the next three. Each is picked
with the world's random numbers when it joins the queue, so the forecast is
the future that will actually happen, unless something forces a change.
While it lasts, a weather type pushes a `"weather"` contribution to `cloud`,
`precipitation`, `wind`, `wind_dir`, `fog` and `temperature`.

Weather types are data: the plugin declares a `type` def kind, and its own
five live in [`defs/types.toml`](../../mods/weather/defs/types.toml). To add
one, list `weather` in your `mod.toml`'s `depends`, then write a
`[[weather.type]]`:

```toml
[[weather.type]]
id = "drizzle"                      # my_mod:drizzle
label = "drizzle"
cold_label = "flurries"             # shown when it falls below freezing
hours = [3, 8]                      # how long a spell lasts
blend = 1.5                         # hours to ease in from the previous weather
first_day = false                   # keep it off the first day
season = { autumn = 1.5, default = 0.5 }   # weight by season
follows = { "weather:cloudy" = 1.5 }       # likelier after overcast
set = { cloud = [70, 90], precipitation = [0.2, 0.8], wind = [1, 4], fog = 0, temperature = -1 }
message = "A fine drizzle sets in." # optional; cold_message for the cold label
```

A type's weight is `weight` × its season's entry × its `follows` entry for
the previous type. `set` takes a number, or `[min, max]` picked once per
spell. Change the shipped types with patches, like any def:
`target = "weather:type/weather:storm"`.

If a weight needs code, register the type from a script instead:
`require("@weather/scripts/weather").register({ ..., weight = function(ctx) ... end })`,
where `ctx` holds `season`, `year_fraction`, `previous` and `first_day`.

The weather module has these functions, and they take the plugin's own
types by bare name (`"storm"`) or qualified (`"weather:storm"`):

| Call | What it does |
|---|---|
| `weather.current()` | `{ id, label, start, ends, set }` for the weather now |
| `weather.forecast()` | The current weather and the three after it |
| `weather.force(id, hours)` | Replace the current weather; the rest of the forecast follows on later |
| `weather.weights(previous, tick)` | Each type's weight to follow `previous` at `tick` |
| `weather.types()` | Registered ids, in order |

It emits `weather:changed` (`from`, `to`, `label`) when the weather changes.
Its incidents (cold snap, heat wave, storm) go through core's storyteller like
any other. A storm incident is just:

```lua
local storyteller = require("@core/scripts/storyteller")
local weather = require("@weather/scripts/weather")

storyteller.register_incident({
	id = "my_mod:squall",
	kind = "neutral",
	min_day = 3,
	weight = 0.2,
	execute = function(ctx)
		weather.force("storm", 2)
	end,
})
```

Weather types register through Luau for now. When plugins can declare their
own def kinds (0208), they'll move to data.

## The sky

`daylight` holds the sky bodies; `light` is `daylight` dimmed by `cloud`. Sky
mods patch `daylight`, and weather only ever sets `cloud`, so a modded sky
keeps the storm dimming. Two suns:

```toml
[[patch]]
target = "field/core:daylight"

[patch.set.ambient.sun]
scale = 0.0

[patch.set.ambient.red_sun]
of = [{ input = "hour", curve = [[4, 0.0], [7, 40.0], [13, 40.0], [16, 0.0]] }]

[patch.set.ambient.white_sun]
of = [{ input = "hour", curve = [[9, 0.0], [12, 90.0], [20, 90.0], [23, 0.0]] }]
```

Light is a number in the simulation; colour is the renderer's business.
`[[sky]]` in core's defs sets the colours: `night` (the darkest the world
gets), `firelight`, `indoor_share` (daylight a roofed room gets through its
walls and door; each window adds its light `pass`) and `tint`s, each
a colour and a strength written as terms. Tints are keyed by label, so a mod
adds one without reshaping the others:

```toml
[[patch]]
target = "sky/core:core"

[patch.set.tint.green_moon]
color = "#7dffa0"
of = [{ input = "hour", curve = [[21.0, 0.0], [23.0, 0.35], [24.0, 0.35]] }]
```

The sun's brightness is the `daylight` field; where it is in the sky, for
the shadows it casts, is `sun` on `[[sky]]`: the hours it rises and sets,
how high it climbs, and the arc of azimuths it crosses (0° is east, 90°
south, the way the map's y grows). A sky without `sun` casts no sun shadows.

```toml
# A slower, lower winter sun.
[[patch]]
target = "sky/core:core"

[patch.set.sun]
rise = 8.0
set = 17.0
peak = 25.0
arc = [20.0, 160.0]
```

## What stops light

The renderer works out what stops light from what things are, so a mod's
walls, windows and trees need nothing extra (DESIGN.md §6e). A blocking
thing stands a storey tall; a door is a door; a blocking thing whose
`boundary` lets some of `light` through is a window. The one thing it can't
guess is how tall something stands, which sets how long its shadow is:

```toml
# Taller oaks, and longer shadows at dusk.
[[patch]]
target = "thing/core:tree_oak"

[patch.set]
height = 2.6   # cells; a blocking thing defaults to 1
```

A plant with a `height` is a canopy: it shades the ground, and some sky
gets through its leaves. Only blocking things and plants cast shadows; a
`height` on anything else does nothing yet.

## Firelight

Anything that emits `light` glows on screen: soft-shadowed by walls, let
out through windows, and flickering. It is baked once when a light or a
wall changes, so a hundred torches cost a frame what one does. A def says
how its flame moves with `glow`; a fire flickers unless it says otherwise:

```toml
# A lamp that burns steadily.
[[patch]]
target = "thing/core:campfire"

[patch.set.glow]
flicker = "steady"   # or "fire", the default
```

Firelight is `[[sky]]`'s `firelight` colour. Fires share three flicker
phases, picked by where they stand, so a fire never pulses in step with one
beside, above or below it (diagonal neighbours can); steady lights share a
fourth. A light's shadows are traced 23 cells at most.

## What the renderer draws

The renderer reads channels, never weather names, so any mod that sets the
channels gets the visuals:

| Channel | On screen |
|---|---|
| `light` | The whole world is multiplied by it (plus firelight and `[[sky]]` tints) |
| `precipitation` | Rain or snow, denser as it rises. Below 0°C it's snow, around freezing a mix. Never inside an enclosed room |
| `wind`, `wind_dir` | Rain slants and snow drifts; `wind_dir` is the direction it blows toward, 0 = east, 90 = south |
| `fog` | A drifting veil |
| heavy `precipitation` with `wind` above 10 m/s | Lightning: each flash throws hard shadows from a random direction |

## Tools

- `cargo run --release -p rim_sim --example year -- --seed 7`: a year of
  weather, one row per day, and how often each type came up.
- `cargo run --release -p rim_sim --example balance -- --seeds 40 --days 20 --start-day 38 --fire`:
  a bot plays from late autumn through winter in a heated hut. `--core` runs
  without the weather plugin.
- `cargo run --release -p rim_sim --example headless -- --days 3 --core`: the
  base game alone.
- In game, the weather readout in the top bar opens the forecast and the
  temperature's breakdown; `O` cycles overlays for fields that vary over the map.
- **F12** shows a Weather devtools panel: force any weather type for 12 hours,
  skip ahead an hour, six hours, a day or a season, and see the next 72 hours
  of cloud, rain, wind and fog from the forecast queue. Forcing is a command
  (`act.send("weather:force", ...)`, handled by the plugin's sim script with
  `rim.on`), the same way any mod's UI can ask its own scripts to act.
