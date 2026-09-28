---
id: fe54d733-82e3-4670-83c5-6fd49b3c5e2e
title: 'Mod options: colony, live and player scopes, and patches gated on colony options'
type: feature
status: backlog
milestone: platform
depends_on:
- 73751f4f-cd52-467a-9098-55d8033e4b4b
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
- determinism
---

## Why

Players want to tune a mod (wolf density, lean years) without editing TOML,
and a modder with no options fakes them with patches or ships two mods. But
anything that changes the simulation is input (DESIGN.md §7). Kept in a
per-player file, as RimWorld's ModSettings are, two machines run different
games from one save. DESIGN.md §10, "Tension: do mods get settings?"

Factorio's `startup` settings are read while prototypes load, and Content
Patcher's config values gate patches with `When`. Both stay predictable
because the values are fixed before the data loads. That is the model here.

## What

A mod declares options in `mod.toml`:

```toml
[[option]]
id = "lean_years"
name = "Lean years"
description = "Berries regrow slowly and herds are thinner."
kind = "bool"              # bool | int | choice, never float
default = false
scope = "colony"           # colony | live | player
```

An `int` takes `min`, `max` and `step`; a `choice` takes `choices`. Ids are
namespaced like defs (`wildlife_plus:lean_years`). The engine types and
defaults every value, so `rim.option` can never return something the
declaration didn't allow.

**Three scopes**

- **colony**: chosen on New colony (73751f4f) and recorded in the epoch
  (in `mods.lock` when 0152 lands). A colony option may **gate patches** and
  is read by scripts. Changing one later is an epoch boundary, like a mod
  update: allowed, and honest that the unreplayed tail goes.
- **live**: read by scripts only. Changed by `Command::SetOption { option,
  value }`, logged and replayed, firing `option_changed { option, value,
  was }`. In co-op the host decides.
- **player**: the UI VM only, stored in the player's `settings.toml`, read
  with `view.option`. The sim VM can never read it; that is a wall.

**Gated patches.** A `[[patch]]` may carry `when`:

```toml
[[patch]]
target = "thing/core:berry_bush"
when = { option = "lean_years", is = true }
[[patch.edit]]
list = "harvest"
match = { designation = "harvest" }
set = { regrow_days = 4.0 }
```

- `when` names a colony option of this mod or of a mod in its `depends` or
  `optional`, with `is` (equals), or `in` (one of) for choices.
- The loader resolves every gate before merging, then runs conflict
  detection on the patches that apply. Conflict reports see exactly what
  loaded.
- `when` on a live or player option is a load error.

**Tooling**

- `rim check` validates each declaration (kind, a default in range or among
  the choices, unique ids) and each `when`.
- `rim test` sets options per world:
  `t.world({ options = { ["wildlife_plus:lean_years"] = true } })`.
- `--option wildlife_plus:lean_years=true` sets a colony option for a
  command-line new game.
- A set (73751f4f) may pin colony option values.

**The first user.** `wildlife_plus` is the example plugin, so it ships
`lean_years` (a gated patch on berry regrowth) and a live option,
`stampedes`, read by `scripts/stampede.luau`.

**Later.** The mod manager (0119) edits player options and shows colony
options read-only once a colony has started. Seed codes (47ec1fa0) hash
colony option values with the mod lock.

## Acceptance criteria

- [ ] `[[option]]` parses with the three kinds and three scopes; `rim check` reports a bad default, a duplicate id, an unknown kind and a `when` on a non-colony option, each naming the file and line (tests)
- [ ] The New colony screen shows each enabled mod's colony options; Start records the values in the epoch, and they survive save and load (test)
- [ ] A gated patch applies only when its option matches, and conflict detection reports only patches that applied (tests for both values)
- [ ] Changing a colony option on an existing colony starts a new epoch and says so (test)
- [ ] `rim.option` returns the recorded, typed value; reading an option of a mod outside `depends`/`optional` is an error naming both mods (tests)
- [ ] `Command::SetOption` changes a live option and fires `option_changed`; a save with the change mid-log replays to the same state (determinism test); a colony or player option refuses the command
- [ ] Player options persist in `settings.toml` and aren't readable from the sim VM (test)
- [ ] `wildlife_plus` ships `lean_years` and `stampedes`, with `rim test` coverage of both values of each
- [ ] docs/modding documents `[[option]]`, the scopes, `when`, `rim.option`, `view.option` and `option_changed`; the types are generated from the API declaration
