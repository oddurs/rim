---
id: 38
title: 'Needs: food and rest, starvation and healing'
type: feature
status: done
milestone: castaway
depends_on:
- 35
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: additive
effort: m
layer: engine
area: needs
pillar:
- survival
---

## Why

Needs are data (decay, seek threshold, damage at empty) with engine satisfier kinds.

## Acceptance criteria

- [x] Food satisfied by eating items; rest by sleeping, faster in beds
- [x] Starvation damages, fed pawns heal
