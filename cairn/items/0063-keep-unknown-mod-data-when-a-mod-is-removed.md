---
id: b4ad855e-8a8a-49ca-8d2e-b7bc8eb2859a
title: Keep unknown mod data when a mod is removed
type: feature
status: backlog
milestone: persistence
depends_on:
- c5d185be-9bb8-491d-b73d-94f75e4018f0
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
