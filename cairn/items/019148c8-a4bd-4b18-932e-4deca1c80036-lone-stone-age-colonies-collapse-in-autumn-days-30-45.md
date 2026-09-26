---
id: 019148c8-a4bd-4b18-932e-4deca1c80036
title: Lone stone-age colonies collapse in autumn (days 30-45)
type: chore
status: backlog
milestone: defense
depends_on:
- 21fc9ea3-86d1-48c7-be8a-3fd28e9dcf0d
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: l
layer: core
area: storyteller
---

## Why

With urgency in work choice (8b2e05db) and #122's tuning, 14 of 20 lone stone-age colonies are alive at day 30 (examples/stone_age.rs, 20 seeds). By day 45, 19 of 20 are gone. The year is 60 days, so days 30-45 are autumn. Deaths over those 20 seeds, in five-day spans from day 30: cold 5, 15, 8; raiders 19, 10, 1; wolves 15, 15, 0. Cold doesn't appear before day 30, raiders from day 15, and wolves all along.

## What

- Why cold kills in autumn: do colonies outgrow their first hut as wanderers join? Is firewood running out, or is the cob hut still missing?
- Raid size against strength (rim.colony_strength now counts skill) as the colony grows through wanderers.
- Wolves: how often, how many, and how aggressive, for a colony with stone tools.

## Acceptance criteria

- [ ] At least half the lone colonies alive at day 45 over 20 seeds, with deaths by cause recorded
- [ ] The first-night and first-week targets in stone_age.rs still pass
