---
id: 21fc9ea3-86d1-48c7-be8a-3fd28e9dcf0d
title: Storyteller pressure on a lone stone-age colony
type: chore
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: m
layer: core
area: storyteller
---

## Why

With the stone age on, a lone colonist gets the first days right: shelter by night one, an axe by day 2, a felled tree by day 3 (4bd94457). Then the colony dies anyway. Across crosscheck seeds 5, 6, 7, 9, 11 and 12, every colony was gone by day 45, and on seed 1 wolves killed the founder on day 3. The storyteller and the predators were tuned for a colonist with an axe on day one, not one knapping flint.

## What

- Measure deaths and their causes by day over a seed sweep with all shipped mods, extending `examples/stone_age.rs` or `examples/balance.rs`.
- Tune raid timing and size, and predator aggression, against the colony's real strength. The strength score should count its tools, if it doesn't already.

## Acceptance criteria

- [x] The causes of death by day, recorded over 20 seeds
- [x] At least half the lone colonies alive at day 30

## 2026-09-26

examples/stone_age.rs now records every colonist's death with its day and cause (an attacker in the last four hours, else the need that ran out: starvation, cold), prints deaths by cause in five-day spans, and colonies alive at day 30. rim.colony_strength, which raids are weighed against, now counts each colonist's real melee (ai::melee_base: skill and the founder's edge), not the creature's base damage; tools don't add to melee in the engine, so they aren't counted. Measured, 20 seeds x 30 days, with urgency (8b2e05db) in: 14/20 colonies alive at day 30, before and after the strength change. Deaths by cause in five-day spans: raiders 0/0/1/14/28/16, wolves 6/6/13/5/19/22, cold 0 until day 30 (the old strength measure: raiders 0/0/1/23/19/32, wolves 6/5/8/3/8/16, cold 4 in days 25-30; totals 130 vs 125, within noise). Both criteria hold. But at 45 days 19/20 colonies are gone: autumn (days 30-45 of a 60-day year) brings cold (5/15/8 deaths) on top of raiders and wolves. That was the item's premise, and it isn't solved here: filed as 019148c8 (defense) rather than tuned blind.

## 2026-09-26

Review: raiders now print as 'raiders' rather than their species label ('human'); a cause is whatever was targeting the colonist in the last four hours, not necessarily a blow. The strength change (reviewer's numbers): a founder counts 8.0-9.3 instead of 6.0, so a lone founder's raids shrink about a fifth to a quarter; a wanderer counts 4.7-6.0 instead of 6.0; a founder with three wanderers is about as before.
