---
id: 2f13e01d-abee-46ca-ace1-5770c8f7feac
title: 'Indoors: sunbeams through windows, room fill, and exposure'
type: feature
status: doing
milestone: lighting
assignee: Oddur Sigurdsson
claimed: 2026-09-26
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
created: 2026-09-26
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

A torch in a hut and a torch in a field should look different, and a window should visibly let the day in. This is §6c's daylight fan and warm wash, done as light. DESIGN.md §6e.

## What

- Under a roof, a sky ray passes only through a window between sill and lintel: sunbeams, long at dusk, a strip at noon, none through a north wall.
- Room fill: `0.3 · Σ(I·r²) / area` per room, flat over its cells, in each light's channel. Computed when rooms rebuild.
- A room's diffuse sky share from its boundary `daylight`, which room rebuild already sums.
- Exposure follows the sky and eases over a second; `max(sky, fire)` goes.
- Contact shadows at wall bases from four occluder reads (`medium` and up).

## Acceptance criteria

- [x] A hut with a west window shows a beam on the floor under a low western sun and none under a high southern one (autotest reads the sun back)
- [x] One light lights a 3×3 hut to its corners and leaves the corners of a larger hall dim (autotest reads the firelight back)
- [x] Room fill rebuilds only with rooms (test)
- [x] Across a sweep of sun angles, no roofed texel sees the sky except along a ray through a window: a ray from under a roof stops at any wall or door whatever its height (readback test)

## 2026-09-26

Agreed with Houses: the window daylight fan and the fire's warm wash belong to this item. 4791e24b drops both and keeps only the gap marker and the open-sky hatch. The plan draws nothing for either, so there's no second fan to replace.

## 2026-09-26

Found in the concept demo: the indoor ray blocked at a wall only while the ray was below the wall's height. A ray that reached a wall exactly at roof height slipped between the roof and the wall top, which lit a thin band of full sun at a fixed distance (wall height / tan elevation) from every wall. It showed on 141 of the daylight hours sampled at the low preset. The roof rests on the walls, so a ray from under it must stop at any wall or door. The new criterion tests for this.

## 2026-09-27

Done as: under a roof, a sun ray passes only through a window between sill (0.28) and lintel (0.92), and stops at any wall or door (the rule the concept demo got wrong). Room fill (0.3 x sum of strength x reach^2 / area, at most 0.4) is baked into the firelight target as flat per-cell quads when rooms or lights change. A room's sky share is [[sky]] indoor_share (core now 0.2: walls and door) plus its windows' light pass, as room rebuild sums it, so a windowless hut is dim by day and windows visibly matter. Firelight adds to the sky rather than taking the max, and exposure (1 by day, up to 2.6 on a moonless night, eased) scales both. A soft shoulder above 0.9 keeps a fire's heart from clipping flat; a knee at 0.75 tipped a Houses door-jamb check, which now compares the jamb with the lit wall beside it rather than the wood's unlit colour. Criteria reworded to what is tested: the sun is pinned rather than set by the clock, and the hall is 5x5, since a 12x10 ring of walls isn't a room under the roof span. The contact shadows at wall bases in What were dropped: the plan's contact band (0779def9) already darkens every mass's base. Autotest: beam 0.72, none from the south 0.00, a windowless room 0.00 from 36 sun positions, a stove lights a 3x3 hut to 0.93 of its middle at the corners, a 5x5 hall 0.28.

## 2026-09-27

Review fixes: (1) the indoor sun rule is now 'out only through a window pane': any other exit stops the ray, so a room closed by deep water rather than wall no longer lets the sun in (autotest hut with a water gap; it read 1.00 before the fix, 0.00 after). (2) The indoor term samples the sun at its own texel, so the linear filter can't carry a wall's sunny outer face onto the floor. (3) The rooms texture is checked every frame, keyed by room rebuild and each room's share, since the field sums windows after the map rebuilds; before, it only refreshed when firelight rebaked. (4) Room fill is compared as data (cell and value): a room rebuild that leaves every fill alone no longer rebakes firelight, and partial bakes reuse the cached fill quads. The Lamp.indoors special case went with it. Autotest: a wall far from any light rebuilds rooms with 0 bakes; opening the lit hut rebakes and its corner drops 0.36 to 0.27. (5) The tone shoulder is gone and Houses' jamb check is back as it was: the shoulder changed wall colours for no visible gain. (6) The hall check is relative, corners at most half the hut's. (7) indoor_share's built-in default is 0.2, matching core.

## 2026-09-27

Before the PR: the far-wall check rebuilds rooms by hand instead of ticking the sim, since a tick could finish building a fire and rightly rebake. It also picks a wall whose whole chunk is out of every light's reach, since occluders report changed chunks until 4b6c3a9c. Light::adapt_now lets a test's eye settle at once: the sun section pinned daylight straight after the storm's night, and exposure still easing from 2.6 pushed the multiply past white, where the contact shadow can't show. That was the occasional 0.91.
