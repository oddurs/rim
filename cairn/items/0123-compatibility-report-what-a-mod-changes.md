---
id: 9a4e2f10-c084-44a4-9f43-5a66c5f39b33
title: 'Compatibility report: what a mod changes'
type: feature
status: backlog
milestone: sdk
depends_on:
- be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
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
