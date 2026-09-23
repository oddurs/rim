---
id: 123
title: 'Compatibility report: what a mod changes'
type: feature
status: backlog
milestone: sdk
depends_on:
- 149
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Before enabling a mod, show what it patches.

## Acceptance criteria

- [ ] Lists patched fields and incidents added
- [ ] Lists defs, def kinds, modules and events the mod adds
- [ ] `rim check --report` prints it; the index shows it on each mod's page
