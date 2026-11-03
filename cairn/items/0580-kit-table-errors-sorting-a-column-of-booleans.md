---
id: 580
uid: 221a0f75-8db3-42f2-be8d-a452ab545694
title: kit.table errors sorting a column of booleans
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
layer: core
area: ui
---

Unverified. kit.table's sort (mods/core/ui/kit.luau) compares values with
`<`/`>` after converting mixed types to strings; two booleans are the same
type and `true < false` is a Luau error, so a column whose value is a
boolean breaks the table's build when its header is clicked.

## 2026-09-28

The same column showed false as a blank cell: kit.table rendered tostring(v or ""). Fixed with the sort, since both are a boolean column not working, and the test checks both.
