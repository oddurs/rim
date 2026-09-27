---
id: d860ad76-252c-4d99-b6f7-67d164ca0fae
title: Rows that wrap in the UI engine
type: feature
status: backlog
milestone: mood
created: 2026-09-27
updated: 2026-09-27
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

- [ ] A row of twenty buttons in a 300 px panel wraps and fits (UI test)
- [ ] The Roles lens with fifteen members shows them all inside the card (UI test)
