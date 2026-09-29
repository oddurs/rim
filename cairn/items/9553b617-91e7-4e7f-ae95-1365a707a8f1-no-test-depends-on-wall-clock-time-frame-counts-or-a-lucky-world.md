---
id: 9553b617-91e7-4e7f-ae95-1365a707a8f1
title: No test depends on wall-clock time, frame counts or a lucky world
type: chore
status: doing
milestone: bare-metal
assignee: lucky-harbor
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: tooling
area: tests
---

## Why

Main went red ten times in two days, mostly from tests tied to real time or to how a world happened to turn out: the founder dying in a storm, the ruler's alert phase, box-select counts, the fade tolerance.

## What

- Audit rim_client's autotest and rim_sim's tests. Any check that waits on frames or real time, or assumes who is where, either pins its world (weather, spawns, hp) or counts what it needs.
- The autotest runs twice on different seeds in the nightly, and a check that fails on one seed only is a flake to fix.

## Acceptance criteria

- [ ] Every autotest section pins or counts what it asserts, audited and listed in a note
- [ ] The nightly autotest passes on three seeds
- [ ] No test failure in 20 consecutive queue-lane runs that the change under test didn't cause

## 2026-09-28

Audit, 2026-09-28, main b90b069e.

The world is deterministic per seed: autotest frames never step the sim (RawInput::default() has advance false), so it moves only by t.ticks(n), in ticks. What varies by machine is the render side and the waits on it. The autotest drives a synthetic clock (T::input adds 1/60 s to raw.time), but the client reads the real one in 15 places: the level fade and the eye's exposure (get_frame_time), the firelight bake's once a second and its flicker (light.rs, get_time), sky flashes and particles (sky.rs), water, chunk animation (draw.rs:524), and the order toast (main.rs last_order). The fade tolerance flake (#307) is this.

Fix 1, the root (main.rs, light.rs, draw.rs, sky.rs; owners agreed): one frame clock. frame() sets app.now from raw.time and app.dt from the step since the last frame; render() and everything it calls read those. The live game's raw.time is macroquad's; the autotest's goes 1/60 s a frame. A guard fails any get_time or get_frame_time in rim_client outside the input builder and measurement.

Then per section (in the per-section files, after 1eabaea8 splits autotest.rs):
- harness light_settles: 10 s of wall clock -> a frame count, once the bake runs on the frame clock.
- 5a69f9c9 moving lights: bakes counted against get_time(); count against the frame clock.
- 24bad102 lighting auto: waits AUTO_WINDOW seconds of wall clock; wait that many frame-clock seconds.
- weather: the flash fading within 120 and 60 frames on sky.rs's real dt; exact on the frame clock.
- 220a059e level fade: a frame's share becomes exact on the frame clock.
- 0047 orders: t.ticks(600), then 'the colonist walks there', for a cell up to 30 away, and snow and mud (#277) slow the walk; t.ticks(240), then 'the attack lands'. Deterministic on seed 7, a lucky world on others: tick until arrived or hurt, bounded.
- warrior loop (autotest.rs:959): 12000 ticks for a wall to stand, counted already; watch on other seeds.
Counted or pinned already, no change: toolbar, camera, clicks and screenshots (the world holds still between ticks), stances (alone is read, not assumed), profiler, sun shadows, firelight, indoors, roofs take the sun, grid and motion (chalk reads raw.time), several selected, urgent hunt, unreachable, water.

Outside the autotest: rim_sim, rim_ui and rim_client unit tests read no clock except behind RIM_BUDGETS or in no_clocks' PRINTS_ONLY; the guard holds. rim_ui's two reload tests sleep 20 ms so a file's mtime changes, safe on APFS and ext4. rim_sim tests pin their seeds: deterministic, and they break when content moves a map (seed 5, #306), not at random; that is d1f37e29.

Nightly: runs the autotest once, on seed 7 (ci.yml:374). Criterion 2 needs seeds added there; the merger owns CI.

## 2026-09-28

The frame clock (this PR): two autotest runs on the same machine drew 69 of 92 screenshots differently on main b90b069e, and 8 of 92 with the frame clock. Of those 8, five show measured timings (the profiler, and one devtools line in four shots), which differ by design. Three are chunks rebuilt under mesh.rs's ZOOM_BUDGET_US, a wall-clock budget, at a new zoom (figures at 7 and 15, roofs at 8): silver-field's file, told. 396 of 396 checks passed in all four runs. On main the auto preset's wait was wall time, so the frames drawn there and everything after them varied from run to run. The auto wait and the moving-lights bake count now run on the autotest's clock, and the guard covers the autotest too.
