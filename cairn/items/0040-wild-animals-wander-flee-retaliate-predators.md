---
id: 40
title: 'Wild animals: wander, flee, retaliate, predators'
type: feature
status: done
milestone: castaway
assignee: Oddur Sigurdsson
depends_on:
- 35
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: ai
pillar:
- survival
---

## Why

Early threats are animals. Prey flees, others retaliate, predators hunt intelligent creatures nearby.

## Acceptance criteria

- [x] flees / aggressive flags from creature defs
- [x] Hunt designation targets butcherable wildlife

## 2026-09-22

tests/animals.rs, 6 tests on an isolated founder: provoked prey flees and gets >= 6 cells away; prey never attacks unprovoked; the boar (from wildlife_plus: neither flees nor aggressive) ignores people until provoked, then fights back; a wolf 5 cells away attacks; a hunt designation gets a hare killed and butchered into meat; colonists can't be designated for hunting. Mutation-checked: disabling the flee branch fails only the flee test; disabling predator aggression fails only the predator test.
