---
id: 220a059e-4927-4f82-ab47-cc8477fd7e08
title: Changing level crossfades the light, and the eye adapts to what the view shows
type: feature
status: backlog
milestone: lighting
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: s
layer: client
area: render
depends_on:
- 3124bd7b-9f9a-4d92-84e6-2df736b6e2fe
---

## Why

Changing level should not pop: the light of the level left and of the level arrived at should blend, and the eye should adapt to how much sky the view holds rather than jumping. DESIGN.md §6e, Depth.

## What

- Changing level crossfades the two cached levels' light over 150 ms.
- Exposure blends between sky-driven and firelight-driven by how much sky reaches the view, and eases over about a second.

## Acceptance criteria

- [ ] Changing level, the mean frame brightness changes by less than 10% per frame during the switch (test)
