---
id: 556
uid: 02119d95-29db-46ac-8130-0a11c974f072
title: types/ui.d.luau's Node type lacks props the engine takes
type: docs
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: docs
---

Unverified in an editor. `UI_TYPES` in crates/rim_ui/src/api.rs declares
`type Node` without `menu`, `at`, `on_hover`, `on_outside` and the token
fields (`look`, `count`, `full`, `hp`, `state`), all of which
`node_from_table` (crates/rim_ui/src/node.rs) accepts and docs/modding/ui.md
documents. Node has a `[number]: any` indexer and no string one, so a strict
luau-lsp flags `menu = {...}` in core's own inspector.luau as an unknown
field. The api_types test checks the ui/act/view members, not Node's fields.
Fix: add them, and have the test compare Node's fields with the properties
node_from_table matches.
