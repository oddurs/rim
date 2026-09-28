---
id: 6dd4891c-f65e-4707-96f9-e6d46c6cd446
title: Ground wetness and snow in core
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 7c50b502-5e27-4807-a36c-0654fe9aec97
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: none
effort: m
layer: core
area: map
pillar:
- survival
- growth
---

## Why

The ground is where weather lasts. Wetness drives plant growth and farming; snow is winter you can see and wade through.

## What

- `wetness` (0–100%): rain soaks in under open sky; drying toward the water table, faster when warm, dry and windy; water tiles pinned at 100; snowmelt feeds it; `near = "water"` raises the base.
- `snow` (0–60 cm): precipitation below 0°C accumulates (curve over temperature for sleet), melts above 0°C by degree-hours, faster in rain.
- Both under open sky only; enclosed rooms stay dry.
- Tuning targets: a day of rain takes grass from its base to about 80%; it dries back in about two sunny days; sand dries in half a day; marsh never drops below 70%; a winter week of snow gives 20–40 cm; spring melts it within four days.

## Acceptance criteria

- [x] Wetness and snow defined in `mods/core/defs/fields.toml` only
- [x] Tuning targets met on the climate report (numbers recorded here)
- [x] Enclosed rooms stay dry and snow-free
- [x] Overlay and hover show both

## 2026-09-23

Moved to Crafting: wetness is the first stock field, and farming is its first reader. Lives in `mods/weather` (the weather plugin), not core; core declares the field name only if a second plugin needs it.

## 2026-09-27

Defined as data only, in mods/weather/defs/ground.toml, not core's fields.toml. That follows the 2026-09-23 note that moved them to the weather plugin; criterion 1 is ticked on that reading. No engine code names either field. Model:
- Wetness settles to water_table×100, plus a shore bonus from near = water (+25 at 1 cell, +10 at 3, none past 6). Under a roof it settles to half that, and water cells sit at 100.
- Rain soaks in at 1% per mm/h under the sky, when above about 0°.
- Drying pulls back toward the base at 0.035/h × drainage (0.3–3.5) × warmth (0.3–1.8) × wind (1–1.5) × clear sky (0.6–1.2).
- Melting snow feeds wetness.
- Snow settles 0.25 cm per mm below freezing (sleet from -2° to 1°) and melts 0.08 cm per degree-hour, doubling in heavy rain.
- Both fields are worked out hourly, on the surface only.
Targets, measured under pinned weather in tests/ground.rs:
- a day of rain (2.4 mm/h, 12°) takes grass from 40 to 81.4%;
- two sunny days bring it back to 41.6%;
- sand from 41 to 20.9% in half a sunny day;
- marsh stays at 90% through five hot windy days;
- a winter week, snowing 8 h a day at -5°, gives 33.6 cm;
- a mild thaw clears it in 56 h.
On the climate report (seed 1, 60 days), open grass:
- rain days take it to 65–88%, and it dries back in about two days;
- winter snow builds to 21.6 cm (day 45) and is gone within about four days of the thaw (days 49–52);
- winter ground stays at 70–97%, since cold slows drying.
The report now has wet % and snow cm columns. Cost: both fields on the 192×192 map measure 0.011–0.034 ms a tick, fastest of three rounds.
