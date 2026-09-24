---
id: 78
title: 'Stat pipeline: base values plus registered modifiers'
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 213
created: 2026-09-22
updated: 2026-09-23
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

## 2026-09-23

The Building sprint (0210) delivers the first concrete slice as 0213: base times material factor for built things, with factor names opaque to the engine. This item generalises that to wealth, threat and move speed.
