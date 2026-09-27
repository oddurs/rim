---
id: fe54d733-82e3-4670-83c5-6fd49b3c5e2e
title: 'Mod options: typed [[option]] in mod.toml, sim options fixed per run and changed by a Command'
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

Players want to tune a mod, like wolf density or winter length, without
editing TOML. A modder with no settings fakes them with patches. But a
setting that changes the simulation is input (DESIGN.md §7). Kept in a
per-player config file, two machines would run different games from one
save, and a replay or bug report couldn't say which. DESIGN.md §10,
"Tension: do mods get settings?"

## What

A mod declares its options in `mod.toml`:

```toml
[[option]]
id = "stampedes"
name = "Stampedes"
description = "Whether herds can stampede through the colony."
kind = "bool"              # bool | int | choice
default = true
scope = "sim"              # sim | ui
mid_run = true             # may change after the colony starts; default false
```

An `int` option takes `min`, `max` and `step`; a `choice` takes `choices`.
Option ids are namespaced by their mod, like defs:
`wildlife_plus:stampedes`. Values are bools, integers or choice strings.
There are no floats, so every value is exact on every platform.

**Sim options**

- The player chooses them on the New colony screen (73751f4f, which
  this depends on). They are fixed when the colony starts.
- They are recorded in the epoch beside the mods, and move into `mods.lock`
  when 0152 lands.
- Scripts read them with `rim.option("stampedes")` for their own mod, or
  `rim.option("wildlife_plus:stampedes")` for a mod in `depends` or
  `optional`.
- Changing one mid-run is a `Command::SetOption { option, value }`. It is
  allowed only when `mid_run = true`, logged and replayed like any command,
  and fires `option_changed { option, value, was }`.

**UI options** live in the player's `settings.toml`, beside UI scale. The UI
VM reads them with `view.option`. They never reach the sim VM (DESIGN.md
§11).

**Defs don't read options.** A patch gated on an option
(`when = { option = ..., is = ... }`) waits for a real mod that needs it,
like load-time def generation (DESIGN.md §10). Conflict detection has to be
able to see it first.

**Tooling**

- `rim check` validates each declaration: the kind, a default in range or
  among the choices, unique ids.
- `rim test` sets options per world:
  `t.world({ options = { ["wildlife_plus:stampedes"] = false } })`.
- `--option wildlife_plus:stampedes=false` sets a sim option for a
  command-line new game.

**The first user.** `wildlife_plus` is the example plugin, so it ships the
first option: `stampedes`, read by `scripts/stampede.luau`. The mechanism
lands with a caller, and modders get a worked example.

**Later.** The mod manager (0119) edits UI options, and shows sim options
read-only once a colony has started. The seed code (47ec1fa0) should hash
sim options with the mod lock, so a shared code with different options
doesn't claim to be the same map.

## Acceptance criteria

- [ ] `[[option]]` parses with the three kinds; `rim check` reports a bad default, a duplicate id and an unknown kind, each naming the file and line (tests)
- [ ] The New colony screen shows each enabled mod's sim options with their defaults; Start records the chosen values in the epoch, and they survive save and load (test)
- [ ] `rim.option` returns the recorded value; reading an option of a mod outside `depends`/`optional` is an error naming both mods (tests)
- [ ] `Command::SetOption` changes a `mid_run` option and fires `option_changed`; a save with a change mid-log replays to the same state (determinism test); a non-`mid_run` option refuses the command
- [ ] UI options persist in `settings.toml` and aren't readable from the sim VM (test)
- [ ] `wildlife_plus:stampedes` ships, with a `rim test` for both values
- [ ] docs/modding documents `[[option]]`, `rim.option`, `view.option` and `option_changed`, and the types are generated from the API declaration
