---
id: d0115f9b-aff8-4046-a344-0f1bece5aa3c
title: 'mods/devtools: stepping and the session timeline'
type: feature
status: backlog
milestone: workbench
depends_on:
- 6ae1513a-d8d0-4744-90e5-386d6d1d1921
- cec3efdf-dc49-4fc0-a35e-5d649efe90e6
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: plugin
area: ui
---

## Problem

Time runs at 1×, 3× or 6×. There is no single tick, no "run to tick N" or "until this is true", and no way back to a moment earlier in the session. DESIGN.md §11a.

## Proposal

With `--dev`: `.` steps one tick, `dev.run(n)` and `dev.run_until(fn, max)` run on, `--pause-at <tick>` stops there on load; a fast-forward runs on a worker thread without freezing the window; the bottom bar shows this session's snapshots, and choosing one rebuilds that moment and branches from it (truncating the log, as §7a allows).

## Acceptance criteria

- [ ] One step advances exactly one tick (autotest)
- [ ] `run_until` stops on the first tick its condition holds (test)
- [ ] Branching from a snapshot and replaying the same commands reaches the same state hash (test)

## 2026-09-27

Ruled 2026-09-27 (DESIGN.md §11a): the workbench's tools are a first-party plugin, mods/devtools, loaded only with --dev. Build this item's UI there, on the dev API (6ae1513a-d8d0-4744-90e5-386d6d1d1921); anything the engine must add goes in that item.
