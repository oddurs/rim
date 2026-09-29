---
id: 620
uid: 519cf424-2aae-49de-a716-00cc8058706e
title: Escape in the tray's find box also closes the tray
type: bug
status: backlog
milestone: bare-metal
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

## Proposed status: doing -> dropped (Oddur Sigurdsson, 2026-09-28)

With 2c567482, Escape from a text box backs out of what the box belongs to: the palette's query closes the palette, and the tray's find box closes the tray. Leaving the box alone would take two presses to close either, which the palette can't afford. Consistent as it is.
