---
id: 646
uid: 8b30fc43-cb86-4cd1-8a6b-11ef3e9cf121
title: The render bench answers the window while it builds its world, instead of beachballing
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-10-02
created: 2026-09-28
updated: 2026-10-02
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
- [x] The game's own new-game load pumps events too, or is measured and shown not to stall

## 2026-09-29

PAUSED: not started. Next step: make bench::world async with a drawn frame between its steps (mods loaded, mapgen, the colony stamped), and the same inside run() around the stacked dig's 2,000 sim steps; main.rs's call gains .await (calm-forest's file: tell them). Wait for a1fe6816-B to merge first: both edit bench.rs. Owner: rapid-cloud.

## 2026-10-02

Measured (M4 Pro, load 14-21, release, window in the background, which doesn't slow setup): bench::world 438-541 ms (Sim::build 59-84 ms of it); a new game's Sim::new 37 ms. The beachball was the stacked view's 2,000-step dig: 114-120 s, 58.4 ms a tick, 58.2 of it in the pawns system. A sample put 97% in ai::choose_work -> World::nearest_tool: every job that needs a tool looked over every tool with a reach test, so with the colony's tools claimed or out of reach, each candidate tree or plan paid a full scan, every choice. Fix: choose_work asks each need once (the world doesn't change while a colonist chooses; same answers, so determinism holds). After: the dig 4.4 s, 2.19 ms a tick (pawns 2.0), 10,729 tool scans in 2,000 ticks. The dig now yields a frame every 100 steps so the window answers. tests/tools.rs a_claimed_axe_is_looked_for_once_a_choice_not_once_a_tree counts World::tool_scans: 30 looks for 30 trees before, 1 after. Criterion 2: the new-game load is 37 ms, nowhere near a stall. Criterion 1 needs the window in front, by eye: asked through rim-c2.
