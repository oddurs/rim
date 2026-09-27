---
id: 102b4525-dc8d-4082-af85-a876add438c0
title: A standing order is announced when its reading crosses, even while its season holds it back
type: bug
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: engine
area: ai
---

## Why

On the first morning of spring the news says "Standing order on: Wood before winter (0)". The wood reading is under 60, but the order waits for autumn and moves nothing. `rule_started` fired on the band crossing alone, ignoring the rule's other conditions.

## What

- `rule_started` and `rule_stopped` fire when a reading rule enters or leaves the rules that hold: band, season, hours, stance and the colony's switch together.
- Nothing is announced on the first work-out after a load.

## Acceptance criteria

- [x] A low reading in spring announces nothing for an autumn order; autumn arriving announces it (test)
- [x] Loading a save with an order on announces nothing (test)
- [x] Switching an acting order off announces it stopped, and a crossed order switched back on announces it started (test)
