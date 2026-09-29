---
id: e488f623-1ea9-475b-9e6e-2ed09b8695c6
title: Tab walks focus into scrolled-out rows and map labels
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

Unverified in play. `Ui::route`'s Tab collects every focusable hit on every
layer, with no regard for a hit's clip, so focus lands on rows scrolled out
of view and on anchored-layer labels over the map, with nothing visible
showing where it went.
