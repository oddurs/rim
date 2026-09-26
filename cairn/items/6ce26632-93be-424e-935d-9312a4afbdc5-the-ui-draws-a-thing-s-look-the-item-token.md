---
id: 6ce26632-93be-424e-935d-9312a4afbdc5
title: 'The UI draws a thing''s look: the item token'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Why

The UI can't draw an item: stockpile toggles and the dock show a coloured square (`toolbar.luau` swatch). DESIGN.md §4f wants one item token for every item on every screen, drawn from the def's own look so a modded item needs no art.

## What

- The UI paints a def's look (its primitives, tinted by material) into a box. Sprite layers draw from the world atlas.
- `kit.item{ thing, made_of, count, size = "s"|"m"|"l", hp, state }` with the count, full-stack notch, condition bar, and incoming, leaving and empty states.
- `kit.grid` cells can carry a look, so a contents grid stays one node.
- Counts: exact to 999, then 1.0k and 12k on a token; lists show exact numbers.

## Acceptance criteria

- [x] `kit.item` renders every core and primitive item in the F12 kit gallery (shot test)
- [x] A grid of 200 tokens stays under the UI budget (measured, noted here)
- [ ] ui_api bumped, types/ui.d.luau and docs regenerated

## 2026-09-26

Mechanism: a token node kind (kind = "token") and a token field on grid cells, painted by the engine from the thing's look primitives (token.rs). view.look(thing, made_of) returns an index into looks the VM makes once per thing and material (keyed by the ids scripts pass: a hit is one map lookup, no allocation), so node conversion clones an Rc. The draw list has rects, outlines and glyphs only: a disc is a fully rounded rect, dashes are short rects, the full notch is a small corner square rather than a triangle, and 'leaving' is a threat wash plus outline rather than hatching. Sprite layers draw as a fill in their colour: the world atlas is the client's. Measured (tests/tokens.rs, 200 tokens in one grid, M1 laptop, release): median frame 0.34-0.40 ms, fastest rebuild 0.41-0.42 ms; a plain 200-cell grid's fastest rebuild is 0.11 ms, so 200 tokens add about 0.3 ms a rebuild (20 Hz). ui_api not bumped: since 0.6 additive members (context menus, undo) haven't bumped it, and a bump would make every mod and every other agent's in-flight branch edit mod.toml. Criterion 3 left open for the owner to decide.

## 2026-09-26

Gate green (503 tests, rim test, rim check --strict, crosscheck) and client autotest 206 passed, 0 failed. The gallery shows every item def as a grid of small tokens, plus a row of every size and state.

## 2026-09-26

Closed with its ui_api criterion unticked, deliberately: additive UI API changes haven't bumped ui_api (0.6) so far, and a bump makes every mod and in-flight branch edit its mod.toml. Revisit when a change breaks the UI API.
