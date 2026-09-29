---
id: 63129c59-5eba-4fec-8f14-557e6d34047c
title: A NaN in the saved window layout loses the window for good
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

`Ui::restore_layout` (crates/rim_ui/src/lib.rs) takes x and y as parsed;
`clamp_window` keeps NaN (f32::clamp of NaN is NaN), so a window whose
ui-layout.toml says `x = nan` is laid out at NaN, can't be hit or dragged,
and is written back as nan. Also `ui.window` takes w and h unchecked (NaN or
negative from a mod). Fix: a non-finite position or size falls back to the
declared default.
