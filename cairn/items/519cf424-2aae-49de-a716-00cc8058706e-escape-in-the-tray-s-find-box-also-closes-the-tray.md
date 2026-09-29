---
id: 519cf424-2aae-49de-a716-00cc8058706e
title: Escape in the tray's find box also closes the tray
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

Unverified as intended or not. Escape with a text input focused: the
engine blurs it (`Key::Escape`), then, the input no longer focused, the
same frame's "escape" runs `core:escape` (mods/core/ui/toolbar.luau), which
closes the tray. "Escape backs out one step at a time" says the first press
should only leave the find box.
