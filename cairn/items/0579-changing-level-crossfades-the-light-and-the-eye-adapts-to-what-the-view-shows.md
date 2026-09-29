---
id: 579
uid: 220a059e-4927-4f82-ab47-cc8477fd7e08
title: Changing level crossfades the light, and the eye adapts to what the view shows
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 329
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
effort: s
layer: client
area: render
---

## Why

Changing level should not pop: the light of the level left and of the level arrived at should blend, and the eye should adapt to how much sky the view holds rather than jumping. DESIGN.md §6e, Depth.

## What

- Changing level crossfades the two cached levels' light over 150 ms.
- Exposure blends between sky-driven and firelight-driven by how much sky reaches the view, and eases over about a second.

## Acceptance criteria

- [x] Changing level, the mean frame brightness changes by less than 10% per frame during the switch (test)

## 2026-09-28

Done as: changing level keeps the frame the old level drew (the world target, taken at the change) and draws it over the new level's for 150 ms at a falling alpha. It never takes fewer than nine frames, since each frame adds at most a 60th of a second, so a slow frame can't jump it. The eye adapts to the open sky the view holds: the mean of the level's open sky over the cells in view, all of it on the surface. It eases no more than a 60th of a second's worth a frame, for the same reason. Autotest, from the lit surface down to the dark level and back: no frame moved the brightness by more than 8% of the brighter level's (a grab draws two frames, so a frame is half a grab's change). The render-scale check nearly caught a fade left running: tests that set the level directly now wait for its fade.

## 2026-09-28

Review fixes: changing level again mid-fade bakes what the screen shows (the fade over the world target) into the frame that fades out, so it doesn't drop to the last level's frame alone. The autotest now also steps down and straight back up two grabs in, and still no frame moves more than 8%. The eye's easing is by the clock again (1 - exp(-1.6 dt), each frame's dt at most a 30th of a second), so the surface's day and night and lightning adapt as fast as before at any frame rate. The fade keeps its nine-frame minimum, which at 20 fps stretches it to 450 ms. Kept at 10%: the brightness bound, which is the criterion's.
