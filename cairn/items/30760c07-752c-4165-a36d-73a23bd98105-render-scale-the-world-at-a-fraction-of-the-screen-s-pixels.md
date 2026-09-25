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

- [x] World renders at a chosen scale while UI text stays at native resolution
- [x] The setting persists and applies without a restart
- [x] Bench shows the GPU pass cost at 50 % vs 100 %
- [x] Autotest screenshots at 50 % reviewed

## 2026-09-24

The world (ground, meshes, pawns, weather, light) draws into a render target at the scale and is blitted with one quad before tool previews and the UI, which stay native. The chunk shader flips y into a target, as macroquad's camera does. At 100% there is no target. The default is 1/DPI above 1.5x (logical pixels); the autotest and bench pin 100% unless a view asks. Palette-only binds (no key) came with it: rim_ui's ui.bind accepts an empty key, which never matches a press or conflicts. settings.toml holds render_scale; a value that doesn't parse is a warning, not a silent default.

## 2026-09-24

CI, llvmpipe, storm view at 100% vs 50%: glFinish 69.4 → 15.7 ms, submit 45.7 → 68.5, so GPU plus submit 115 → 84 ms; world CPU the same once the camera switch and blit count as GL submission (they flush the target's batch, where a software rasteriser draws).
