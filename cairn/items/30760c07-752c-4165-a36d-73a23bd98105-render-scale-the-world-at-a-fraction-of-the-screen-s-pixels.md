---
id: 30760c07-752c-4165-a36d-73a23bd98105
title: 'Render scale: the world at a fraction of the screen''s pixels'
type: feature
status: doing
milestone: graphics
assignee: Oddur Sigurdsson
claimed: 2026-09-24
depends_on:
- 4376f91e-f671-4ca5-9a6a-8c42848043eb
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

`high_dpi` is on and nothing can turn the world's resolution down. On a
Retina or 4K screen every full-screen pass (ground, lightmap, weather) costs
four times the pixels, and pixels are where an integrated GPU runs out.

## What

- The world (ground, things, pawns, weather, light) draws into an offscreen
  target at a render scale, then one quad scales it to the screen. The UI
  draws after, at native resolution, so text stays sharp.
- A setting: 50–100 %, in steps. The default is 100 % unless the screen's
  DPI factor is above 1.5, where it defaults to 1 / DPI factor (logical
  pixels).
- At 100 % no offscreen target is used, so the default path costs nothing.
- The bench reports frame time at 100 % and 50 %.

## Acceptance criteria

- [ ] World renders at a chosen scale while UI text stays at native resolution
- [ ] The setting persists and applies without a restart
- [ ] Bench shows the GPU pass cost at 50 % vs 100 %
- [ ] Autotest screenshots at 50 % reviewed
