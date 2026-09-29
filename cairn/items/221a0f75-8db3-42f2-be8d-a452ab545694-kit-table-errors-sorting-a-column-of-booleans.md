---
id: 221a0f75-8db3-42f2-be8d-a452ab545694
title: kit.table errors sorting a column of booleans
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: core
area: ui
---

Unverified. kit.table's sort (mods/core/ui/kit.luau) compares values with
`<`/`>` after converting mixed types to strings; two booleans are the same
type and `true < false` is a Luau error, so a column whose value is a
boolean breaks the table's build when its header is clicked.
