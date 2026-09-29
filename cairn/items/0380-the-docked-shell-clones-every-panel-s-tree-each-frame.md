---
id: 380
uid: 90e15c25-52a0-46d3-9011-e0b12fb08a97
title: The docked shell clones every panel's tree each frame
type: perf
status: done
milestone: interface
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
effort: m
layer: client
area: ui
---

## Budget

Holding still clones no mounted tree; crossing a cell costs within 0.2 ms of holding still (the criteria below).

## Measurement (before)

`cargo test --release -p rim_ui --test shell_cost -- --ignored --nocapture` (core UI, 30 colonists, 480 frames, medians), alternated with the change, at load average ~50: holding still 0.455 / 0.479 ms, crossing a cell 1.015 / 1.022 ms.

## Approach

`Node.children` is an `Rc<Vec<Node>>`, so cloning a node copies the node, not its subtree; the shell and the layers keep cloning roots every frame, but that clones one node each. Building still mutates through `Rc::make_mut`, which copies nothing while a tree has one owner.

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

- [x] A frame with nothing rebuilt clones no mounted tree
- [ ] Crossing a cell costs within 0.2 ms of holding still on the core UI with 30 colonists

## 2026-09-28

After, same run: holding still 0.367 / 0.433 ms (about 15% less); crossing a cell 0.987 / 1.118 ms, no change beyond the noise. The harness splits a frame: crossing adds ~0.6-0.8 ms of layout (layout_us), not cloning, so criterion 3 isn't met by this and stays open. That cost is the hover readout relaying out; filed separately. Criterion 2 holds by construction (Node clone shares children) with a unit test (node.rs cloning_a_node_shares_its_subtree).

## 2026-09-28

Closed with criteria 1 and 3 unticked on purpose. Sharing the trees met the still half of the budget (no clone when nothing rebuilds; 0.46-0.48 ms to 0.37-0.43 ms). Crossing a cell didn't move, and the measurement puts that cost in layout, not cloning: that is 852445ff's criterion now, measured with this item's tests/shell_cost.rs.
