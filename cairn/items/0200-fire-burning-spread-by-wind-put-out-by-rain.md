---
id: 9bd9e8ab-6eef-44dc-b814-0b376ceec1e5
title: 'Fire: burning, spread by wind, put out by rain'
type: feature
status: backlog
milestone: colony
depends_on:
- 7c50b502-5e27-4807-a36c-0654fe9aec97
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
- fa0de3f5-7ee7-4316-9e00-7db21bc43412
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: l
layer: engine
area: sim
pillar:
- survival
- performance
---

## Why

Fire is the other half of weather: a dry summer with wind makes a grass fire a disaster, rain ends it, and lightning in a storm starts one. The Weather sprint provides every input fire needs (wetness, wind and direction, exposure, precipitation).

## What

- Flammability on thing and terrain defs; burning as a stock field or component with heat emission.
- Spread weighted by wind direction and dryness, suppressed by wetness and rain.
- Lightning strikes in storms (core `storm` regime) as an ignition source; colonists fight fires as a job.

## Acceptance criteria

- [ ] A fire spreads downwind faster than upwind (test)
- [ ] Rain puts fires out; wet ground doesn't catch
- [ ] Lightning can start a fire in a dry storm
