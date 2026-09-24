---
id: 1aab96a4-3a6a-4e09-b4c1-e097e3310cc1
title: 'Seasonal storyteller: incidents that know the time of year'
type: feature
status: backlog
milestone: eras
depends_on:
- cee5648a-9714-4498-9c57-1f46253100d0
- eb3b0ac7-9922-4ab2-b9c1-8171e0b7f05f
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: s
layer: core
area: storyteller
pillar:
- growth
- survival
---

## Why

A good storyteller paces the year: migrations in spring, raids after harvest, wanderers seeking shelter as winter comes. The calendar (0182) makes this plain Luau.

## What

- Incident weights in `storyteller.luau` read `rim.season()`.
- A calm spell after a hard weather incident, so storms and raids don't stack.

## Acceptance criteria

- [ ] Incident mix differs by season on the balance harness (recorded)
- [ ] No raid lands within a day of a cold snap or storm ending
