---
id: 535a1fb9-2cd8-4798-b31c-04f89fb1aee0
title: Bodies as client data, drawn in one batch at three levels of detail
type: feature
status: backlog
milestone: people
depends_on:
- cc289956-8b67-429b-8f9a-5a98e651398e
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
pillar:
- plugin-first
- performance
effort: l
layer: client
area: render
---

## Why

A pawn is a disc in its creature's colour (`draw.rs` `pawns()`), so a
colonist, a raider and a wolf differ only in colour and size, and nothing
shows which way anyone faces. DESIGN.md §6h rules that a person is a plan
figure drawn from above in the plan's ink, and that what a creature looks like
is client data.

## What

**Bodies are client data**, under a mod's `ui/` (`ui/bodies.toml`), the side
#246 gives each player:

```toml
[[body]]
id = "human"
parts = [
  { id = "shadow", draw = "disc", r = 0.26, squash = 0.88, color = "#00000042", world = true },
  { id = "foot_l", draw = "disc", x = -0.085, r = 0.06, squash = 1.45, color = "@feet", move = "stride", phase = 0.0 },
  { id = "foot_r", draw = "disc", x =  0.085, r = 0.06, squash = 1.45, color = "@feet", move = "stride", phase = 0.5 },
  { id = "hand_l", draw = "disc", x = -0.235, r = 0.052, color = "@skin", line = "light", move = "swing", phase = 0.5 },
  { id = "hand_r", draw = "disc", x =  0.235, r = 0.052, color = "@skin", line = "light", move = "swing", phase = 0.0 },
  { id = "torso",  draw = "box", w = 0.5, h = 0.27, round = 0.5, color = "@torso", line = "light", move = "sway" },
  { id = "head",   draw = "disc", y = -0.03, r = 0.115, color = "@skin", line = "light", move = "lean" },
]
sockets = { hair = "head", worn = "torso", hold = [0, -0.22], shoulder = [0.16, 0], tool = "hand_r" }
```

- A part is a look layer (docs/modding/looks.md: `disc`, `box`, `line`,
  `arc`, `sprite`, the `line` weights, `color`) plus `move` (a gait channel)
  and `phase`. Written facing north; the renderer turns the whole figure by
  its facing. `world = true` parts (the shadow) don't turn.
- `@name` colours are appearance channels: `@skin`, `@feet`, `@torso`,
  `@hair`. Until appearance lands, they come from the creature's `color`.
- Core ships `human`, and bodies for `deer`, `wolf` and `hare`;
  wildlife_plus ships `boar`.
- The creature def gains `body = "core:human"`, an id the sim never reads. A
  missing body or part draws today's disc, with a warning naming it.
- Validation matches looks: an unknown field, a bad weight or a size out of
  range is an error naming the file and part.

**The pawn batch** draws every visible pawn through the path the spike
(cc289956) chose, in one call.

**Three levels of detail**, by pixels per cell `z`:

| Level | `z` | Draws |
|---|---|---|
| Dot | below 10 | a disc in the main colour, the faction ring |
| Silhouette | 10 to 20 | shadow, torso, head, hair colour, ring, a carried dot |
| Full | from 20 (`DETAIL_ZOOM`) | every part |

- The faction ring moves under the figure at the `hair` weight in the faction
  colour. Chalkline's selection and hover rings are unchanged.
- The health bar and the drafted mark stay where they are.

## Acceptance criteria

- [ ] `[[body]]` loads from `ui/`, with validation errors naming the file and part (tests)
- [ ] Core's human and three animals, and wildlife_plus's boar, draw from their bodies; a creature without one draws a disc and warns
- [ ] One draw call for all pawns at every level of detail (bench counts draw calls)
- [ ] The level switches at 10 and 20 px a cell (unit test on the chooser)
- [ ] Screenshots at 7, 15 and 40 px a cell in the autotest
- [ ] `rim --bench-render`: 200 pawns at full detail inside 0.4 ms of CPU on the reference machine, or the measured number and why, noted here
- [ ] Editing `ui/bodies.toml` in a running game redraws every pawn with no sim reload
- [ ] docs/modding/bodies.md documents `[[body]]`, parts, sockets and channels
