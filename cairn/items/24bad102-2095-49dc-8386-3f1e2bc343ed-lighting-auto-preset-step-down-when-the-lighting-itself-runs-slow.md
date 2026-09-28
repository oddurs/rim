---
id: 24bad102-2095-49dc-8386-3f1e2bc343ed
title: 'Lighting auto preset: step down when the lighting itself runs slow'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 12fbe8fb-c581-4f2b-81e6-bdd2a2eb72b3
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: m
layer: client
area: render
---

## Why

A player who never opens the settings should still get a lighting preset their machine can carry. The presets (12fbe8fb) shipped without `auto`: the first version watched mean frame time. That mixes in vsync, the sim, the UI and everything else, so a slow sim would have turned the lights down without making anything faster. It was cut in review.

## What

- `auto` in `[lighting] quality`, starting at `medium`.
- It steps down on what the lighting passes cost, not on the frame. GPU timer queries work only on desktop GL 3.3+ that isn't Apple's tile renderer (bench.rs), so elsewhere it needs another signal, or it stays where it is and says so.
- It never steps up mid-game, and it says in the log when it steps.

## Acceptance criteria

- [x] With the lighting slowed artificially, `auto` steps down one preset per window and stops at `low` (test)
- [x] A slow sim with cheap lighting leaves the preset where it is (test)

## 2026-09-28

Done as: auto reads the lighting's own GPU time, from two timer queries a frame (the passes before the world, and the multiply after it). It reads them back four frames later, and only once GL says the result is ready, so no frame waits. A frame whose oldest queries the GPU still holds goes untimed rather than restart them. A window is 3 s and at least 30 frames. It steps down a preset when the window's mean is over 2 ms (an eighth of a 60 Hz frame), and stops at low. Overrides set by hand stay over each preset. A gap in frames longer than a window starts it afresh, and frames timed before the player last chose are dropped. Where GL can't time a pass (Apple's tile GPU, GL ES), auto logs that and stays at medium. On the Mac it reads: 'Apple M4 Pro can't time the lighting on its own, so it stays at medium'. The bench fixes the preset, so its numbers are one preset's.

## 2026-09-28

Review fixes: a pending query is never restarted (a GPU more than four frames behind skips timing instead). Frames from before a choice are dropped. The autotest check can fail: frames read back where GL times, or 'unavailable' and medium where it doesn't. The bench runs a fixed preset. A gap restarts the window. An empty GL renderer name means ask again, not unavailable. Light.setting is private, so a new setting always goes through set(). A frame counts only if both its queries ran.
