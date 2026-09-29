---
id: 3442707f-87b2-4ba5-8f35-0ecb7f9bc99e
title: The UI clips in the shader, so every window shares one batch
type: perf
status: backlog
milestone: rimos
depends_on:
- 90e15c25-52a0-46d3-9011-e0b12fb08a97
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: client
area: render
---

## Problem

The UI draws as one batched mesh, but every clipped node flushes it (draw.rs, Clip/Unclip), and every window and scroll area clips. Six open apps could add a dozen draw calls to today's ~130, of which the UI is 3.

## Proposal

Carry a clip rectangle per vertex and discard outside it in the fragment shader, so clipping never breaks the batch.

## Acceptance criteria

- [ ] With six windows open, each with a scroll area, the UI is the same number of draw calls as with none (autotest prints both)
- [ ] Clipped text and rectangles look the same as today in the ui-shots pictures
- [ ] Before-and-after draw calls and UI CPU on the same machine, in the PR
