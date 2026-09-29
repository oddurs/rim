---
id: 421
uid: e5d8b445-42ed-4e87-8390-ad54d525757b
title: 'Occluders: one texel per cell of height, roof and opening'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 360
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Why

Every shadow in §6e is a march through one small texture. It has to say, per cell, how tall the cell is, whether it is roofed, whether it is a window or a door, and how much it stops sky and flame. DESIGN.md §6e.

## What

- An RGBA8 texture, one texel per cell: R height, G roofed / window / door, B sky opacity, A light opacity. Walls, doors and windows from their defs; trees and canopy from a `height` on the thing def (default by kind); solid terrain and today's rock things alike, so rock-is-terrain (8cc6252d) swaps the source without touching the shader.
- "Roofed" goes through one query. Today it is `map.indoors`; the roof span (24100bb9) replaces it with the per-cell `roofed(z, c)`.
- Rebuilt only when the map's wall, terrain or room revision changes, keyed like `Sky::update_lightmap` today. Once chunks (96d2dac9, on the grid branch) land, only dirty chunks are rewritten.

## Acceptance criteria

- [x] A frame with no map change does no occluder work (test on the cache key)
- [x] Rebuild time on 192 × 192 and 250 × 250 recorded here
- [x] A window, a door and a tree read back with the expected channels (unit test on the packer)

## 2026-09-26

Agreed with Houses: 24100bb9 exposes Map::roofed(i) per cell (and covered(i) for span alone), computed at room rebuild and per level through Depth. The occluder packer calls roofed(i) in place of map.indoors once it lands. df049dac keeps the roof height as a per-cell array rebuilt with rooms; the name comes when it lands. The occluder texture doesn't go through draw.rs's Sink.

## 2026-09-26

Started stacked on 6a6dfe88 (PR #179), which it depends on; the branch rebases onto main once that merges.

## 2026-09-26

Roofed is the sim's indoors(p), not Map::roofed(i). After #170, roofed(i) is covered(i): within a support's span, which includes open ground beside a lone wall. indoors(p) is an enclosed room with every cell covered, so a hall beyond its span is outdoors in the sim, and lighting keeps it outdoors rather than disagree with the light field. A room rebuild repacks only the chunks where a cell's indoor bit changed, found by comparing each cell's packed G with indoors(p).

## 2026-09-26

Review fixes: G is now only the roof bit (0/255), so the linear filter blends a soft half-dim edge where a floor meets its wall, as the old lightmap did, and never into a kind that isn't there. Window and door moved to R's low two bits beside a 6-bit height; passes that read kinds sample cell centres. The chunk key is a new per-chunk Map::fixture_rev (bumped only in set_fixture) with terrain_rev, so hauling, designations and looks don't wake the occluders. Rock goes through fixture_def_at, so its thing's height and boundary apply. A room rebuild checks chunk by chunk and stops at the first cell whose roof moved.

## 2026-09-26

Whole repack (first frame, or invalidated), release, Apple M4 Pro under other sessions' builds: 192x192 0.71-0.78 ms (one run 3.0 ms, load), 250x250 0.75-1.28 ms. Reading each fixture's def in place, not through World::fixture_def_at (which clones the Thing), took it from 7.2 ms. After that only changed chunks repack: a fixture placed repacks its one 32x32 chunk.

## 2026-09-27

Merged in #201; closed in the Proving ground planning PR's cairn cleanup (2026-09-27), since the item was left at doing/review after its merge.
