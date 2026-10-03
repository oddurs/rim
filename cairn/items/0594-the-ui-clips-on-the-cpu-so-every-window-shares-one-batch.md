---
id: 594
uid: 3442707f-87b2-4ba5-8f35-0ecb7f9bc99e
title: The UI clips on the CPU, so every window shares one batch
type: perf
status: doing
milestone: rimos
assignee: Oddur Sigurdsson
claimed: 2026-09-28
depends_on:
- 380
created: 2026-09-28
updated: 2026-09-29
priority: p0
api: none
effort: m
layer: client
area: render
---

## Problem

The UI draws as one batched mesh, but every clipped node flushes it (draw.rs, Clip/Unclip), and every window and scroll area clips. Six open apps could add a dozen draw calls to today's ~130, of which the UI is 3.

## Proposal

Trim each quad and triangle to the clip rectangle on the CPU as it enters the batch (UVs cut in proportion; triangles by Sutherland-Hodgman), so clipping never breaks the batch. Chosen over a per-vertex rectangle in the shader: no new vertex format or material, and the UI is a few thousand vertices, so the trim costs little.

## Acceptance criteria

- [ ] With six windows open, each with a scroll area, the UI is the same number of draw calls as with none (autotest prints both)
- [ ] Clipped text and rectangles look the same as today in the ui-shots pictures
- [ ] Before-and-after draw calls and UI CPU on the same machine, in the PR

## 2026-09-28

Criterion 1 asked for six windows, but today sheets are one at a time, so the most the core mod can have open is three: a sheet, the palette and the gallery. The check opens those, each with its clip and scroll areas, and asserts no more draw calls than none open. The six-window version is a criterion on 0e773a3e, which makes sheets into apps.

## 2026-09-29

PAUSED (merge freeze): done: clipping on the CPU in UiBatch, both unit tests, full check green (913 tests) after rebasing onto main. The autotest check now counts the UI's own draw calls (draw::ui_calls): with three windows open, calls = flushes for size + 1. The kit gallery alone outgrows one 15k-vertex mesh, which is why whole-frame totals read 12 against 11. Left: rerun the autotest, then the A/B: autotest and render bench on this draw.rs, then on main's draw.rs with the same two counters, back to back on one machine; a screenshot diff; and CI's render bench ui column on the same runner class. Next: that, then ready. Branch perf/3442707f-one-ui-batch, draft PR.
