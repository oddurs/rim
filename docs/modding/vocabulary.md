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

## Thing tags

| Tag | Means |
|---|---|
| `table` | Something to eat at: a chair's spot with `beside = "table"` faces it |
