---
id: 694
uid: eea57f29-34f1-4530-af43-22b3749dde3a
title: A colony can be left for the title screen
type: feature
status: doing
milestone: rimos
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p1
api: additive
effort: m
layer: client
area: ui
---

## Problem

Once a colony is open, nothing can take the player back to the title: main.rs runs the title once, then the game loop until the process exits. A lost colony (8db05e27) and the system menu's Leave to title (0e1d71a9) both need a way out.

## Proposal

- `act.leave()`: save and close the game as quitting does, then show the title.
- The App build and the game loop move out of main.rs into play.rs (calm-forest asked for this; main.rs keeps mods, conf and the title→play loop).
- Re-entering the title reuses the Ui, its atlas and the loaded mods: nothing reloads.

## Acceptance criteria

- [ ] Leaving a colony closes it, and the title comes back on the same UI with nothing of the game built (autotest)
- [x] A game closed between logs, as leaving closes it, loads back at the tick it left (test)
- [x] docs/modding/ui.md and the generated API reference list act.leave; the command palette offers Leave to title
