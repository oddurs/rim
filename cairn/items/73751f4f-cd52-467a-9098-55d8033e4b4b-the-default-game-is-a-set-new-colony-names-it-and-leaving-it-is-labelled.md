---
id: 73751f4f-cd52-467a-9098-55d8033e4b4b
title: 'The default game is a set: New colony names it, and leaving it is labelled'
type: feature
status: backlog
milestone: platform
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: l
layer: client
area: modding
pillar:
- plugin-first
---

## Why

Today New colony starts a game at once. `title.luau` calls `act.new_colony`,
the client calls `save::open(mods, seed, Start::New)`, and `Sim::new` loads
every folder in `./mods`, including `wildlife_plus`, which is an example.
There is no screen between the click and tick 0. Nothing names the set of
mods the default install is balanced as (DESIGN.md §6, "Costs we accept"),
and nothing in a save says which game a colony is.

The first-party plugins are split for modders: Iron needs Core, Crafting,
Primitive and Timber. A new player shouldn't have to meet six switches. A
player who changes them should know they have left the balanced game.
DESIGN.md §10, "Tension: a mod list, or a game?"

Three backlog items assume a new-game screen that nobody owns: the premise
picker (e874ca2d), seed codes (47ec1fa0) and the mod manager (0119). This
item builds that screen.

## What

**A set file.** `sets/rim.toml`, beside `mods/`:

```toml
id = "rim"
name = "rim"
description = "The default game."
mods = ["core", "weather", "crafting", "primitive", "timber", "iron"]
```

- The list is what the default install is balanced as. `wildlife_plus` stays
  installed but isn't in the set, because it's an example plugin.
- A set must be closed under `depends`. Listing `iron` without `timber` is an
  error.
- A set is not a mod and has no content.
- When the lockfile lands (0152), a set gains versions and hashes and becomes
  a lockfile that ships with rim.

**The New colony screen** (the title layer, core's UI):

- **Game.** The set's name, "6 mods", and a label. The label reads "Tested
  together" when the enabled mods equal the set. Otherwise it reads
  "Untested combination" with the difference ("+Wildlife+ −Iron"). It never
  blocks and never warns.
- **World.** The seed and map size. Seed codes (47ec1fa0) replace the number
  when they land.
- **Premise.** A slot for the premise picker (e874ca2d). Until that lands,
  the slot shows the castaway.
- **Start**, focused, so New colony then Enter starts the default game.

**Change** lists every installed mod with a switch, its name, version and
description.

- Turning a mod on turns on what it needs.
- Turning a mod off turns off what needs it.
- A note names each mod that moved: "Turned off Timber and Iron, which need
  Primitive".
- Core is locked on, with one line saying why.
- Dev-only mods (`mods/devtools`, DESIGN.md §11a) aren't listed.
- Browsing the index, conflicts and the load-order column belong to 0119,
  which builds on this list. Sim options (fe54d733) add a row per enabled
  mod here.

**The game goes with the colony.**

- The epoch records the id of the set the colony started from.
- The title screen's save row and the top bar show the game line: "rim", or
  "rim +Wildlife+".
- A save from before this change shows no game line. If the epoch encoding
  needs a format bump for the new field, older saves read as having no set.

**Paths that don't change.** A game started from the command line
(`--seed`, no saving, the autotests, bench) keeps loading every installed
mod, because the autotests use `wildlife_plus` content. `--set <id>` starts
a command-line game from a set.

The client starts a game from a set with `Sim::with_mods(mods, seed, enabled)`,
which already exists.

## Acceptance criteria

- [ ] `sets/rim.toml` lists the six mods above; `rim check` reports a set that names a missing mod or isn't closed under `depends`, naming the file and the id (test)
- [ ] New colony opens the screen; Start with no changes loads exactly the set's mods (test on the loaded ids)
- [ ] Switches cascade both ways with a note naming each mod that moved, and Core can't be turned off (rim_ui test)
- [ ] The label reads "Tested together", or "Untested combination" with the +/− difference; it never disables Start
- [ ] The save records the set; the title row and the top bar show the game line; a save from before the change loads and shows none (test)
- [ ] `--seed`, the autotests and bench still load every installed mod; `--set rim` loads the set (test)
- [ ] DESIGN.md §10 "a mod list, or a game?" matches what shipped
