---
id: 563
uid: 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91
title: 'Sky bodies move in the sim: orbits, seasons and phases, worked out deterministically'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: additive
effort: l
layer: engine
area: sim
---

## Why

The user wants the sun and moon simulated in full (2026-09-28). That reverses the earlier call that the moon is picture-only. Today the sim's daylight is a fixed hour curve: the same day all year, with no seasons. Where a body sits in the sky exists only in the renderer's `rise`/`set`/`peak`/`arc` curves. So the sim and the picture aren't one model.

## What

- `[[sky_body]]` gains orbit data. It's all optional, and the defaults reproduce today's sun:
  - `day_period`: days per crossing. The sun is 1.0. The moon is about 1.035, so it rises about 50 minutes later each day.
  - `phase_days` and `phase_offset`: the cycle of lit fraction. None for the sun.
  - `tilt`: degrees of declination swing over the calendar year. About 23 for the sun.
  - `latitude`: the world's, on the calendar def, default 45.
- The engine works out each body's state as a pure function of (tick, calendar, defs):
  - `altitude` and `azimuth` in degrees;
  - `up`, 0..1 above the horizon, with a short twilight band;
  - `phase`, the lit fraction, 0..1.
  Nothing is saved: it's derived from the tick.
- **Deterministic maths only.** No `f64::sin/cos/tan/asin/acos/atan2/powf/exp/ln` in rim_sim/src: the platform libm differs, and agree would split. Use one in-crate polynomial set (sin, cos, asin, atan2) built from + − × ÷ only.
- Field term input: `{ input = "body", body = "<id>", of = "altitude" | "azimuth" | "up" | "phase" }`, so fields read bodies as data. A `World` accessor returns every body's state for the renderer. That accessor is the only source the client uses.
- Cost: worked out at most every few dozen ticks and cached. Report the per-tick cost.

## Acceptance criteria

- [x] At latitude 45 the sun's day is longer at midsummer than at midwinter, and equal at the equinoxes; noon altitude follows the season (test)
- [x] The moon's phase cycles with `phase_days`, and it rises later each day by its `day_period` (test)
- [x] A guard test finds no std trig or transcendental calls in rim_sim/src, and the in-crate functions match std within 1e-6 (test)
- [x] A year of body states hashes the same run to run (test), and agree passes on all four platforms in the queue lane
- [x] DESIGN.md describes the model; docs/modding and types list the new `sky_body` fields and the `body` input

## 2026-09-28

Built as rim_sim::sky. Body states are worked out in Fields::update_ambient, on the outdoor values' beat (AMBIENT_INTERVAL = 20 ticks, when the sun moves 0.36°), so there is one cache and no second clock. A load recomputes them from the saved clock. A refresh costs 157 ns for core's two bodies, about 8 ns a tick. Added a `transit` hour (default 12) beside the listed orbit fields: without it a moon's crossing can't be placed against its phases. Core's moon has day_period 15/14, so it laps the sun once a phase cycle and every full moon is highest at midnight, and tilt -23, so a full moon rides high in winter. The calendar gains `midsummer` (default 0.25; core 0.375, where mods/weather's warmest day is). The maths: sin by Taylor to x^19 on ±π/2, atan folded to ±tan(π/8) with 21 terms, atan2 by quadrant, altitude as atan2(up, horizontal), so no asin. All within 5e-16 of std. f64::sqrt is allowed: IEEE 754 requires it correctly rounded, and it's an instruction, not libm. The guard (tests/no_trig.rs) bans method and f64::/f32:: forms. `x.log(b)` counts only with a number for its base, since savefile has a method of its own named log. Scripts read a body with rim.sky_body(id). Nothing in core reads a body yet, so the sim's numbers are unchanged until a9e6b195.

## 2026-09-28

Criterion 5: docs/modding/weather.md lists the orbit keys and the body input. types/rim.d.luau gains the SkyBody type and rim.sky_body; the defs have no schema file of their own. Criterion 4's hash test is pinned; agree is for the queue lane.

## 2026-09-28

Criterion 4: agree passed on all four platforms in queue run 36470949523 (https://github.com/oddurs/rim/actions/runs/36470949523), and its crosscheck matched main's run 36465180225 through day 60. Merged as #328.
