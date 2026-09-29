---
id: 308
uid: 15a4e81b-45b0-4970-97a4-24a47430da2f
key: lighting
title: Lighting
type: milestone
status: done
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: additive
---

Shadows from the sun, the moons and every wall; torches that flicker and fill their rooms; sunbeams through windows. The sim's `light` comes from where the sun and moon are (a full moon lights the night a little, the user's decision on 2026-09-28); the rest is the renderer's, computed at the rate each kind of light changes. Design: DESIGN.md §6e.

## Goal

On the default install at the `medium` preset, a dusk colony shows long shadows from walls and trees, a sunbeam through each west window, torches that flicker out of step, and a hut lit to its corners by one brazier. `rim --bench-render` reports every lighting pass with CPU and GPU time, inside the §8 budget on the reference machine. Adding 500 torches doesn't change the per-frame cost.

## Order

1. **Measure first.** Every lighting pass in the render bench, with today's `Sky::light` as the baseline.
2. **Occluders.** One texel per cell, rebuilt with rooms.
3. **Sun shadows**, replacing `Sky::light`, once the one-light spike (0779def9) is decided.
4. **Firelight**, baked into flicker channels.
5. **Indoors:** sunbeams, room fill, exposure.
6. **Dynamic lights**, presets, and resolution that follows zoom.
7. **Roofs, levels, moons:** each waits on the milestone that brings the thing it lights.

## What this waits on elsewhere

- Houses (2f7e95c6): the roof span (24100bb9) makes "roofed" per cell; roofs from far away (df049dac); the one-light shadow (ae5c3807) is reconciled by this milestone's spike.
- Depth (e58c8ff7): positions gain z (e311c029), the view (5689930d), underground (f2a8ffc7).
- Plugin API: sky bodies (ebb814ad) for moons and planets that mods add.
- Rock is terrain (8cc6252d) changes where solid cells come from; the occluder pass reads both until it lands.
- Items 1–6 need none of these and can start now.

## Not in this milestone

- Light in the sim: shadows never reach gameplay (§6e). Darkness limiting sight stays in Defense (713009ac).
- Coloured light from mods beyond four flicker channels: overflow joins the nearest colour.

## Where it sits

Decided 2026-09-26: Lighting runs alongside Houses and ahead of Depth. Its first six items touch only `sky.rs` and new client passes, not the chunk mesher Houses is changing. The contact shadow (0779def9) now lives in its compose pass, so Houses ships walls with a highlight and gets the shadow when Sun shadows (8f4f1de8) lands. Depth is a large breaking change to positions everywhere; lighting's per-level item waits for it, and the rest shouldn't. Due date: not set, since agents only read `due`.

## 2026-09-28

Reopened 2026-09-28: the user wants the sun and moon simulated in full, not picture-only. Items: 0a27bfbb-276b-4b28-a7cc-1e02a3f8dc91 (engine), a9e6b195 (core content), efd56e49 (renderer).

## 2026-09-28

Closed at 22 of 22, the three sky items included: 0a27bfbb #328, a9e6b195 #337, efd56e49 #339. The goal's GPU time on the reference machine is still unmeasured: Apple can't time a pass, and CI's llvmpipe is the CPU. It's on the user's manual list (rim --bench-render on an immediate-mode desktop GPU, ultra included), with the auto preset's budget and the moonlight feel check.
