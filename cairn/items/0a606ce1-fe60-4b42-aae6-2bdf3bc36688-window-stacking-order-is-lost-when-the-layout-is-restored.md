---
id: 0a606ce1-fe60-4b42-aae6-2bdf3bc36688
title: Window stacking order is lost when the layout is restored
type: bug
status: backlog
milestone: rimos
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Problem

ui-layout.toml keeps each window's record in a toml::Table keyed by id, and no crate enables toml's preserve_order, so windows come back sorted by id. The stacking order the player left is lost on restart. This is inferred from the code (rim_ui lib.rs:217-228, 554-592), not yet shown by a test.

## Proposal

Save the stacking order explicitly, as an ordered list beside the records, and restore it.

## Acceptance criteria

- [ ] A rim_ui test opens three windows, raises the first, saves the layout, reloads it, and finds the same order (fails today, or proves the inference wrong)
