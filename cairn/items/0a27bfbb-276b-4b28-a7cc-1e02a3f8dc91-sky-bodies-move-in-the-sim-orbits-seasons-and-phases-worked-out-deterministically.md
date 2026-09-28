---
id: 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91
title: 'Sky bodies move in the sim: orbits, seasons and phases, worked out deterministically'
type: feature
status: backlog
milestone: lighting
created: 2026-09-28
updated: 2026-09-28
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

- [ ] At latitude 45 the sun's day is longer at midsummer than at midwinter, and equal at the equinoxes; noon altitude follows the season (test)
- [ ] The moon's phase cycles with `phase_days`, and it rises later each day by its `day_period` (test)
- [ ] A guard test finds no std trig or transcendental calls in rim_sim/src, and the in-crate functions match std within 1e-6 (test)
- [ ] A year of body states hashes the same run to run (test), and agree passes on all four platforms in the queue lane
- [ ] DESIGN.md describes the model; docs/modding and types list the new `sky_body` fields and the `body` input
