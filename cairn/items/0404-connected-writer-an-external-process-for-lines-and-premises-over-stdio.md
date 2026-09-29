---
id: 404
uid: bdb88bff-4dd2-4bee-acea-be3c937a8b54
title: 'Connected writer: an external process for lines and premises over stdio'
type: feature
status: backlog
milestone: story
depends_on:
- 406
- 412
created: 2026-09-26
updated: 2026-09-26
priority: p3
api: additive
effort: l
layer: client
area: ui
pillar:
- plugin-first
---

## Why

A player can connect their own model, or any program, as the writer. It never
runs in the sim (DESIGN.md §4g, Telling). No model ships with the game.

## Acceptance criteria

- [ ] A documented JSON-lines protocol over stdio: intent in, line out; prompt in, premise package out
- [ ] Lines naming anything not in the intent are rejected
- [ ] Time out to templates; the sim never waits
- [ ] Lines kept in the save's presentation sidecar
- [ ] Premise packages pass `rim check` before they are offered
