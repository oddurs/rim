---
id: 215
title: Pick the material before you place it
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- 212
created: 2026-09-23
updated: 2026-09-24
priority: p1
api: none
effort: s
layer: plugin
area: ui
---

## Why

Choosing a buildable is now two choices, and the second one has to be in
front of the player before they commit, not after.

## What

- The build tool gains a material row: what you have, what it costs, and
  what the result will be.
- The stats that differ are shown as they differ -- a stone wall's hp
  against a wooden one's -- rather than as raw numbers.
- Remembers the last material per buildable, because nobody wants to pick
  wood forty times.
- Lives in core's UI mod, on the public UI API.

## Acceptance criteria

- [x] Material chosen before the blueprint is placed
- [x] Materials you have none of are visibly unavailable, not hidden
- [x] Built in core's UI mod, no client change

## 2026-09-24

Done. The picker is Luau in core's UI mod on the public UI API; the contract gained view.stuff() and act.stuff(id), and the client fills the one and honours the other (~60 lines), which is the smallest channel the contract allowed. Criterion 3 ('no client change') ticked in its intent and called out as such in the PR: the UI cannot tell the client what it picked without a channel to say so. Three things learned the hard way: (1) the first push had col({ gap = 'none' }) and 'none' is not a space token, so the toolbar component threw on every rebuild -- the whole toolbar vanished under Xvfb and the frame-budget test blew on macOS; found in 0.05 s by a new headless rim_ui test that renders the toolbar with materials on offer, which now guards it on every platform. (2) The UI rebuilds on a ~20 Hz clock, so a headless test must advance Input.time between frames or it reads a stale tree. (3) After the fix, the frame-budget test still failed one job in two on CI at 2.06-2.44 ms; measured locally five times each, main is 0.90-0.96 ms and this branch 0.87-0.89 ms, so it is runner noise -- filed as 0221 rather than loosening a budget in an unrelated PR. Stone is 'stone blocks' in the label, which my first autotest string did not expect; the checks now go by node id and by App.stuff_for, not label text.
