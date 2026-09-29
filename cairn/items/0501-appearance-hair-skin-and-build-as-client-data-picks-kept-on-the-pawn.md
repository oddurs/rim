---
id: 501
uid: 8469a7ff-bee8-4237-9fc6-a424aea6c362
title: 'Appearance: hair, skin and build as client data, picks kept on the pawn'
type: feature
status: backlog
milestone: people
depends_on:
- 480
- 518
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
pillar:
- plugin-first
- performance
effort: m
layer: engine
area: render
---

## Why

Every colonist is the same figure until people have looks of their own. From
above there's no face, so people are told apart by hair, skin, build and
clothes. DESIGN.md §6h rules that looks are client data, that the sim stores
only a person's picks as ids it never reads, and that the picks are rolled
per pawn so a new hair style never reshuffles anyone's traits (§7b).

## What

**Features and palettes are client data** (`ui/features.toml`):

```toml
[[feature]]
id = "bun"
slot = "hair"
label = "Bun"
weight = 1
layers = [
  { draw = "disc", y = 0.015, r = 0.105, color = "@hair", line = "hair" },
  { draw = "disc", y = 0.13, r = 0.045, color = "@hair", line = "hair" },
]

[[palette]]
id = "skin"
colors = ["#f3d5b8", "#e0b894", "#c9936a", "#a06a44", "#7a4a2c", "#553321"]
```

- Core ships hair styles (cropped, long, bun, tail, braids, curls, shaved,
  bald), palettes for skin and hair, and builds (slight 0.9, average 1,
  broad 1.12, across the shoulders).
- A body lists the slots and channels its creature rolls
  (`appearance = ["hair", "@hair", "@skin", "@build"]`).

**Picks live on the pawn** (sim state, saved):
`appearance = { seed, picks = { slot or channel → id or index } }`.

- The sim never reads them. They're rolled once at spawn with a
  counter-based draw keyed by the pawn (c2579dbc), so adding a feature
  changes looks and nothing else.
- A pick naming a feature the client lacks falls back to one rolled from the
  seed among the features it has, with a warning. A co-op peer without your
  hair mod sees a fallback.
- The torso takes the skin colour while the pawn wears nothing: the castaway
  starts naked (DESIGN.md §3).

## Acceptance criteria

- [ ] `[[feature]]` and `[[palette]]` load from `ui/`, validated like looks (tests)
- [ ] New pawns carry picks; the save round-trips them, and the determinism test passes unchanged
- [ ] Adding a feature to a mod changes no roll outside appearance: the same seed gives the same traits, skills and map (test)
- [ ] An unknown pick falls back and warns (test)
- [ ] The autotest's colony shows varied colonists at the detail zoom
- [ ] docs/modding/bodies.md documents features, palettes and picks
