---
id: 492
uid: 6f8be218-a3ff-4c58-ac18-62b041896b79
title: 'Hands: carried items, bulky loads on the shoulder, tools and the work swing'
type: feature
status: backlog
milestone: people
depends_on:
- 498
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
pillar:
- performance
effort: m
layer: client
area: render
---

## Why

What someone holds is half of what they're doing. A pawn carrying logs, one
holding an axe and one hauling berries look the same today, and a worker
lunges into a blow (`worksite.rs` `lunge`) with nothing in hand.

## What

- **Carried items.** A carried lot is drawn at the body's `hold` socket with
  the item token drawn from the thing's own look (#166), so a mod's item
  carries correctly with no work. The `carry` gait moves both hands to hold
  it.
- **Bulky loads.** An item with the `bulky` tag goes on the `shoulder`
  socket, drawn along the facing, and the `carry_bulky` gait applies.
- **Tools.** A held tool is drawn from its thing's look at the `tool` socket.
  It hangs at the side while walking and swings while working.
- **The work swing.** At a worksite, the tool hand arcs back and down, timed
  to the same strikes as the lunge and the chips (`site.struck`,
  `ticks_to_strike`), so the axe lands when the chips fly. A paused game
  holds the swing where it is.
- **Drafted.** A drafted pawn plants its feet wider and holds its weapon
  forward.

## Acceptance criteria

- [ ] A carried item draws at the hold socket with its own look, and bulky items on the shoulder (autotest screenshots)
- [ ] A held tool draws in the hand; at a worksite its swing peaks on the strike tick (unit test on the timing function)
- [ ] Paused mid-swing, the pose doesn't change between frames (test)
- [ ] Drafted pawns show their stance
- [ ] `rim --bench-render`: the pawn pass stays within 535a1fb9's budget with 200 pawns carrying
