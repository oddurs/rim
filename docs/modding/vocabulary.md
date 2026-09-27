# Core's shared names

When two plugins must agree on a name, core defines it (DESIGN.md §5, rule
2). A plugin that uses these names works with every other plugin that does,
without either depending on the other. They are a contract: core keeps them
stable, and changing one is an API change.

Refer to them qualified (`core:gather`) from your own defs. Tags aren't
namespaced: they are plain strings that defs match.

## Fields

`temperature`, `daylight`, `light`, `cloud`, `precipitation`, `wind`,
`wind_dir` and `fog`. See [Weather](weather.md).

## Designations

| Id | Marks a thing to be |
|---|---|
| `chop` | Felled: trees |
| `mine` | Quarried: rock |
| `harvest` | Picked: berry bushes, crops |
| `gather` | Taken from without taking the thing: branches off a tree, stones off the ground |
| `hunt` | Hunted and butchered: wild creatures |
| `deconstruct` | Taken down: what the colony built |

A thing has one harvest per designation (`harvest = [...]`), so an oak can
be gathered for branches and chopped for wood. Core's own things have no
`gather` harvest: it exists so plugins agree on one.

## Tool tags

What a tool does. A harvest or a recipe that `requires` a tag can be worked
only by a pawn holding a tool that has it. Core defines no tools and
requires none, so with no plugins every job is bare-handed.

| Tag | Means | For example |
|---|---|---|
| `cutting` | A sharp edge | a flint flake, a knife |
| `chopping` | Fells and splits wood | a hand axe, an axe |
| `pounding` | Strikes stone | a hammerstone, a maul, a pick |
| `digging` | Breaks and lifts earth | a digging stick, a spade |
| `piercing` | A point that pierces | a spear, an awl |

A tool is an item with a `tool` block, and a harvest names what it needs:

```toml
[[thing]]
id = "hand_axe"
label = "hand axe"
category = "item"
hp = 60
tool = { tags = ["chopping", "cutting"], speed = 0.6, wear = 3 }

[[patch]]
target = "thing/core:tree_oak"
set = { harvest = { requires = ["chopping"] } }
```

A pawn holds one tool at a time. Given work that needs one it doesn't
hold, it fetches the nearest free tool that covers it and puts down what it
had. `speed` is work per tick against bare hands' 1, times the `tool_speed`
factor of what the tool is made of. `wear` is the hp a finished job costs
it, and at none left it breaks. Work no tool in the colony can do waits,
and a selected thing says so: "Needs a chopping tool."

## Thing tags

| Tag | Means |
|---|---|
| `table` | Something to eat at: a chair's spot with `beside = "table"` faces it |
| `bulky` | Too big for a basket, a crate or a shelf: wood, stone blocks, branches. Bulk stores and the ground take it |
| `bed` | Somewhere to sleep, as a room role counts it (core's bed, primitive's grass pallet) |
| `fire` | A hearth: core's campfire and stove |
| `seat` | Something to sit on: core's chair |

## Item categories

The tree stores, stockpiles and bills filter by (DESIGN.md §4f). Core's
top level is `food`, `materials`, `tools` and `other`; add your own shelf
under one with `parent`. An item is in a category by tag, by id, or by what
it is, so a new food or tool lands in the right place without a patch.

```toml
[[item_category]]
id = "metal"
label = "Metal"
parent = "core:materials"
order = 50
tags = ["metal"]          # items with any of these tags
things = ["ingot"]        # and these, by id
with = []                 # items with a "food", "tool" or "stuff" block
stuff = []                # items that are material of these stuff categories
```

An item can be in several categories. One category may say `rest = true`
(core's `other`): items no category claims land there, so every item can be
filtered. Scripts read the tree as `rim.item_categories`.

## Room roles

What a room is for is data (DESIGN.md §6c). A `[[room_role]]` names the
tags a room must hold, and in load order the first role a room meets is
its role. Things count for the room they stand in; a thing that blocks,
like a workbench, counts for the first room beside it.

| Role | Needs | From |
|---|---|---|
| `core:dormitory` | `bed = 2` | core |
| `core:home` | `bed = 1, fire = 1` | core |
| `core:bedroom` | `bed = 1` | core |
| `core:hall` | `table = 1, seat = 1` | core |
| `crafting:workshop` | `crafting:hand = 1` | crafting |

```toml
[[room_role]]
id = "reading_room"
label = "Reading room"
needs = { seat = 2 }
min_cells = 9        # smallest room that can take it (default 0)
enclosed = true      # only enclosed rooms (the default)
```

A role with no `needs` would fit every room, so it doesn't load. Scripts
read a room's role with `rim.room_at(x, y).role` (its qualified id) and
`.role_label`. Roles are worked out again only when rooms rebuild or a
thing carrying a counted tag is built or taken away.

## Storage

Stores are stockpile zones and containers (DESIGN.md §4f). Core names the
levels they sort by: a stack only ever moves to a store at a higher one.

```toml
[[store_priority]]          # core's; patch it to change the scale
id = "core"
labels = ["Low", "Normal", "Preferred", "Important", "Critical"]
default = 1                 # where a new store starts, an index from 0
```

A container is any building with a `store` block. Its contents are stacks
held in its slots, off the ground: one cell holds as many stacks as it has
slots.

```toml
[[thing]]
id = "crate"
label = "crate"
category = "building"
blocks = true
build = { stuff = { category = "structural", count = 6 }, work = 400 }

[thing.store]
slots = 4                   # stacks it holds
stack_scale = 1             # each slot holds this many times a stack
accepts = { not_tags = ["bulky"] }   # what it can ever take (below)
shelter = false             # contents out of the weather
display = "fill"            # "fill", "items" or "none"
look_stages = 4             # for "fill": steps from empty to full
```

`accepts` takes `tags`, `things` and `categories` (any of them lets an item
in; none at all means any item) and `not_tags` (never these). The player's
filter narrows it and can never widen it. Colonists haul into a container,
take from it for building, crafting and meals, and fetch tools from it,
standing beside it. Torn down or destroyed, it sets everything it held on
the ground nearby.

Scripts: `rim.store(id)` reads a container's level, slots and contents;
`rim.store_put(id, { thing, count, made_of })` puts things in (a caravan
unloading) and returns what didn't fit; `rim.store_take(id, slot, count)`
takes from a slot; `rim.stock(...)` counts what the colony has, stored or
loose.
