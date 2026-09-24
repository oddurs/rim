---
id: 179
key: weather
title: Weather
type: milestone
status: done
depends_on:
- 161
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
api: none
due: 2026-10-01
---

A one-week sprint inside Shelter. The world gets seasons and weather you can see coming: firelit nights, rain that rolls in on the forecast, and a first winter that needs a heated hut. The engine gains a few general mechanisms (terms over curves, a calendar, named contributions to outdoor values, script data); the weather itself is a first-party plugin, `mods/weather`, built only on the public API. Design: DESIGN.md §4c.

## Sprint goal

Play a year with `weather` installed: a firelit first night, spring rain arriving when the forecast said it would, a storm that sends everyone inside, and a winter that kills a colony without heating. Remove the plugin and the game plays exactly as it did before the sprint.

## Plan

- **Days 1–2, engine:** terms and curves (outdoor values only), the calendar and script events, named contributions to outdoor values, script data.
- **Days 3–4, the plugin:** `mods/weather` with seasons, weather types and a forecast; cold snap, heat wave and storm.
- **Days 5–6, seeing it:** lighting from the light field; rain, snow, fog and lightning from the weather channels; season, weather and forecast in the HUD.
- **Day 7, proof:** the guide, a year of balance, and a core-only CI run.

## Definition of done

- `20_climate.luau` is gone: core's day and night are data, and the client's hard-coded night curve is gone (it reads the light field).
- Seasons, weather types, the forecast and weather incidents live in `mods/weather`, which uses only the public API. The engine never names a weather type.
- The game loads and plays with core alone (CI), and the first week with `weather` matches the §4a warmth numbers.
- A year with weather hashes identically on every run, and the forecast is never wrong without an override.
- Every item below is closed or explicitly moved out, with a note saying why.

## Out of scope (moved to the milestones that use them)

- Per-cell weather state for farming: terrain properties (0185), stock fields (0186), ground wetness and snow (0187), plant growth (0191), snow and mud slowing movement (0192). All in Crafting, where farming (0110) needs them.
- Wind shelter (0188, as a plain engine function), feels-like temperature (0189), drafty rooms (0190): Colony.
- The ground shader renderer (0193): Scale, with terrain render caching (0098).
- Weather devtools (0196): SDK. Custom def kinds (new): Plugin API.
- Water flow (0199), fire (0200), getting wet (0201), mood (0202), biomes (0203), audio (0204), sight in fog (0205), seasonal storyteller (0206).

## Risks

- **Winter balance:** seasons change the temperature everything was tuned on. Spring reproduces today's curve; the balance item runs a full year.
- **Plugin-first cost:** the weather plugin is Luau, so its data is less typed. Load-time checks in the plugin, and custom def kinds later, cover it.

## 2026-09-23

Filed 208 (custom def kinds, Plugin API) as a follow-up; weather types register through Luau until then.

## 2026-09-23

Sprint closed 2026-09-24, ahead of its 2026-10-01 date. Definition of done: 20_climate.luau is gone and core's day and night are data; the client's darkness curve is gone (it reads the light field); seasons, weather, the forecast and incidents live in mods/weather on the public API, and the engine never names a weather type; core alone plays in CI; the first week with weather matches §4a within noise; a year hashes identically and the forecast is never wrong except where forced. Every item is closed. Merged daylight/light split from a parallel session (c3bff5e). Follow-ups: 0208 custom def kinds (weather types to data), 0209 sky bodies, 0196 weather devtools, the per-cell items in Crafting.
