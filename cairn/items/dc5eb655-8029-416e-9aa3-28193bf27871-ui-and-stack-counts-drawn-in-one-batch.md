---
id: dc5eb655-8029-416e-9aa3-28193bf27871
title: UI and stack counts drawn in one batch
type: perf
status: done
milestone: shelter
created: 2026-09-24
updated: 2026-09-24
priority: p2
api: none
effort: s
layer: client
area: render
---

Draw the UI as meshes textured by the glyph atlas (a reserved white block for shapes), and world stack counts with the UI's text, so a clip region is one draw call.

## 2026-09-24

PR #22. HUD open at normal zoom: 72-78 -> 5 draw calls; zoomed all the way out 68 -> ~10. The autotest counts draw calls and times render() itself (the old frame time included the vsync wait).
