---
id: 646
uid: 8b30fc43-cb86-4cd1-8a6b-11ef3e9cf121
title: The render bench answers the window while it builds its world, instead of beachballing
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-29
priority: p3
api: none
effort: s
layer: client
area: perf
---

## Problem

On the Mac, `rim --bench-render` builds its 250×250 world, 200 pawns and the stamped colony before its first frame, and pumps no window events meanwhile, so macOS shows the beachball over its window for the whole setup (seen by the user during the 2026-09-28 baseline). It measures nothing wrong, but it looks like a hang, and a player who meets the same stall at a big load would think the game froze.

## Proposal

Draw a frame (a "building the bench world" line is enough) between the setup's steps, so events are pumped: after the mods load, after mapgen, after the colony is stamped. Check whether the game's own load has the same stall at the default map size.

## Acceptance criteria

- [ ] A bench run on the Mac shows no beachball during setup (by eye, window in front)
- [ ] The game's own new-game load pumps events too, or is measured and shown not to stall

## 2026-09-29

PAUSED: not started. Next step: make bench::world async with a drawn frame between its steps (mods loaded, mapgen, the colony stamped), and the same inside run() around the stacked dig's 2,000 sim steps; main.rs's call gains .await (calm-forest's file: tell them). Wait for a1fe6816-B to merge first: both edit bench.rs. Owner: rapid-cloud.
