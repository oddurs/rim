---
id: 183
title: 'Ambient terms: outdoor channels computed from data, with named pushes'
type: feature
status: planned
milestone: weather
depends_on:
- 181
- 182
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: sim
pillar:
- plugin-first
- determinism
---

## Why

The outdoor value of every field is today set by `20_climate.luau` with `rim.set_ambient`, and the last writer silently wins. Declaring ambients as terms makes the climate data, lets incidents add to it rather than overwrite it, and gives every channel a breakdown.

## What

- `[[field]]` gains `ambient = [terms]`. Fields are evaluated in dependency order (temperature reads cloud); a cycle is a load error naming the fields.
- Recomputed every 20 ticks (one game minute and a bit), O(channels).
- `rim.push_ambient(field, key, value, hours)`: a named, timed offset, shown in the breakdown and expiring on its own. Pushing the same key replaces it. `rim.clear_ambient(field, key)`.
- `rim.set_ambient` keeps working for fields without ambient terms, and warns once for fields with them.
- Core fields: `temperature`, `light`, `cloud`, `precipitation`, `humidity`, `wind`, `wind_dir` (degrees, eased the short way round). `20_climate.luau` is deleted; its curve becomes core data.

## Acceptance criteria

- [ ] Core's climate is data only; `20_climate.luau` is gone
- [ ] Spring's first three days stay within 1.5°C of today's curve each hour, so the warmth tuning in §4a holds
- [ ] A push shows in the breakdown, stacks with other keys and expires on time
- [ ] Dependency cycles are reported at load
- [ ] Tick cost measured before and after
