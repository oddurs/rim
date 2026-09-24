---
id: 17505800-ecc8-4b94-b0e6-7bf04a26f06e
title: 'Theme tokens: ui/theme.toml, mod patches and UI scale'
type: feature
status: done
milestone: interface
depends_on:
- 36d3ea49-e9dd-40fa-b6aa-9dd67eee7485
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: s
layer: client
area: ui
pillar:
- plugin-first
---

## Why

Every visual value comes from a token, so the look is plain and consistent and any mod can restyle the game without code.

## What

Tokens for spacing (4 px grid), text sizes, colours, radius and border live in `ui/theme.toml` in any mod. They merge in load order with the same patch semantics and conflict reporting as defs. Components reference tokens by name (`space.m`, `color.accent`). One UI scale multiplies every size and defaults to the display's DPI.

## Acceptance criteria

- [x] core ships `mods/core/ui/theme.toml`: translucent surfaces, 1 px hairlines, one accent, three text sizes
- [x] Another mod's theme.toml overrides tokens; two mods setting one token is reported as a conflict
- [x] Unknown token names are load errors naming the file and component
- [x] UI scale follows DPI by default and can be set explicitly; every size scales

## 2026-09-23

mods/core/ui/theme.toml: translucent surfaces, 1 px hairlines, one accent, four text sizes. Override and conflict tested (theme_tokens_can_be_overridden_and_conflicts_are_reported); unknown tokens are errors naming the component; UI scale test shows 2x doubles sizes. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
