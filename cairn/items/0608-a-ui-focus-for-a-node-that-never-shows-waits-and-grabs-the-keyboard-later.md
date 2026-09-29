---
id: 608
uid: 457314a0-9711-49c8-9974-0d86b10b8cdb
title: A ui.focus for a node that never shows waits, and grabs the keyboard later
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

Unverified in play; core has no call that hits it today. `ui.focus(id)`
parks the id in `Ui::focus_pending` until a node with that id is laid out
(crates/rim_ui/src/lib.rs, end of `frame`). If the node doesn't appear
(the handler asked for an input its panel then chose not to show), the
request never expires: whenever a node with that id appears, minutes later,
it takes the keyboard, and every key goes to it. Fix: a request lasts a
frame or two, as long as it takes the node that asked to be built.
