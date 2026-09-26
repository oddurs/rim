---
id: f1924f03-4122-4997-b1bd-f826e9cd3ac2
title: Why a colonist is doing that, and who will take a job
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 0e73145a-39c4-4f1d-88a4-59d803a2f535
- f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
created: 2026-09-24
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
pillar:
- growth
effort: m
layer: client
area: ui
---

## Why

"Why is nobody cooking?" is the question players ask most and the one the genre answers worst. The data already exists in the pools; show it (DESIGN.md §4d).

## What

- The why panel on a colonist: what they picked with its score, and each work type they passed over with its reason (unreachable, reserved, no materials, priority 0), each linked to the map.
- Hovering a Work Board column lights its waiting jobs on the map.
- Hovering a job on the map ranks who would take it and roughly when ("Bo in ~20s, then Cyd").
- `rim.explain_work(pawn)` and `view.explain_work(pawn)` expose the same to scripts and mods.

## Acceptance criteria

- [x] The why panel names the real reason for each skipped work type (tests for each reason)
- [x] The "who takes this" ranking matches who actually takes it on a test map
- [x] Costs nothing measurable when nobody is inspecting

## 2026-09-25

Tools (7016d86b) give a refusal reason the why panel should show: rim_sim::ai::work_blocked says 'Needs a chopping tool.' when no tool in the colony has the tag, and 'Needs a free chopping tool.' when the colony's are held, claimed or out of reach. The thing inspector already shows it.

## 2026-09-25

Work pools (0e73145a) were dropped by the owner on 2026-09-25: the candidates and the reason each work type was passed over come from find_work itself, recorded only for a pawn someone is inspecting. The body's 'the data already exists in the pools' now means find_work's sources, which work_waiting (from the Work Board) also counts.

## 2026-09-25

Claimed with --force: its blocker, work pools 0e73145a, was dropped by the owner on 2026-09-25; the drop is in PR #118, not yet merged when this started.

## 2026-09-25

Built as find_work split in two: choose_work (read-only: the choice, with an optional Refusals sink that keeps the nearest refusal per work type) and find_work (choose, then reserve). explain_work(w, pawn) runs the same choice with the sink: each work type in tie-break order is Picked(job), Never (level 0), Nothing (none waiting), Reserved (the nearest is someone else's), Unreachable, NoMaterials(what), NeedsTool(tags) or Beaten(the type that won). who_takes(w, target) runs the choice for each free colonist and ranks those who'd pick this target: idle ones by when they next think (a wanderer once its wander is up) then spawn order, with an estimate of think wait plus walk; busy ones after, with none. order::why_text words a reason; order::describe words any job. Surfaces: rim.explain_work, rim.who_takes; view.explain_work; the inspector's Work tab shows each type's reason (the pick in green); the hover readout says 'Next: Bo in ~20 s, then Cyd' over anything with work on it. Cost when nobody inspects: bench mean tick 0.062 ms either side, p99 2.64 vs 2.67 ms. Tests: a reason each for Never, Nothing, Picked, Beaten, Reserved, Unreachable, NoMaterials, NeedsTool; the first in who_takes' line is who reserves the job; explaining changes nothing (state hash). Also fixed WorkTypeDef.order's doc: order breaks ties at the same level and distance, as §4d rules (rim-fa's sweep found the doc misleading). Lighting a board column's jobs on the map is renderer work: split to 9ebfa104.

## 2026-09-25

Review fixes: the hover's who_takes was 30 ms with 30 idle colonists and 29 walls planned with no wood anywhere, because choose_work (so find_work too) scanned every thing for each plan whose material was missing. Now a material with nothing to bring is asked once per search: 0.35 ms for the same case, and explain_work 0.01 ms. who_takes asks only colonists free to choose (idle, or wandering); busy ones aren't searched. A job someone holds has nobody next, and the hover says 'Being done by X'. Haul skipped for a better level now names the type that actually won. A tool gate with no known tag reads 'Needs a tool'.
