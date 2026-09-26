---
id: 03b9b791-082e-48f1-b51e-af6f42627685
title: Derived fields and feels-like temperature
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 3114946b-5434-4171-9cdb-86ae4e7bb38d
- fa0de3f5-7ee7-4316-9e00-7db21bc43412
created: 2026-09-23
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
effort: s
layer: engine
area: needs
pillar:
- survival
- plugin-first
---

## Why

What hurts a colonist is not the thermometer but wind chill and cold rain. A derived field computes a value from other fields on read, with no storage; feels-like temperature is the first one, and warmth reads it with a one-line data change.

## What

- `kind = "derived"` with `value = [terms]`, computed on read, O(terms). Overlay and hover work unchanged.
- Core `feels_like` = temperature − wind chill (wind × exposure × curve of temperature, only below 10°C) − a wet penalty (precipitation × sky).
- The warmth need's field becomes `feels_like`.
- Indoors, feels-like equals the room temperature.

## Acceptance criteria

- [x] Derived fields load from data and read correctly at any cell
- [x] Standing in the lee of a wall in a cold wind is measurably warmer than in the open (test)
- [x] Warmth drains faster in a cold storm than on a still night at the same temperature
- [x] Warmth balance rerun (see the balance item) with no regression in the first week

## 2026-09-23

Moved to Colony with wind shelter (0188).

## 2026-09-25

Built as a field kind: kind = "derived" with value = terms. Terms gained a per-cell input, { field = "id" }: another field's value at the cell being read (the room's indoors, the lee's in the lee), and its outdoor value where there is no cell. A derived field's value terms are also its ambient terms, so its outdoor reading, pushes and dependency ordering (cycles refused) come for free; Fields::value_fixed evaluates them per cell on read and stores nothing. Core feels_like = temperature + wind_chill + wet: wind_chill = -(wind past 4 m/s) × exposure × curve(temperature: 1.0 at -20°, 0.5 at 0°, 0 at 10°), so 16 m/s at 0° in the open is -6°; wet = -0.5 × (precipitation past a 2 mm/h drizzle) × curve(temperature: 1 at 5°, 0 at 20°). The warmth need reads feels_like, and the comfort search now finds the lee of a wall. Cost: needs 0.0001 ms a tick in the bench. Balance (60 seeds x 7 days, weather on), shelter branch vs this: first version (chill from any wind, wet from any rain) froze 14.1 to 23.7 colonist-hours per run without a fire, a regression, so both got thresholds matching drafty rooms' breeze. Final: no fire lost 2 to 3, runs with a death 11 to 13, froze 14.1 to 16.1 h, founder froze after night one 33 to 36; with a fire lost 4 to 4, deaths 5 to 7, froze 5.5 to 9.0 h. The with-fire numbers move by more than the change could explain (the harsher version measured 6.5), so at 60 seeds the harness's noise is about ±3 colonist-hours.

## 2026-09-25

Rebased onto drafty rooms, then measured the combination at 120 seeds x 7 days against main (which has drafty rooms). The first tuning (chill up to 1°/m/s, wet 0.5°/mm/h) doubled runs with a death: 14 to 28 of 120, lost 6 to 9, with freezing hours flat (17.9 to 17.4). The deaths were in storms, typically after boars broke the hut's door (seed 3: storm at d5.66, two colonists dead by d5.84). Halved: chill up to 0.5°/m/s past 4 m/s (16 m/s at 0° is -3°), wet 0.3°/mm/h past 2. Result: runs with a death 18 (main 14), lost 8 (6), froze 16.4 h (17.9), founder froze after night one 71 (68): about one standard deviation, within noise. Storms are colder than before, but not lethal in week one.
