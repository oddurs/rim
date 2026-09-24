---
id: 179
key: weather
title: Weather
type: milestone
status: planned
depends_on:
- 161
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
due: 2026-10-14
---

A three-week sprint inside Shelter. Weather becomes a first-class system: seasons, weather regimes with a forecast, ground wetness and snow, wind shelter, plants that grow in the weather, and lighting and weather you can see. All of it is one engine mechanism (labelled terms over curves, on field layers) and core data, so a mod can add a monsoon or a harsher winter without touching Rust. Design: DESIGN.md §4c.

## Sprint goal

Play a full year. Spring starts like today, a storm rolls in on the forecast you watched coming, rain darkens the ground and the berry bushes flush, the wind finds the gap in your wall, and the first snow of winter stops the bushes and slows your colonists on the way to the woodpile. A campfire is the only light in the dark. Every number on screen can explain itself.

## Plan

- **Days 1–2, the core:** spike (evaluator cost, a headless year), the term and curve evaluator, the calendar.
- **Days 3–5, the air:** ambient terms replace the climate script; weather regimes and the forecast queue; terrain properties and `near`.
- **Days 6–9, the ground:** stock fields; wetness and snow in core; wind shelter; feels-like temperature; drafty rooms.
- **Days 10–11, consequences:** plant growth driven by fields; snow slows movement.
- **Days 12–16, seeing it:** lighting from the light field; the ground renderer; precipitation, fog and lightning; weather in the HUD.
- **Days 17–21, hackability and balance:** devtools and the climate report; incidents on pushes; guide and `weather_plus`; the year balance pass.

## Definition of done

- `20_climate.luau` is gone: core's climate and weather are data, and the client's hard-coded night curve is gone (it reads the light field).
- A year runs headless on 40 seeds; the climate report and balance numbers are recorded in DESIGN.md §4c.
- Weather adds at most 0.03 ms mean and 0.2 ms max per sim tick at 250×250, and client weather visuals cost at most 1 ms per frame, both measured in the profiler.
- A year with weather hashes identically on every run (determinism test), and the forecast is never wrong without an override.
- `weather_plus` adds a regime and an incident with no engine change, and the guide's samples run in a test.
- Every item below is closed or explicitly moved out, with a note saying why.

## Out of scope (follow-ups filed)

- Water flow, runoff and floods: 0199 (Scale)
- Fire, spread by wind and put out by rain, with lightning strikes: 0200 (Colony)
- Pawn wetness and clothing insulation: 0201 (Crafting)
- Mood from weather and seasons: 0202 (Mood)
- Other biomes as a plugin: 0203 (World)
- Weather audio: 0204 (1.0)
- Fog and darkness limit sight: 0205 (Defense)
- A seasonal storyteller: 0206 (Eras)
- Saving the weather state: folded into 0060
- Farming (0110) now depends on this sprint's growth, wetness, terrain properties and calendar

## Risks

- **Term evaluator cost** in the per-cell loop. The spike measures it before anything is built on it; the fallback is baking curves into 256-entry lookup tables.
- **Balance drift:** seasons change the temperature every system was tuned on. Spring must reproduce today's curve, and the balance item reruns the warmth and colony harnesses across a year.
- **Rendering rewrite:** the ground renderer replaces per-cell rectangles with textures and a shader; the autotest screenshots catch regressions, and the old path stays until the new one matches.
- **Scope:** 21 items in three weeks. p2 items (drafts, snow movement) move out first.
