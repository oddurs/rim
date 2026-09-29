---
id: 186
uid: d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
title: 'Stock fields: per-cell state with staggered updates'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 181
- 185
created: 2026-09-23
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: l
layer: engine
area: sim
pillar:
- performance
- determinism
- plugin-first
---

## Why

Wetness and snow remember: it rained yesterday, so the ground is still wet. Today's fields are stateless per cell (ambient plus stamps). A new field kind stores a value per cell and changes it by declared rates, cheaply enough for a 250×250 map at 6× speed.

## What

- `kind = "stock"` on `[[field]]`: `range`, `period_minutes`, `rate = [terms]` (per game hour), `base = [terms]` (equilibrium, exposed as `base` / `above_base`), `init = [terms]` (map generation).
- Storage: `i16` hundredths when the range fits, else `i32`; one contiguous array per field.
- Staggered: each tick updates a slice of rows so every cell updates once per period, with elapsed time scaled to the period. No allocation per tick.
- Emitters with `emit = [{ field = "wetness", amount, radius }]` add a rate, using the existing stamp mechanism.
- `rim.field_add(id, x, y, v)` and `rim.field_set`; the overlay and hover work unchanged.
- Included in the state hash. Serialisation hands off to 0060.

## Acceptance criteria

- [x] A stock field declared only in data rises, falls and settles to its base as its terms say (tests)
- [x] Every cell updates exactly once per period, whatever the map size (test)
- [x] 250×250 with two stock fields: at most 0.02 ms mean per tick, measured and recorded here
- [x] Emitters and scripts can add to a stock field
- [x] Deterministic: same seed, same values after a year

## 2026-09-23

Moved to Crafting with the lean Weather sprint. v1 terms (0181) only take global inputs (year, hour, outdoor values, noise, constants); this item extends them with per-cell inputs (field, self, base, above_base, terrain, sky, near) and owns the per-cell benchmark from the dropped spike 0180.

## 2026-09-27

Built as items say, with one change: values are i32 in 1/10000ths (terms' Q), not i16 hundredths when the range fits. The cost per tick is the slice, cells over the period's ticks, not memory bandwidth. One code path, and rates too small for a hundredth still accumulate. Ranges are checked to fit (±200000). Inputs: rate reads input = self, base and above_base; any per-cell term reads input = sky (1 under open sky, 0 under a roof, from Map::covered) and the terrain/near inputs from 9b569a33. A tick's slice is cells×k/P to cells×(k+1)/P for tick k of P, so every cell is worked out exactly once a period on any map (tested on 1×1, 7×3, 64×64 with three levels, and 250×250). Reads in a tick see the values from before it: new values go through a reused scratch, with no allocation per tick. Measured (tests/stock_fields.rs, release, M1 laptop, fastest of three rounds of 4 periods): two stock fields on 250×250 cost 0.0027 ms a tick, against the 0.02 budget. Values are in the state hash (folded per cell, only when the hash is asked for) and in the save: SavedFields.stock, serde default, remapped by field id. A field the save lacks is worked out afresh from init. Emitters on a stock field add to its rate. Boundaries still refuse stock fields. rim.field_add and rim.field_set work only on stock fields and error otherwise.

## 2026-09-27

Added levels = "surface" (the default) or "all". Wetness on the real 192×192 map with three levels below measured 0.29 ms a tick for two fields over all 147k cells: real terms with a dozen inputs cost about 0.4 µs a cell, not the synthetic 18 ns. Weather belongs to the surface, and ore can say levels = "all". A field kept on the surface reads 0 below it, and rim.field_add/field_set error there. A saved array of the wrong length is worked out afresh.

## 2026-09-27

input = sky reads 0 in an enclosed room and 1 elsewhere, from Map::indoors, not Map::covered. covered means within a support's span, which rings every cliff and wall for 4 or 5 cells. The climate report's probe by granite never got rained on.
