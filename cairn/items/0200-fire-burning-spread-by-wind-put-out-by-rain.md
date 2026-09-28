---
id: 9bd9e8ab-6eef-44dc-b814-0b376ceec1e5
title: 'Fire: burning, spread by wind, put out by rain'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 7c50b502-5e27-4807-a36c-0654fe9aec97
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
- fa0de3f5-7ee7-4316-9e00-7db21bc43412
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A fire spreads downwind faster than upwind (test)
- [x] Rain puts fires out; wet ground doesn't catch
- [x] Lightning can start a fire in a dry storm

## 2026-09-25

Blocked outside Colony: it depends on stock fields (d77d9e1f), which are in Crafting (due 2027-05-01), and on wind shelter (fa0de3f5, in Colony). DESIGN.md §5 also lists fire spread as a plugin ('Out: ... water flow, fire spread'), so it may belong in a first-party plugin rather than the engine layer this item names.

## Proposed milestone: colony -> crafting (Oddur Sigurdsson, 2026-09-25)

It can't land before stock fields (d77d9e1f), which are in Crafting; keeping it in Colony means Colony can't close. DESIGN.md §5 also puts fire spread in a plugin.

## Accepted milestone: crafting (Oddur Sigurdsson, 2026-09-25)

Proposed by Oddur Sigurdsson on 2026-09-25.

## 2026-09-27

Unblocked: wind shelter (fa0de3f5) is done and stock fields merged as #231.
Built as a plugin, mods/fire (depends on core and weather), per DESIGN §5. The engine gains only generic script calls: rim.fixture_at, rim.floor_at, rim.place (a non-item thing, whole), rim.remove, rim.damage (hp; destroyed at none), rim.designate, plus hp on rim.thing and category on rim.thing_defs. It names no fire content.
Why not a stock field for burning: spread reads neighbours and the wind, which terms can't, and a stock field steps every cell of the map. The script keeps a list of burning cells, so a step costs the fire, not the map. It uses the world RNG and ordered lists, so it's deterministic.
Heat: each burning cell carries fire:flames on its floor layer. Flames emit temperature and light like any emitter, cost 600% to walk through, and carry a beat_out harvest. Near home (30 cells from the colony's middle) the script designates them, and a Firefight work type (priority 1, auto 2/6) does the job. A beaten-out cell is out.
Spread per step (every 250 ticks): chance = 0.14 × heat × flammability × dryness × wind. Flammability comes from the terrain's flammability prop (patched onto core grass 0.8, rich soil 0.6, dirt 0.15, marsh 0.2), a plant 1, or a building's material factor / 3. Dryness = 1 − wetness/60. The wind factor is by octant from wind_dir with a cosine table, 1 + wind/10 × 1.5 × cos, floored at 0.05, so there's no trigonometry. Rain over 0.5 mm/h stops spread and burns heat off. Burnt ground doesn't catch for two days. The draw comes first against the cell's best case, so most neighbours cost no map reads.
Lightning: [[fire.lightning]] data gives strikes per hour by weather. A strike catches under the sky on dry burnable ground with little rain. The plugin adds a summer dry_storm weather type (no rain, windy) where it does; in the core storm's rain it fizzles.
Measured (tests/fire.rs, release, load average about 50): 12 steps in a 12 m/s east wind burned 25 cells east of the start and 0 west. One step with 289 cells burning takes 3.3–4.4 ms (fastest of five), 0.013–0.018 ms a tick amortized, as a spike every 250 ticks.
Not done: items lying in a fire don't burn, pawns in fire take no damage, and there's no draw for a fixture that is itself burning (only the flames under it).
