# Replacing core

`core` is the base game, but to the engine it is a mod like any other
(DESIGN.md §5). A total conversion can leave it out and bring its own. A test
keeps that true: `crates/rim_sim/tests/no_core.rs` loads a game with no core,
runs a day, and round-trips a save. Its fixture, in
`crates/rim_sim/tests/fixtures/no_core/`, is the smallest game that loads.

## What a game must declare

Three defs, and nothing else is required:

| Def | Why |
|---|---|
| `[[terrain]]` | Map generation paints every cell with one. Give it a `gen` band covering the whole range (`elevation = [0.0, 1.0]`). |
| `[[creature]]` | The start's colonists are one. |
| `[[start]]` | Who the game begins with: its `creature`. |

The fixture's versions, in full:

```toml
[[terrain]]
id = "ground"
label = "ground"
color = "#6b8f4e"
path_cost = 100
gen = { elevation = [0.0, 1.0], priority = 1 }

[[creature]]
id = "settler"
label = "settler"
color = "#d8b890"
size = 0.36
speed = 13
max_hp = 100
melee_damage = 5
melee_cooldown = 90
intelligent = true
needs = []

[[start]]
id = "landing"
creature = "settler"
title = "the settler"
```

A game that misses one gets a load error that names it (`no [[terrain]]
defined`, `unknown creature`), not a crash.

## What you give up

Everything else core defines is optional to the engine: needs, a calendar
and sky, rooms, work styles, storage and names. The fixture's settler has
no needs, so it never goes hungry. With no `[[names]]` list, a colonist is
called by its creature's label.

What you lose is core's shared vocabulary ([Core's shared names](vocabulary.md)).
Plugins written for core refer to `core:` ids and fields such as
`core:temperature`. Beside a replacement they fail to load, or quietly do
nothing, unless the replacement declares the same names. A conversion that
wants those plugins to work declares them. One that doesn't, doesn't.
