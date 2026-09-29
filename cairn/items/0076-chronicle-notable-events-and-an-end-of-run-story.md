---
id: 76
uid: 58ba2b7a-8407-462e-916a-579147e2436a
title: 'Chronicle: notable events and an end-of-run story'
type: feature
status: backlog
milestone: eras
created: 2026-09-22
updated: 2026-09-26
priority: p1
api: additive
effort: l
layer: engine
area: sim
pillar:
- growth
---

## Why

Every run should leave a story.

## Acceptance criteria

- [ ] Events recorded with day
- [ ] Readable chronicle when the colony falls
- [ ] Records are compact and structured: kind, tick, place, actors, witnesses, a small data table
- [ ] Each record has a notability from its event def; routine events live in a short ring, notable ones are kept, bounded in size
- [ ] Saved as its own section, and readable from Luau

## 2026-09-26

The Story milestone (DESIGN.md §4g) builds its memories on this store: the Memories item indexes these records by entity, place and day, and Witnesses fills the witnesses field. So the records are structured and bounded from the start rather than lines of text.
