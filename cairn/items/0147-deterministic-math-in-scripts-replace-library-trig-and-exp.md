---
id: 147
title: 'Deterministic math in scripts: replace library trig and exp'
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 157
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: breaking
effort: s
layer: engine
area: scripting
pillar:
- determinism
---

## Why

Luau's `math.sin`, `math.cos`, `math.exp`, `math.log` and `math.pow` call the platform C library, which can differ in the last bit between macOS, Windows and Linux. That is enough to desync co-op or break a replay once a mod uses them (DESIGN.md §10).

## Acceptance criteria

- [ ] Those functions replaced in the sandbox by deterministic implementations with documented accuracy
- [ ] Test vectors that must match exactly on all CI platforms

## 2026-09-23

Less pressing after the Weather sprint: 20_climate.luau (the one script that needed smooth curves) becomes data in 0183, evaluated in fixed point by the engine. Scripts still need a safe alternative for anything else periodic.
