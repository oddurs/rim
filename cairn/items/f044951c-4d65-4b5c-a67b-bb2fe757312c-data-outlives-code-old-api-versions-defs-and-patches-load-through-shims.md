---
id: f044951c-4d65-4b5c-a67b-bb2fe757312c
title: 'Data outlives code: old API versions'' defs and patches load through shims'
type: feature
status: backlog
milestone: sdk
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Before 1.0, `check_api` loads a mod only if its `api` equals the engine's
minor. Going from 0.6 to 0.7 refuses every mod, including a data-only mod
the change never touched, and nothing tells the player which mods would
have worked.

Factorio's single `factorio_version` forces a re-release of every mod each
major, and old mods die in bulk. Content Patcher's `Format` field lets the
runtime keep reading old packs through shims. rim can do the same cheaply
for everything that is data. DESIGN.md §10, "Tension: a moving API or mods
that rot?"

## What

- The engine keeps a **data shim** per past minor: a function that rewrites
  a `mod.toml`, def, patch or theme table written for 0.N into 0.N+1's shape
  (renamed fields, moved tables). Shims chain.
- `DATA_API_OLDEST` names the oldest minor the shims reach.
- A mod with no `scripts/` and no `ui/` scripts, targeting a minor from
  `DATA_API_OLDEST` on, loads through the shims, with a note naming the
  version.
- A mod with scripts still needs the current minor; its path is the
  deprecation window (63d2f10a).
- A PR that changes the data format lands with its shim and a fixture mod
  written for the old minor. A test loads every fixture through the chain.

## Acceptance criteria

- [ ] The shim chain exists with an identity shim for 0.6, and a fixture data-only mod per supported minor loads (test)
- [ ] A script mod targeting an older minor is refused with the reason and the version it needs (test)
- [ ] A data-only mod older than `DATA_API_OLDEST` is refused, naming the oldest supported minor (test)
- [ ] A test fails when the data API minor rises without a fixture for the previous one
- [ ] DESIGN.md §10 and docs/modding describe what survives an engine update
