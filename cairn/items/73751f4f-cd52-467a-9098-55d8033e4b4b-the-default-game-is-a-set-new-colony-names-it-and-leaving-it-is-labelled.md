---
id: 73751f4f-cd52-467a-9098-55d8033e4b4b
title: 'The default game is a set: New colony names it, and leaving it is labelled'
type: feature
status: backlog
milestone: platform
depends_on:
- e4b96647-cb31-45ed-a3f3-ff22412465bb
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

Today New colony starts a game at once: `title.luau` calls
`act.new_colony`, the client calls `save::open(mods, seed, Start::New)`,
and `Sim::new` loads every folder in `./mods`, including `wildlife_plus`,
an example. Nothing names the mods the default install is balanced as
(DESIGN.md §6, "Costs we accept"), and nothing in a save says which game a
colony is.

The first-party plugins are split for modders: Iron needs Core, Crafting,
Primitive and Timber. A new player shouldn't meet six switches; a player who
changes them should know they left the balanced game. DESIGN.md §10,
"Tension: a mod list, or a game?"

The premise picker (e874ca2d), seed codes (47ec1fa0) and the mod manager
(0119) all assume a new-game screen nobody owns. This item builds it.

## What

**A set file**, `sets/rim.toml` beside `mods/`:

```toml
id = "rim"
name = "rim"
description = "The default game."
aliases = []
mods = ["core", "weather", "crafting", "primitive", "timber", "iron"]
[options]                  # colony options the set pins, once options land
```

- A set lists **sim-side** mods only (e4b96647).
  Client-side mods are the player's own and are never in a set.
- `wildlife_plus` stays installed but isn't in the set: it's an example.
- A set must be closed under `depends`; listing `iron` without `timber` is
  an error.
- `aliases` lists former ids, so a renamed set still names old saves.
- A set is not a mod: it has no content, and it isn't listed as one.
- When the lockfile lands (0152), a set gains versions and hashes and is a
  lockfile that ships with rim.

**The New colony screen** (the title layer, core's UI):

- **Game.** The set's name, "6 mods", and a label: "Tested together" when
  the enabled sim-side mods equal the set, otherwise "Untested combination"
  with the difference ("+Wildlife+ −Iron"). It never blocks and never warns.
  Client-side mods don't change it.
- **Contests.** Any contested slot the ladder settled by load order is listed
  with a Pick link (dbb92ebe).
- **World.** The seed and map size; seed codes (47ec1fa0) replace the number.
- **Premise.** A slot for the premise picker (e874ca2d), showing the castaway
  until it lands.
- **Start**, focused, so New colony then Enter starts the default game.

**Change** lists every installed mod: sim-side mods with switches, then the
player's client-side mods under "Yours, not part of the colony".

- Turning a sim mod on turns on what it needs; turning one off turns off
  what needs it; a note names each mod that moved ("Turned off Timber and
  Iron, which need Primitive").
- Core is locked on, with one line saying why.
- Dev-only mods (`mods/devtools`, DESIGN.md §11a) aren't listed.
- The index, conflicts in depth and the load-order column are 0119's, built
  on this list. Colony options (fe54d733) add a row per enabled mod.

**The game goes with the colony.** The epoch records the set id it started
from. The title screen's save row and the top bar show the game line ("rim",
or "rim +Wildlife+"). A save from before this change shows none; if the
epoch encoding needs a format bump, older saves read as having no set.

**Paths that don't change.** A command-line game (`--seed`, no saving, the
autotests, bench) keeps loading every installed mod, because the autotests
use `wildlife_plus` content. `--set <id>` starts one from a set.

The client starts a game from a set with `Sim::with_mods(mods, seed,
enabled)`, which exists.

## Acceptance criteria

- [ ] `sets/rim.toml` lists the six mods above; `rim check` reports a set that names a missing mod, a client-side mod, or isn't closed under `depends`, naming the file and the id (test)
- [ ] New colony opens the screen; Start with no changes loads exactly the set's mods (test on the loaded ids)
- [ ] Switches cascade both ways with a note naming each mod that moved, and Core can't be turned off (rim_ui test)
- [ ] The label reads "Tested together", or "Untested combination" with the +/− difference; a client-side mod never changes it; it never disables Start
- [ ] Contests settled by load order are listed with a Pick link
- [ ] The save records the set; the title row and the top bar show the game line; a save from before the change loads and shows none (test)
- [ ] `--seed`, the autotests and bench still load every installed mod; `--set rim` loads the set (test)
- [ ] DESIGN.md §10 "a mod list, or a game?" matches what shipped
