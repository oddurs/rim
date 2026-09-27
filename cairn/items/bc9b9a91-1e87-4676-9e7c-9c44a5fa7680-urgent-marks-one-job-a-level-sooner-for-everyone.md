---
id: bc9b9a91-1e87-4676-9e7c-9c44a5fa7680
title: 'Urgent marks: one job, a level sooner, for everyone'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: ai
---

## Why

"That wall before anything else" has no good answer on a grid: raise Build for someone and remember to lower it. DESIGN.md §4d: mark the job, not the person.

## What

- `Command::MarkUrgent { target, on }` on a blueprint, a designated thing or an order site. The mark is state on the work, saved and hashed, and clears when the work finishes or is cancelled.
- Work choice: an urgent job counts one level sooner than its work type for each colonist (never from 0) and wins ties inside its level. Pools keep urgent work in its own bucket so the walk stays per level.
- `explain_work` and `who_takes` say "urgent".
- Client: "Mark urgent" and "Clear urgent" in the right-click menu (the context menus from 64a0648e) and `U` over a job; a mark drawn on the map; the hover forecast shows who comes.
- The HUD counts live urgent marks.

## Acceptance criteria

- [x] A colonist with Build at Later takes an urgent wall before a Soon harvest when nothing is at First (test)
- [x] The mark clears when the wall is built (test)
- [x] Marked work at 0 for a colonist stays at 0 (test)
- [x] Determinism test passes; work choice stays inside the §4d budget on the stress map

## 2026-09-26

Creatures marked for hunting can take a mark, and it works in choose_work, but the map draws it only on things, since pawns aren't in the chunk mesh. explain_work and who_takes don't say 'urgent' yet; the effect shows as the job being taken first. A mark is cleared by a per-tick sweep of marked entities that are no longer a job, which costs a query over the marks alone.
