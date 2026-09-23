---
id: 155
title: 'Mod crater: engine CI runs indexed mods'' tests before a change lands'
type: feature
status: backlog
milestone: platform
depends_on:
- 146
- 153
created: 2026-09-23
updated: 2026-09-23
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
