---
id: 8733f34a-4447-47ad-8f7e-42af2c66e4ac
title: Warmth need and exposure outdoors
type: feature
status: done
milestone: shelter
depends_on:
- f51af7ba-8fba-4127-bda0-07387348a786
- 6adcd8f6-07a8-44d2-9135-872bcc0115ad
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: needs
pillar:
- survival
---

## Why

Shelter only matters if the world hurts you outside it.

## Acceptance criteria

- [x] Warmth drains under open sky at night and in bad weather
- [x] Enclosed rooms protect
- [x] Hypothermia damage at zero

## 2026-09-23

Warmth is a field need on temperature (comfort 10-32°C). Drains in proportion to degrees outside comfort, refills inside, 40 hp/day hypothermia at zero. Colonists seek the nearest comfortable cell (or the least-bad one if clearly better), sleep somewhere warm without a bed, wake if warmth drops below 15%, and wait somewhere comfortable when idle. Bug found by tracing: the comfort search marked cells seen before the corner-cutting check, so rooms behind doors were unreachable. Balance (40 seeds, founder hours at zero warmth after night one): no shelter 14.5h in 40/40 runs, hut 2.6h in 17/40, hut+fire 2.8h in 12/40. Numbers and reasoning in DESIGN.md 4a.
