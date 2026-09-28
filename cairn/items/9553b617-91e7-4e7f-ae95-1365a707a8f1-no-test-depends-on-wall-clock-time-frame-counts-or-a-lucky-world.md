---
id: 9553b617-91e7-4e7f-ae95-1365a707a8f1
title: No test depends on wall-clock time, frame counts or a lucky world
type: chore
status: backlog
milestone: bare-metal
assignee: lucky-harbor
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Why

Main went red ten times in two days, mostly from tests tied to real time or to how a world happened to turn out: the founder dying in a storm, the ruler's alert phase, box-select counts, the fade tolerance.

## What

- Audit rim_client's autotest and rim_sim's tests. Any check that waits on frames or real time, or assumes who is where, either pins its world (weather, spawns, hp) or counts what it needs.
- The autotest runs twice on different seeds in the nightly, and a check that fails on one seed only is a flake to fix.

## Acceptance criteria

- [ ] Every autotest section pins or counts what it asserts, audited and listed in a note
- [ ] The nightly autotest passes on three seeds
- [ ] No test failure in 20 consecutive queue-lane runs that the change under test didn't cause
