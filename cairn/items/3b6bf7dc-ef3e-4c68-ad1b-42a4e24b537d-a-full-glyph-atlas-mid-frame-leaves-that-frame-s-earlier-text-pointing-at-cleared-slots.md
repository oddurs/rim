---
id: 3b6bf7dc-ef3e-4c68-ad1b-42a4e24b537d
title: A full glyph atlas mid-frame leaves that frame's earlier text pointing at cleared slots
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-29
priority: p3
api: none
layer: client
area: ui
---

Unverified visually. `Text::slot` and `place_image` (crates/rim_ui/src/text.rs)
clear the whole atlas when a new glyph doesn't fit and carry on. Quads
already emitted earlier in the same frame hold UVs into the cleared atlas,
which the next glyphs overwrite: one frame of wrong glyphs for everything
painted before the clear. Rare (a big CJK or emoji burst). Fix: on a clear,
re-run the frame's paint, or grow the atlas instead.

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/3b6bf7dc-atlas-clear-mid-frame and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
