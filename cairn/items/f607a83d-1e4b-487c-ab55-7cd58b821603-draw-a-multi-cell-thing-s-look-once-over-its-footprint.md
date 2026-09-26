---
id: f607a83d-1e4b-487c-ab55-7cd58b821603
title: Draw a multi-cell thing's look once, over its footprint
type: feature
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 8cf4db07-217d-42f9-aed0-8c119d5acf0c
created: 2026-09-25
updated: 2026-09-26
priority: p2
api: additive
effort: m
layer: client
area: render
---

## Why

A thing can cover several cells (8cf4db07): it is one entity in every cell of its footprint. The renderer draws per cell, so today a 2x1 table would draw as two 1x1 tables. No shipped thing is bigger than one cell yet, so nothing looks wrong yet.

## What

- The chunk mesh draws a fixture only at its anchor (`Thing.pos`), skipping the other cells of its footprint.
- A look's layers are laid out across the footprint (w x h cells), not one cell: primitives scale on two axes, or a look says how it stretches.
- A multi-cell thing that crosses a chunk edge is redrawn when either chunk rebuilds.

## Acceptance criteria

- [x] A 2x1 thing draws once, across both cells (autotest screenshot)
- [x] It crosses a chunk edge without a seam or a double draw
- [x] Render benchmark unchanged
