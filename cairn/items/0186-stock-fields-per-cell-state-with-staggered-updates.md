---
id: d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
title: 'Stock fields: per-cell state with staggered updates'
type: feature
status: backlog
milestone: crafting
depends_on:
- 3114946b-5434-4171-9cdb-86ae4e7bb38d
- 9b569a33-c488-42df-84c2-9bfa83a14b3e
created: 2026-09-23
updated: 2026-09-23
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

- [ ] A stock field declared only in data rises, falls and settles to its base as its terms say (tests)
- [ ] Every cell updates exactly once per period, whatever the map size (test)
- [ ] 250×250 with two stock fields: at most 0.02 ms mean per tick, measured and recorded here
- [ ] Emitters and scripts can add to a stock field
- [ ] Deterministic: same seed, same values after a year

## 2026-09-23

Moved to Crafting with the lean Weather sprint. v1 terms (0181) only take global inputs (year, hour, outdoor values, noise, constants); this item extends them with per-cell inputs (field, self, base, above_base, terrain, sky, near) and owns the per-cell benchmark from the dropped spike 0180.
