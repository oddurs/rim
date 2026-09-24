---
id: a7da68e2-7934-47e2-ae5d-3d438ebb61f1
title: 'Component kit v2: slider, text input and tables'
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- 01e4d9fe-095a-47d9-b06b-5d2c5099f063
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Why

The work-priorities grid and settings need tables, sliders and text entry.

## Acceptance criteria

- [x] `slider`, `input` (with IME-safe text entry) and `table` (sortable, virtualised rows)
- [x] Snapshot tests and gallery entries like v1

## 2026-09-24

slider and input landed with the text-input item (kit.slider on on_drag, kit.input on the engine's buffer); text entry goes through the client's char events (macquad's get_char_pressed), which is the OS text path an IME composes into, so composed characters arrive as chars rather than key codes. This item adds sorting to kit.table: rows = { record } with columns naming a key or a value function; a click on a header sorts by that column (state id..'.sort', ascending then descending, sort = false refuses); the rows stay virtual through the list node, so a sort of 500 records costs one table.sort per rebuild and builds under 40 nodes. The hand-built count/row(i) mode stays for tables without records. Gallery shows a 500-row sortable table; the test sorts 300 rows both ways and checks the row count stays virtual.
