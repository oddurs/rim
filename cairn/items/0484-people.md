---
id: 484
uid: 5d09b04e-2cfa-4fca-bfc9-474289138e4d
key: people
title: People
type: milestone
status: backlog
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
---

People seen from above: a pawn drawn as a plan figure in the plan's own ink,
walking with feet and hands that trade places, rounding corners, carrying,
working and lying down, in three levels of detail and one draw call. What a
person looks like is data a mod can change. Design: DESIGN.md §6h.

## Goal

At the detail zoom, every colonist, raider and animal is a figure that faces
where it goes, walks with a visible stride, turns through corners instead of
snapping, carries what it holds, and swings its tool when its worksite
strikes. Zoomed out, figures become silhouettes, then dots. 200 pawns draw in
one call inside 0.4 ms of the renderer's 4 ms, and the sim is unchanged. A
mod adds a hair style, a gait or a creature body with data alone.

## Order

1. **The batch spike** answers how figures are drawn on macOS's GL; **rounded
   corners** needs nothing and ships first.
2. **Bodies and the batch**, then **gaits**; the **bench pass** gates both.
3. **Appearance**, **hands**, **lying down**.
4. **The rig overlay**; **the cast card** when the story milestone's premise
   picker exists.

## What this waits on elsewhere

- Chalkline's reduce-motion setting (bf3079fb) for gaits' still mode.
- Random streams (c2579dbc) for per-pawn appearance draws.
- The premise picker (e874ca2d), premises (bea51754), traits (39915ec5) and,
  from #246, the New colony screen (73751f4f) for the cast card.

## Not in this milestone

- Apparel and armour (41006a46, defense): worn layers will use this body's
  sockets.
- Emotion on the body (95929d9b, story): arrives as gaits.
- A body left by death (66617791, story): drawn with the lying pose.
- A carried torch's light (5a69f9c9, lighting): a held item this body carries.

## Where it sits

Independent of the other milestones except for the cast card. Due date: not
set.
