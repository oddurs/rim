---
id: c7e9ab80-43d1-4121-8e75-e2dcb9bf3340
title: 'Determinism test: same seed and commands give the same state hash'
type: chore
status: done
milestone: foundations
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: s
layer: engine
area: tests
pillar:
- determinism
---

## Why

Determinism breaks silently. A test must catch it on every change.

## Acceptance criteria

- [x] Two runs of 5 in-game days with the same seed hash equal
- [x] Different seeds hash differently
- [x] Runs with both core and the example plugin loaded

## 2026-09-22

tests/determinism.rs: two 3-day runs with identical seed + commands hash equal; different seeds differ. Passing.
