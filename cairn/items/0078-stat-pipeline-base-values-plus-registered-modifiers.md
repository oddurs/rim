---
id: 78
title: 'Stat pipeline: base values plus registered modifiers'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: l
layer: engine
area: modding
pillar:
- plugin-first
- performance
---

## Why

Mods add to a stat instead of overriding a function. Wealth, threat and move speed become stats.

## Acceptance criteria

- [ ] Stats declared in defs
- [ ] Modifiers from defs and scripts
- [ ] Cached with dirty tracking
