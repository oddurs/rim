---
id: 311
uid: 1a17d685-c3fb-41ba-b3b2-3de0d626c7aa
title: 'Moons and planets: [[sky_body]] gives a body a path, colour and softness'
type: feature
status: done
milestone: lighting
assignee: Oddur Sigurdsson
depends_on:
- 379
- 209
created: 2026-09-26
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: additive
effort: m
layer: client
area: render
---

## Why

"A mod can add a second sun and a green moon" (DESIGN.md §4c) only matters if they light the world: their own colour, their own shadows, crossing the sky on their own path. 0209 (ebb814ad) gives them brightness cycles in the sim; this gives the renderer the rest. DESIGN.md §6e.

## What

- `[[sky_body]]` in defs: `term` (the labelled daylight term that sets its brightness), `rise`, `set`, `peak_elevation`, `arc`, `colour`, `angular_size`, `shadows`.
- Core declares the sun and a moon whose brightness follows 0209's cycle (phases).
- The shadow budget goes to the brightest bodies each rebuild.
- `docs/` example: a green-moon mod.

## Acceptance criteria

- [x] A test mod adding a green moon lights the night green and casts its own shadows, with no engine or client change (autotest screenshot)
- [x] With one shadow slot, the sun casts by day and the moon by night

## 2026-09-28

Done as:
- [[sky_body]] defs: field, term (the labelled term that is its brightness), rise, set, peak, arc, color, angular_size (softness over the sun's 0.5°) and shadows. The engine checks the field has the term. Core declares a sun and a moon.
- The moon is a new daylight term, 1.5 at most, full every 15 days and on the first night, so night light is no longer 0. Plants on a [[0,0],[30,1]] curve grow at most 5% at night; the sim runs and soaks pass.
- A sky with no bodies keeps its lone sun; beside bodies it's ignored, with a warning.
- The renderer: each body's share is its term over its field's terms (evaluated directly, so pins and pushes don't hide it). The sun pass marches up to four bodies into the RGBA channels, brightest straight light first, as many as the preset's sky_shadows (1, 1, 2, 4), chosen each rebuild. The multiply lands each slotted body's light in its colour; a body without a slot adds its colour to the ambient.
- A pinned sun (tests) is a plain white sun with all the sky's light.

## 2026-09-28

Review fixes:
- Roofs take the bodies' colours.
- The contact shadow is cleared by a body only as much as its light is the day's (wholly at 10% light), where it had been normalised so any faint moon cleared it (unit test, fails the old way).
- The rebuild key names which bodies hold the slots.
- A body must be half a degree up to take a slot.
- Bodies are worked out once a frame.
- A lone sun has its own id, not 0.
- A wide body's softness is applied after the key, so cloud drift doesn't rebuild it more often.
- The autotest's green moon goes to any midnight and puts the clock back.
Declined: roof slopes under a moon get the moon's relief as they would the sun's; that is the moon's light.

## 2026-09-28

Autotest (screenshot 64_green_moon). A green moon mod's defs are applied live, the way the loader applies them: its daylight term is compiled with Terms::compile, its [[sky_body]] resolved with the loader's resolve, and the defs swapped in with nothing in engine or client naming it. At the next midnight on medium's one shadow slot, it takes the slot over core's moon. It casts its own shadow (0.00 behind the wall, 1.00 seven cells on). The view's green share rises from 0.346 to 0.375. The clock only moves forward: setting it back broke the unreachable-jobs section after it. A sim test loads the same kind of mod from disk (sky_bodies.rs). The unit test for criterion 2 uses core's real sun and moon: with one slot, the sun at noon and the moon at midnight.

## 2026-09-28

Decided on the PR: the moon is the picture's alone. A body's brightness is either a field's term (the sim's light too) or terms of its own that only the renderer reads. Core's moon now has its own terms, cloud dimming included, and daylight is the sun's alone, so the sim's night light is 0 again and the climate and fields tests assert that again. The picture's sky light is the sim's light plus those bodies'. The two-suns example removes core's moon, and its green moon still chooses to light the sim.
