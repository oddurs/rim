---
id: efd56e49-5ca4-409a-b26b-8c41be8d1a4a
title: The renderer draws the sky from the sim's bodies
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: breaking
effort: m
layer: client
area: render
---

## Why

For the picture and the sim to be one model, the renderer should take each body's position and brightness from the sim (0a27bfbb) and keep no curves of its own.

## What

- The client reads each body's `altitude`, `azimuth`, `up` and `phase` from the sim's accessor. Its own `rise`/`set`/`peak`/`arc` go. Shadows follow the real azimuth, so they swing through the seasons.
- The moon's phase shows in the picture through its light: moonlight's brightness and the strength of its shadows. That light comes from the same number the sim uses. The view is top-down, with no sky to draw a disc in.
- Removing `rise`/`set`/`peak`/`arc` from `[[sky_body]]` is a defs break: bump the API version as DESIGN.md says, and update the two-suns example and the docs.
- Lighting presets, the autotest's lighting sections and the render bench keep working.

## Acceptance criteria

- [x] The client reads no rise/set/peak/arc; the fields are gone from the schema and the API version is bumped
- [x] At the same hour a shadow points differently at midsummer and midwinter (autotest)
- [x] The picture's moonlight and the sim's are one number (test or autotest)
- [x] The render bench stays within budget in the queue lane

## 2026-09-28

Claimed ahead of 0a27bfbb, at rim-c2's request: built on quiet-field's published BodyState accessor, and merges after 0a27bfbb.

## 2026-09-28

The body said the moon's disc and halo show its phase. The view is top-down and draws no sky, so rim-c2 agreed the phase shows through moonlight's brightness and its shadows' strength, and the line is amended. A phase glyph by the clock would be its own p3 UI item; not filed, since nothing asks for it yet.

## 2026-09-28

Done as:
- The renderer takes each body's altitude and azimuth from World::sky_body_states and keeps no path: sun_at, SunPath, sky.sun and a body's rise/set/peak/arc are gone. [[sky]] denies unknown fields, so a mod naming the old keys fails to load and names the field.
- API 0.6 to 0.7 across every mod.toml and fixture, ui_api untouched. The shared test helpers build the line from API_VERSION.
- A picture-only body's light counts only while it is up, since core's moon's hour curve and its orbit disagree until a9e6b195.
- The two-suns example's lights read their own bodies' up (and the moon its phase), so brightness follows where each stands.
- The bench's moving and dusk views moved to 17:15, where the sun stands about 9° up on the bench's day; at 18:40 it is set.
Criteria:
- (1) grep finds no reader.
- (2) Autotest: a lone wall in the open at 09:00, on the year's highest and lowest days for core's sun: the shadow swings 33°, each season's spot dark and one lit in the other.
- (3) Unit test: a mod moon on a daylight term; the picture's sky light equals the sim's light, and the moon's share is 1. A second test covers core's own picture-only moon. Switch (3) to core's moon in whichever of this and a9e6b195 lands second.
- (4) scripts/task bench locally within budget (worst world 1.054 of 4 ms), dusk sun rebuild 0.171 ms CPU. The queue lane confirms it before merge.

## 2026-09-28

Rebased after a9e6b195 (#337), which landed first. Criterion 3 now runs on core's moon, a daylight term: at the first midnight, full, the sim's light is above 1, the picture's sky light equals it, and the moon's share is 1. The mod-moon helper is gone, since removing core's moon body now breaks core's moon term. The two-suns example keeps quiet-field's moon-off block.
