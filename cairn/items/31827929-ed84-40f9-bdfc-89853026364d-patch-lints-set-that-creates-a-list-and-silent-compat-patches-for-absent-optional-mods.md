---
id: 31827929-ed84-40f9-bdfc-89853026364d
title: 'Patch lints: set that creates a list, and silent compat patches for absent optional mods'
type: feature
status: backlog
milestone: sdk
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Two patch habits cost modders later:

- **`set` where `append` composes.** Iron gives timber's plank wall, crate,
  shelf and tool rack their nails and fittings with
  `set = { build = { cost = [...] } }`. The list didn't exist, so it works.
  But a second mod adding a cost the same way makes a conflict, where
  `append` would let both survive (docs/modding/patches.md, "Conflicts").
- **Correct compatibility patches fail `--strict`.** A patch whose target
  isn't loaded is a warning; the code's own comment says "patching an
  optional mod that isn't installed is normal". `rim check --strict` fails
  on warnings, so a mod with a correct compatibility patch fails CI whenever
  the other mod is absent.

## What

- A `set` that creates a list field the target doesn't have warns, and
  suggests `append`. Iron's four patches move to `append`.
- A patch whose target is in the namespace of a mod listed in the patching
  mod's `optional`, when that mod isn't installed, is skipped without a
  warning (noted in the debug log). A missing target in any other namespace
  still warns.

## Acceptance criteria

- [ ] A `set` creating a list warns with the suggestion; `append` doesn't (tests)
- [ ] Iron's four build-cost patches use `append`, and iron's tests pass
- [ ] A compatibility patch for an absent optional mod passes `rim check --strict` (test)
- [ ] A patch to a missing def in a non-optional namespace still warns (test)
- [ ] patches.md documents both
