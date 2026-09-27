---
id: d80bce19-fb21-4a64-9ff9-07f47d85bf2b
title: 'Mod crater: engine CI runs indexed mods'' tests before a change lands'
type: feature
status: backlog
milestone: platform
depends_on:
- acaa16f4-f165-4ddf-a5aa-da860100abcb
- b7f5cde1-a8c4-4d7d-b93d-bdf0ba0d523b
created: 2026-09-23
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: tooling
area: tests
pillar:
- plugin-first
---

## Why

Find API breaks ourselves before modders do (DESIGN.md §10). Named after Rust's crater.

## Acceptance criteria

- [ ] Job that fetches every indexed mod and runs `rim check` and `rim test` against a candidate engine
- [ ] Report diffs pass/fail against the last release, per mod
- [ ] Runs on release candidates and on demand for API-changing PRs

## 2026-09-27

Its results are published to the index as compat/results/<id>.toml (f2a251f6), and the game marks mods known broken on its engine. Data-only mods survive API minors through shims (f044951c), so the crater's failures are mostly script mods.
