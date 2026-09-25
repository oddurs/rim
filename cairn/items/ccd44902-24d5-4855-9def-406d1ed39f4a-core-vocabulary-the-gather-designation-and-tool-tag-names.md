---
id: ccd44902-24d5-4855-9def-406d1ed39f4a
title: 'Core vocabulary: the gather designation and tool tag names'
type: content
status: done
milestone: stone-age
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p0
api: additive
effort: s
layer: core
area: modding
---

## Why

DESIGN.md §5 rule 2: when two plugins must agree on a name, core defines it. The stone age (e3846c47), later tiers, tailoring, farming and weapons all need to agree on what "a chopping tool" and "gathering" mean, without depending on each other (§6: plugins connect through core's names).

## What

- **A `gather` designation** in `mods/core/defs/colony.toml`, beside chop, mine and harvest. It has a label and a colour and targets things. Its toolbar button comes for free, because the toolbar is generated from designation defs.
- **Tool tag names, documented** in `docs/modding` as the shared vocabulary: `cutting`, `chopping`, `pounding`, `digging`, `piercing`. Core defines no tools. The names are a contract, like `temperature`.
- **Core's own defs don't gate anything.** With no plugins, `tree_oak` and `granite` stay bare-hand work, so the game is complete but shallow (§5, and the CI smoke test).

## Acceptance criteria

- [x] `core:gather` exists and shows in the toolbar
- [x] The tool tag names are documented as core's vocabulary
- [x] Core alone plays unchanged: the smoke test passes

## 2026-09-25

The toolbar now shows only designations something loaded can be marked for (rim_client markable): with core alone, gather would be a button that does nothing.
