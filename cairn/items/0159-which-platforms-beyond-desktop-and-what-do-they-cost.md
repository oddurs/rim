---
id: 159
title: Which platforms beyond desktop, and what do they cost?
type: spike
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
priority: p2
api: none
effort: s
layer: engine
area: modding
---

## Question

Desktop (macOS, Windows, Linux, plus Steam Deck) is the committed target. Should web, mobile or consoles be targets, and what must the mod platform look like now so they stay possible? This has to be decided before the SDK and the index (0149, 0153) are built on top of the mod-loading layer.

## Options

- **Desktop only:** keep `std::fs` mod loading and wasmtime; say so in DESIGN.md.
- **Keep web open:** hide mod loading behind a source trait (directory, archive, in-memory), so the loader never calls `std::fs` directly. Known blockers: Luau (C++ inside mlua) doesn't build for macroquad's wasm target; browser fetches from GitHub need CORS; wasmtime can't run in a browser (the WASM tier would use the browser's own engine).
- **Keep mobile open:** touch-first input, and a WASM tier that can interpret instead of JIT (iOS forbids JIT). App Store rules on downloadable mods need checking.
- **Consoles:** no macroquad backends and restricted downloadable mods; likely out.

## Decision


## Acceptance criteria

- [ ] Decision recorded in DESIGN.md
- [ ] If any non-desktop target stays open, the loader constraint is written into 0149 and 0153
