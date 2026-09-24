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
| `mods/weather` | Seasons, weather types, the forecast, cold snaps and storms, the weather readout |

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
| `{ noise = "gusts", hours = 2 }` | Smooth noise from -1 to 1, changing over about `hours` game hours |
| `4.5` | A constant |

- A **curve** is up to 16 `[x, y]` points with increasing `x`. Between points
  it's a straight line; beyond the ends it holds the end value.
- Fields are evaluated in dependency order: `temperature` can read `cloud`.
  A cycle is an error when the game loads, and so is a typo in a field or
  input name. The error names your mod, the field and the term.
- Evaluation is fixed point, so every machine gets exactly the same numbers.
  Don't compute simulation values in Luau with `math.sin` or `math.cos`: they
  can differ in the last bit between platforms.

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
rim.set_data("my_mod:state", { spells = 3, last = "rain" })
local s = rim.get_data("my_mod:state")   -- a fresh copy, or nil
```

Only plain data: nil, booleans, numbers, strings and tables of those. Use your
mod id as a prefix. In a UI script, `view.data("my_mod:state")` reads it.

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

Add a weather type. List `weather` in your `mod.toml`'s `depends` (or
`optional`, if your mod works without it) and require its module:

```lua
local weather = require("@weather/scripts/weather")

weather.register({
	id = "my_mod:drizzle",
	label = "drizzle",
	cold_label = "flurries", -- shown when it falls below freezing
	hours = { 3, 8 }, -- how long a spell lasts
	blend = 1.5, -- hours to ease in from the previous weather
	weight = function(ctx)
		-- ctx: season, year_fraction, previous, first_day
		if ctx.first_day then
			return 0
		end
		return if ctx.season == "autumn" then 1.5 else 0.5
	end,
	set = {
		-- a number, or { min, max } picked once per spell
		cloud = { 70, 90 },
		precipitation = { 0.2, 0.8 },
		wind = { 1, 4 },
		fog = 0,
		temperature = -1,
	},
	message = "A fine drizzle sets in.", -- optional; a string or function(label)
})
```

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
gets), `firelight`, `indoor_share` (daylight through windows) and `tint`s, each
a colour and a strength written as terms. Tints are keyed by label, so a mod
adds one without reshaping the others:

```toml
[[patch]]
target = "sky/core:core"

[patch.set.tint.green_moon]
color = "#7dffa0"
of = [{ input = "hour", curve = [[21.0, 0.0], [23.0, 0.35], [24.0, 0.35]] }]
```

## What the renderer draws

The renderer reads channels, never weather names, so any mod that sets the
channels gets the visuals:

| Channel | On screen |
|---|---|
| `light` | The whole world is multiplied by it (plus firelight and `[[sky]]` tints) |
| `precipitation` | Rain or snow, denser as it rises. Below 0°C it's snow, around freezing a mix. Never inside an enclosed room |
| `wind`, `wind_dir` | Rain slants and snow drifts; `wind_dir` is the direction it blows toward, 0 = east, 90 = south |
| `fog` | A drifting veil |
| heavy `precipitation` with `wind` above 10 m/s | Lightning |

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
