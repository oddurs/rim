---
id: 324
uid: 2e145c65-0b65-41e5-bb3c-a9728379d892
title: 'Modding docs: work types, roles, Auto, readings, orders and lenses'
type: docs
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 340
- 385
- 403
- 413
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p2
api: none
effort: s
layer: core
area: docs
---

## Why

Every rung of the ladder is a mod surface, and none of it is documented beyond DESIGN.md.

## What

- `docs/modding/work.md`: `[[work_type]]` with `auto`, `[[priority_scale]]` labels, `[[work_role]]` and patching core's roles, stances and `[[priority_rule]]` with `when.reading`, replacing the planner, publishing a reading, and adding a lens.
- `docs/modding/vocabulary.md` and `types/rim.d.luau` up to date.
- A worked example: a mod that adds a Tailor work type, a Tailor role, and a "Clothes are worn" order on a reading it publishes.

## Acceptance criteria

- [x] The worked example loads in a fixture test
- [x] Every new def field and Luau function in this milestone is documented
