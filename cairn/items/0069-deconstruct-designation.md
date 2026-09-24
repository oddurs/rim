---
id: 69
title: Deconstruct designation
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- 212
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: engine
area: building
---

## Why

Undo building and recover materials. Until now a finished wall was
forever: the only way to move it was to lose it.

## What

- `[[designation]]` gains `targets = "built"`: it applies to things the
  colony built, not to plants or rock. Core names one, `deconstruct`.
- `Job::Deconstruct`: as much work as the thing took to put up (scaled by
  its material, like everything else), then it is gone and `build.refund`
  (default 0.75) of its cost comes back -- in what it was made of. A stone
  wall gives stone.
- `find_work` dispatches on the designation's target kind instead of
  assuming every designated fixture is a harvest.
- Right-click on something the colony built offers it, labelled from the
  designation def.

## Acceptance criteria

- [x] Refund a fraction of cost, in the material it was made of
- [x] Designating a rectangle marks built things only
- [x] Cancel unmarks it and the work stops
- [x] Right-click offers it on an owned building

## 2026-09-23

Pulled into the Building sprint (0210): refunding a deconstructed building means knowing what it was made of, which only exists once 0212 lands MadeOf.

## 2026-09-23

Done. Mechanism: Targets::Built on a designation def -- applies to fixtures with Owner(Player) and a build def, never blueprints (Cancel owns those). Job::Deconstruct takes the thing's scaled build work (stat 'work', so stone takes longer to take down as well as put up), then refunds build.refund (default 0.75) of cost_of(e): the material and count for stuff, the recipe otherwise. find_work now dispatches on the designation's target kind; before, every designated fixture was assumed to be a harvest, and a deconstruct mark on a wall would have spun (run_harvest returns None without a harvest def). Right-click on an owned building offers it, labelled from whichever designation targets built things, and an ordered deconstruct inserts the mark so Cancel can still stop it. Core names it 'deconstruct'; the engine never does. Refund rounds: 5 stone at 0.75 gives 4.
