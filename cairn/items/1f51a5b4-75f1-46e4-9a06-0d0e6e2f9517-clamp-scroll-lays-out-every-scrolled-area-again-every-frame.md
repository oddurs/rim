---
id: 1f51a5b4-75f1-46e4-9a06-0d0e6e2f9517
title: clamp_scroll lays out every scrolled area again every frame
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: perf
---

Unmeasured. `Ui::clamp_scroll` (crates/rim_ui/src/lib.rs) runs, every
frame, a fresh taffy layout of each scroll area whose offset is above 0
(`Engine::content_height` rebuilds the subtree with `fill`). A scrolled Work
Board or news sheet, or any scrolled window body, pays a full layout of its
content each frame even when nothing changed. Measure with a scrolled board
in the UI bench before changing; the clamp only needs redoing when the tree
hash, the area's size or the offset changes.
