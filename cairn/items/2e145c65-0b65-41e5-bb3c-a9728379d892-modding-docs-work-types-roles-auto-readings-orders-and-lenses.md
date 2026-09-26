---
id: 2e145c65-0b65-41e5-bb3c-a9728379d892
title: 'Modding docs: work types, roles, Auto, readings, orders and lenses'
type: docs
status: backlog
milestone: work
depends_on:
- 4c9fc19b-3b7b-4bc3-93b4-7b5d600cbc8d
- 97a12814-63c8-4096-9009-c5c45712cbf7
- bc9b9a91-1e87-4676-9e7c-9c44a5fa7680
- d41e504f-4e16-493c-bb47-28c5d25a6f05
created: 2026-09-26
updated: 2026-09-26
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

- [ ] The worked example loads in a fixture test
- [ ] Every new def field and Luau function in this milestone is documented
