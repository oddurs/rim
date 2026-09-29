---
id: 545
uid: e88e8ce5-5925-4f36-8e66-3eaf59e0f9f1
title: 'One open_cells for the tests: a ring walk that doesn''t repeat cells'
type: chore
status: done
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: s
layer: tooling
area: tests
---

## Why

Eight test files copy a helper that finds open cells ring by ring, but each ring walks the whole square, so past the first nine cells it hands out the same cells again. stuff.rs tripped over it when bridges added two buildables (fixed there during ba8253df). The others take few enough cells today.

## What

- A fixed `open_cells` in tests/common: each ring's own cells, nearest first.
- deconstruct, spots, factors, feels_like, boundary, furniture, snapshot and floors use it.

## Acceptance criteria

- [x] No test file keeps its own copy, and the suite passes

## 2026-09-28

Six copies were left, not eight: feels_like, boundary and snapshot had dropped theirs. stuff.rs's fixed ring walk is now common::open_cells. The client's draw.rs has its own ring walk with a different signature, which already walks only each ring's edge, so it stays.
