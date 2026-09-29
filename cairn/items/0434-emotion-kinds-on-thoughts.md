---
id: 434
uid: fa4b580d-885e-41ac-98ea-5b1a04b9d615
title: Emotion kinds on thoughts
type: feature
status: backlog
milestone: story
depends_on:
- 88
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: plugin
area: needs
pillar:
- plugin-first
---

## Why

A person at −20 from grief acts and speaks differently from one at −20 from
disgust (DESIGN.md §4g, Feelings).

## Acceptance criteria

- [ ] `[[emotion]]` kinds declared by `mood`; a thought names one
- [ ] Mood stays the sum; each pawn's strongest emotions are readable from scripts
- [ ] Mods add emotions as data
