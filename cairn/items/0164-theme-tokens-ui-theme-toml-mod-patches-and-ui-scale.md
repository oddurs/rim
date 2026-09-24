---
id: 164
title: 'Theme tokens: ui/theme.toml, mod patches and UI scale'
type: feature
status: planned
milestone: interface
depends_on:
- 162
created: 2026-09-23
updated: 2026-09-23
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

- [ ] core ships `mods/core/ui/theme.toml`: translucent surfaces, 1 px hairlines, one accent, three text sizes
- [ ] Another mod's theme.toml overrides tokens; two mods setting one token is reported as a conflict
- [ ] Unknown token names are load errors naming the file and component
- [ ] UI scale follows DPI by default and can be set explicitly; every size scales
