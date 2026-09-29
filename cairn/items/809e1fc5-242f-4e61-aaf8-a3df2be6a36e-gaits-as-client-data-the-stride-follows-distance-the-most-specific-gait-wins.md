---
id: 809e1fc5-242f-4e61-aaf8-a3df2be6a36e
title: 'Gaits as client data: the stride follows distance, the most specific gait wins'
type: feature
status: backlog
milestone: people
depends_on:
- 535a1fb9-2cd8-4798-b31c-04f89fb1aee0
- bf3079fb-0d7d-4db7-8f46-f19f97e110dd
created: 2026-09-27
updated: 2026-09-29
priority: p0
api: additive
pillar:
- plugin-first
- performance
effort: m
layer: client
area: render
---

## Why

A figure without a stride glides. DESIGN.md §6h rules that motion follows the
sim, never the wall clock: the walk cycle advances with distance walked, idle
breathing with sim ticks, so a paused game holds still, 6× stays coherent and
a replay looks identical. How someone walks (hauling logs, limping, tired) is
information a player reads at a glance.

## What

**Gaits are client data** (`ui/gaits.toml`):

```toml
[[gait]]
id = "walk"
cycle = 1.1       # cells per full stride
stride = 0.12     # foot travel fore and aft
swing = 0.075     # hand swing
sway = 3          # shoulder turn, degrees

[[gait]]
id = "carry_bulky"
when = { carrying = "bulky" }
stride = 0.085
swing = 0.04
sway = 5
hold = "shoulder"
```

- Core ships `idle`, `walk`, `run`, `carry`, `carry_bulky`, `limp` and
  `tired`.
- `when` reads a fixed set of pawn facts the client already sees: `moving`,
  `running` (drafted or fleeing), `carrying`, `carrying = "bulky"`,
  `health_below`, `rest_below`, `drafted`, `working`. Later items add facts
  (emotions, pawn tags).
- The most specific gait wins, meaning the most conditions met. A tie is a
  contested slot under the ladder (dbb92ebe, from #246): until that lands, the
  tie is reported and the first by id wins.
- Channels: `stride` moves parts with `move = "stride"` along the facing by
  ±stride·sin(φ + phase·2π); `swing` likewise for hands; `sway` turns the
  torso; `lean` moves the head forward; `breath` scales the torso about 1%.
- φ advances with the distance the pawn is drawn to move (a client cache per
  entity), so slower steps in snow (6f1e7410) shorten nothing and simply take
  longer. `breath` advances with the sim tick.
- With `reduce_motion` on (bf3079fb), every channel rests at 0: pawns glide.

## Acceptance criteria

- [ ] `[[gait]]` loads from `ui/`; an unknown fact in `when` is an error listing the known ones (test)
- [ ] The chooser picks the most specific gait for each fact set in a table test, and reports a tie
- [ ] Phase is a function of distance: the same path drawn at 1× and 6× gives the same pose at the same cell (test)
- [ ] Paused, no pose changes between frames (test)
- [ ] Reduce motion rests every channel (test)
- [ ] The autotest shows walk, carry_bulky and limp at the detail zoom
- [ ] docs/modding/bodies.md documents gaits and the facts

## 2026-09-29

PAUSED (Bare Metal freeze, 2026-09-29). Done: rim_ui::gait (ui/gaits.toml loader, Facts, chooser with specificity and tie report, need ids qualified by the gait's mod), core's seven gaits, Strides (phase by distance drawn, jumps ignored) and breath by sim tick in motion.rs, Pose in figures.rs moving stride/swing/sway/lean parts, reduce_motion rests every channel, rim check validates gaits.toml, the guide section in docs/modding/bodies.md, tests (gait: 4, figures: stride and still, motion: speed-independence and paused), and an autotest section shooting walk, carry_bulky and limp. Left: rebase onto main (the autotest is now a module per section, #345; draw.rs uses app.now, #351), the gate and the autotest, then the PR. Next step: rebase and move the gaits autotest section into its own module. Branch: feat/809e1fc5-gaits, one commit 7f9d269c on the old bodies commit 9c9a93f7; draft PR once a gate slot allows the push.
