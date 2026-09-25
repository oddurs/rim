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
