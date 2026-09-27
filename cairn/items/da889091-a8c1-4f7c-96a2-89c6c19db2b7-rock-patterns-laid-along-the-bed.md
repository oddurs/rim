---
id: da889091-a8c1-4f7c-96a2-89c6c19db2b7
title: Rock patterns laid along the bed
type: feature
status: backlog
milestone: rock-face
depends_on:
- 3f90e043-bf62-48c8-ac67-d043dc755b6e
- 7c53ec62-85bb-4752-8bed-9b1271d0eef3
- 8fea2eef-1909-4e97-8960-4888305d0003
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Problem

Strata (3f90e043) adds packed earth, clay, chalk, limestone, wet limestone and bedrock, and they differ only in tint. The one rock pattern, `crag`, is random squiggles. Wall patterns (7c53ec62) follow a wall's run, and rock has no run: bedded limestone drawn as `courses` reads as a wall. DESIGN.md §6g, "Whose pattern does rock use?".

## Proposal

- Seven patterns join `rim_sim::look::Pattern`, after the lithology symbols on geological maps:
  - `igneous`: three crosses and a tick per cell, plus two joint sets (about 62° and −24°, 2.6 and 3.1 cells apart) run through the mass
  - `bedded`: beds 0.34 cells apart, wobbling ±0.045, with joints about 1.35 cells apart, staggered bed to bed, 40% of them missing
  - `nodular`: faint beds 0.5 apart, and flint nodules (dark lumps, pale rind) along every other bed
  - `laminated`: rows of short dashes 0.15 apart
  - `pebbly`: four pebbles and grit
  - `hatch`: close cross-hatching
  - `seep`: `bedded` plus, on a cell whose depth is 1, a damp band and droplets along each open side
- Rock patterns are laid in world space along the bed: `q = y + x·tan(dip)`, with the dip from the stratum's `bed = { dip = 6 }` (degrees; the surface uses 6). They never follow a run or a facing. Wall patterns are unchanged.
- The same fade band as walls: none below 14 points a cell, full from 22 (`pattern.rs` `FADE`/`FULL`).
- Core: granite `igneous`, limestone `bedded`, wet limestone `seep`, chalk `nodular`, clay `laminated`, packed earth `pebbly`, bedrock `hatch`. `crag` stays for mods.
- The reference is `rockPattern` in the concept artifact.

## Acceptance criteria

- [ ] Every solid terrain's thing in core declares one of these patterns, and `rim check` rejects an unknown pattern name
- [ ] Beds cross a cell boundary and a kind boundary without a break (autotest shot, looked at)
- [ ] A stratum's `bed.dip` changes the angle of its beds (fixture test with a mod stratum at 20°)
- [ ] Render bench: whole map unchanged; quarry and close views within the §8 budget, numbers recorded here
- [ ] `docs/modding/looks.md` lists the rock patterns and `bed`
