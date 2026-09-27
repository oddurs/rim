---
id: 90e15c25-52a0-46d3-9011-e0b12fb08a97
title: The docked shell clones every panel's tree each frame
type: perf
status: backlog
milestone: interface
created: 2026-09-26
updated: 2026-09-26
priority: p3
api: none
effort: m
layer: client
area: ui
---

## Budget

## Measurement (before)

## Approach

## Acceptance criteria

- [ ] Benchmark shows the budget is met

`Ui::shell` builds the docked layer's root from clones of every mounted
tree (`group` clones each `Node`), every frame, rebuilt or not. With
layout kept between frames (06014a48), cloning and dropping those trees is
the largest single cost left in a frame's UI work: in a profile of the
pointer crossing a cell every frame on the core UI, `Node::clone` and its
drop outweighed layout. Holding still costs ~0.65 ms of UI CPU a frame;
crossing a cell ~1.05 ms.

Shared children (`Rc<Node>`) or a shell that refers to the mounted
trees instead of owning copies would remove it.

## Acceptance criteria

- [ ] A frame with nothing rebuilt clones no mounted tree
- [ ] Crossing a cell costs within 0.2 ms of holding still on the core UI with 30 colonists
