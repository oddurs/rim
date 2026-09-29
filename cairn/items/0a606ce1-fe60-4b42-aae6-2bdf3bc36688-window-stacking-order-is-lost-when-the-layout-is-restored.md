---
id: 0a606ce1-fe60-4b42-aae6-2bdf3bc36688
title: Window stacking order is lost when the layout is restored
type: bug
status: done
milestone: rimos
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-28
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

- [x] A rim_ui test opens three windows, raises the first, saves the layout, reloads it, and finds the same order (fails today, or proves the inference wrong)

## 2026-09-28

Confirmed by the test before the fix: the restored order came back as declared (a, b, c), not as left (b, c, a). Two causes, not one: the table sorts ids, and restore also rewrote records in place without moving them. The layout now carries order = [ids] at the top; a file without it restores as before.

## 2026-09-29

PAUSED (merge freeze): done: the fix, and the test that failed before it and passes after (rim_ui engine window tests pass). Left: rebase onto main, scripts/task check, ready. Branch fix/0a606ce1-stacking-order, draft PR.
