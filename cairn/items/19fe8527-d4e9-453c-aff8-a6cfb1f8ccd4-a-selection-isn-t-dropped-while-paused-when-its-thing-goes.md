---
id: 19fe8527-d4e9-453c-aff8-a6cfb1f8ccd4
title: A selection isn't dropped while paused when its thing goes
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

Unverified in play. `step` (crates/rim_client/src/main.rs) prunes a selected
pawn or thing that is gone, and a selected zone that was removed, only after
the early return for a paused game (`if app.paused || colony_lost { return }`).
Orders are applied while paused (`apply_pending`), so cancelling a selected
blueprint, clearing a selected zone, or devtools' advance leaves
`app.selected`/`selected_zone` naming nothing until the game runs. The
inspector copes (view.thing is nil) but the selection chalk, the order hint
and Tab's position use the stale id.
