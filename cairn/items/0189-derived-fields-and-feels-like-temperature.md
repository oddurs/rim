---
id: 189
title: Derived fields and feels-like temperature
type: feature
status: planned
milestone: weather
depends_on:
- 181
- 188
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: s
layer: engine
area: needs
pillar:
- survival
- plugin-first
---

## Why

What hurts a colonist is not the thermometer but wind chill and cold rain. A derived field computes a value from other fields on read, with no storage; feels-like temperature is the first one, and warmth reads it with a one-line data change.

## What

- `kind = "derived"` with `value = [terms]`, computed on read, O(terms). Overlay and hover work unchanged.
- Core `feels_like` = temperature − wind chill (wind × exposure × curve of temperature, only below 10°C) − a wet penalty (precipitation × sky).
- The warmth need's field becomes `feels_like`.
- Indoors, feels-like equals the room temperature.

## Acceptance criteria

- [ ] Derived fields load from data and read correctly at any cell
- [ ] Standing in the lee of a wall in a cold wind is measurably warmer than in the open (test)
- [ ] Warmth drains faster in a cold storm than on a still night at the same temperature
- [ ] Warmth balance rerun (see the balance item) with no regression in the first week
