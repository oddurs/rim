---
id: 534
uid: d860ad76-252c-4d99-b6f7-67d164ca0fae
title: Rows that wrap in the UI engine
type: feature
status: done
milestone: mood
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: additive
effort: m
layer: engine
area: ui
---

## Why

UI rows never wrap. The work milestone worked around it three times: the Person lens's shelves are sized to fit core's seven work types, the Roles lens's member chips come seven to a line by hand, and the settler dialog splits its role buttons over two rows. A mod with more work types or roles overflows each of them.

## What

- `wrap = true` on a row lays its children in lines within its width (flex-wrap), with the row's gap between lines.
- Core's shelves, role cards and the settler dialog use it instead of fixed chunks.

## Acceptance criteria

- [x] A row of twenty buttons in a 300 px panel wraps and fits (UI test)
- [x] The Roles lens with fifteen members shows them all inside the card (UI test)

## 2026-09-27

wrap on a row maps to taffy's flex_wrap; the gap between lines is the row's gap (taffy's row gap was already set to it). The flag is in the layout hash and in Style, so the kept tree's sync sees a toggle as a style change: the kept-tree test gained wrap, unwrap, rewrap steps. A row needs a width to wrap at (from its parent, grow, w or maxw). Core's settler dialog caps its role row at maxw 340. Two more hand-split rows remain outside the work screens: storage.luau:410 and zones.luau:108 ('Rows of a few, since a row doesn't wrap').
