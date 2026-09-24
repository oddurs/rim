---
id: 63d2f10a-ec0f-499b-8af8-2905bb64c6d2
title: API deprecations warn with the replacement and the removal version
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: additive
effort: s
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Break freely before 1.0, but never silently (DESIGN.md §10). A modder should learn about a removal from a warning, not from a crash report.

## Acceptance criteria

- [ ] A deprecated Luau call or def field keeps working for one minor version
- [ ] Each use logs once per mod: what to use instead and which api version removes it
- [ ] `rim check` lists deprecated uses
