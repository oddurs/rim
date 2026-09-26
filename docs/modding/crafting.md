# Crafting: recipes, stations and bills

Making things is a plugin: [`mods/crafting`](../../mods/crafting/). The
engine knows only [work orders](scripting.md#work-orders): "bring these, then
work here". The plugin owns everything above them: what can be made, where,
and how many. Your mod adds recipes and stations as data, and never touches
the plugin's code.

The design is in [DESIGN.md §4e](../../DESIGN.md).

## Recipes

A recipe is a `[[crafting.recipe]]` def in your mod. Depend on `crafting` in
your `mod.toml`.

```toml
[[thing]]
id = "shard"
label = "shard"
color = "#556677"
category = "item"
look.layers = [{ draw = "fill", x = 0.3, y = 0.3, w = 0.4, h = 0.4 }]
stack_limit = 25
tags = ["sharp_stone"]
stuff = { categories = ["lithic"], factors = { hp = 0.6 } }

[[thing]]
id = "knife"
label = "knife"
color = "#99aabb"
category = "item"
look.layers = [{ draw = "fill", x = 0.2, y = 0.45, w = 0.6, h = 0.1 }]
hp = 60
tool = { tags = ["cutting"] }

[[crafting.recipe]]
id = "knife"
label = "stone knife"
station = "crafting:hand"         # a tag: any thing carrying it makes this
inputs = [
    { tag = "sharp_stone", count = 1 },   # anything with the tag
    { thing = "core:wood", count = 1 },   # or one thing
]
outputs = [{ thing = "knife" }]
work = 300                        # ticks at bare hands' pace
```

| Field | Means |
|---|---|
| `label` | What the bill and the work order are called |
| `station` | A tag. Any thing whose `tags` include it can make this |
| `inputs` | `{ thing, count }` or `{ tag, count }`, brought in this order |
| `outputs` | `{ thing, count }`, dropped at the station when the work is done |
| `work` | Ticks at bare hands' pace, divided by the station's speed |
| `requires` | Tool tags (default none): the worker holds a tool with every one |
| `work_type` | The Work Board column (default `crafting:craft`) |
| `stuff` | Whether outputs are made of the first input that is a material (default true) |

Bare ids are your own mod's (`knife` is `your_mod:knife`); name another
mod's things with their prefix. A recipe that names a thing that doesn't
exist stops the game from loading, and says which.

The knife above takes the first material that went in: a knife knapped from
a flint shard is a flint knife, with flint's hp. Order `inputs` so the one
the output is made of comes first, or set `stuff = false`.

## Stations

A station is anything built that carries a station tag. The plugin ships
two, with two tags:

| Thing | Tags | |
|---|---|---|
| `crafting:spot` | `crafting:hand` | A patch of ground marked for handwork. Free: 20 work, nothing brought |
| `crafting:workbench` | `crafting:hand`, `crafting:bench` | Structural stuff. Works at 1.5× |

So `station = "crafting:hand"` is handwork, which can be done at either one,
and `crafting:bench` needs the bench. Your own station is a thing with your
own tag (and `crafting:hand`, if handwork can be done there too). A
`[[crafting.station]]` sets its speed:

```toml
[[thing]]
id = "anvil"
label = "anvil"
color = "#3a3a44"
category = "building"
look.layers = [{ draw = "fill", x = 0.15, y = 0.35, w = 0.7, h = 0.4 }]
blocks = true
tags = ["smithing"]
build = { menu = "production", work = 600, cost = [{ thing = "core:wood", count = 10 }] }

[[crafting.station]]
id = "anvil"
thing = "anvil"
speed = 1.25
```

The crafting spot is free because its build says so: `free = true`. A build
needs a `cost`, a `stuff`, or `free = true`, so a forgotten cost is an error
rather than a free building.

## Bills

Selecting a station shows its bills in the inspector. A bill is a recipe and
how many:

- **Do N more**: make N, then the bill is finished.
- **Until N**: make while there are fewer than N of the first output lying
  about. Use one and it makes another.
- **Forever.**

Bills run top first. The station takes the first bill that can run, and
posts it as a work order; the colonists bring the inputs and work it, and
the outputs appear at the station. A bill that can't run says why in the
panel: "no sharp_stone", "1 of 2 wood", "have 5". A tool the order
needs and no colonist has shows on the station itself: "Needs a pounding
tool." A recipe the engine refuses, such as one that
requires a tool tag no tool has, is paused with the reason. Pausing or
removing the bill being made takes its order down, and
what was brought is put back on the ground.

The bills are script data (`crafting:bills`), saved and hashed with the
world. The panel changes them only through `act.send`, so they're commands
like any other player input, and a replay makes the same things.

## For scripts

A mod that depends on `crafting` can manage bills from its own scripts:

```lua
local bills = require("@crafting/scripts/bills")

local id = bills.add(station, "my_mod:knife")      -- nil if it can't make it there
bills.set(station, id, { mode = "until", target = 5 })
bills.move(station, id, -1)                         -- up one
bills.remove(station, id)
bills.of(station)                                   -- its bills, top first
bills.recipes_at("crafting:workbench")              -- recipe ids, in load order
```

The panel's buttons send `crafting:add_bill`, `crafting:set_bill`,
`crafting:move_bill` and `crafting:remove_bill`, which call these.

A recipe whose station tag or input tag no thing carries loads, since a
later mod may add it, and the console says so.
