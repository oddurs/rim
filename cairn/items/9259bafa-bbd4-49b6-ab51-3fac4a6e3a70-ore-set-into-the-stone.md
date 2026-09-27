---
id: 9259bafa-bbd4-49b6-ab51-3fac4a6e3a70
title: Ore set into the stone
type: feature
status: backlog
milestone: rock-face
depends_on:
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
- db7f1e06-6ca0-4f80-8967-ca3e15b72653
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: client
area: render
---

## Problem

Veins are a per-cell ore amount on a stock field (db7f1e06, d77d9e1f), and nothing draws them. A line drawn across the rock was tried in the concept and read as a cable, not ore. DESIGN.md §6g, "Finding it".

## Proposal

Do what Minecraft does: an ore block is its host stone with ore set into it, and a vein is a cluster of ore blocks.

- A layer primitive `ore`: `{ draw = "ore", field = "core:iron_ore", color = "#d49a78", alt = "#b86e4c" }`. A cell with an amount above 0.05:
  - keeps its kind's fill and pattern, with a cast of `color` at 12% × amount;
  - has one to four nuggets (`1 + round(3 · amount)`) in five slots around the cell's middle, chosen by the cell's hash. Each is a five- or six-sided lump 0.1 to 0.15 of a cell across, with an ink edge, a small shadow and a glint. Every third nugget uses `alt`;
  - has one raw lump 1.7 times the size when the amount is above 0.85.
- Below 10 points a cell, a veined cell is a flat tint of `color` at 25 to 75% by amount, so veins stay findable zoomed out.
- Core colours, chosen to show on any host: iron `#d49a78`/`#b86e4c`, copper `#d07a45` with green `#4fae8a`, tin `#b9bec6`/`#8c929c`, coal `#1d1c20`.
- The reference is `drawOre` in the concept artifact.

## Acceptance criteria

- [ ] A cell's nugget count follows its amount, and mining part of a vein's amount removes nuggets (unit test on the count, shot)
- [ ] The same cell always draws the same nuggets (stable hash of `rock_close`)
- [ ] At 8 points a cell a vein shows as tinted cells (shot)
- [ ] Render bench: whole map within noise with ore on the map
- [ ] `docs/modding/looks.md` documents `ore`
