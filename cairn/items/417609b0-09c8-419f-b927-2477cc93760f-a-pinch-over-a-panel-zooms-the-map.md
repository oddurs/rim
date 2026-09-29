---
id: 417609b0-09c8-419f-b927-2477cc93760f
title: A pinch over a panel zooms the map
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

Unverified on a trackpad. `scroll_camera` (crates/rim_client/src/main.rs)
applies the pinch; it runs when `!out.captured_wheel`, which the UI sets only
when the wheel moved (`input.wheel != 0.0`). A pinch has no wheel, so
pinching over the inspector or a window zooms the map under it.
