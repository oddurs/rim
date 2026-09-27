---
id: e88e8ce5-5925-4f36-8e66-3eaf59e0f9f1
title: 'One open_cells for the tests: a ring walk that doesn''t repeat cells'
type: chore
status: backlog
created: 2026-09-27
updated: 2026-09-27
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

- [ ] No test file keeps its own copy, and the suite passes
