---
id: 63
title: Keep unknown mod data when a mod is removed
type: feature
status: backlog
milestone: persistence
depends_on:
- 61
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- plugin-first
---

## Why

Removing a mod must not corrupt a save; re-adding it should restore its data.

## Acceptance criteria

- [ ] Unknown components kept or dropped cleanly with a report
