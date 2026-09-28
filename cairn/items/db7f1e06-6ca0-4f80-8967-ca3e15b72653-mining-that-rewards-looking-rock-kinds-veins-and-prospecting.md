---
id: db7f1e06-6ca0-4f80-8967-ca3e15b72653
title: 'Mining that rewards looking: rock kinds, veins and prospecting'
type: content
status: done
milestone: crafting
assignee: Oddur Sigurdsson
depends_on:
- 8cc6252d-67a0-4652-b764-851f3e6bc72a
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
- e3846c47-f425-46f5-8e96-fec056074052
created: 2026-09-24
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: m
layer: core
area: map
---

## Why

Mining exists — `granite` yields 10 stone for 500 work
(`mods/core/defs/nature.toml:33`) — and it is a chore, because every cell
gives the same thing. Mining becomes iconic when the wall you pick matters.

On one plane the payoff is already there and needs no engine work: digging
into a rock face leaves a space bounded by rock, which `Map::ensure_rooms`
(`crates/rim_sim/src/map.rs:299`) treats as an enclosed room. So it is warm,
dark and weather-proof for free, out of the field system as it stands (§4a).
Digging in is *already* the best shelter in the game. Nothing tells the player
that.

## What

- **More than one rock.** Granite, limestone, flint-bearing chalk: different
  work, different yields, different `stuff` factors (`defs.rs:256`). Mapgen
  bands them, so where you landed decides what you can make.
- **Veins, not uniform stone.** Ore is a per-cell amount on a stock field
  (`d77d9e1f`), so a vein depletes and a rich cell is worth returning to.
  Veins are placed by mapgen as a stage, which a script may take over (§6a).
- **Flint is the reason to mine on day one.** It is `knappable`, the
  input to every stone tool (`e3846c47`), so the wall is where tools come from. Surface flint is
  scarce; the wall has more.
- **Prospecting is reading, not a minigame.** A rock cell shows its kind and
  whether it is veined once a pawn has stood next to it. No new mechanism —
  the map overlay every field gets for free (`O` cycles overlays, §4a).

## Out of scope

Cave-ins and structural support. They are a pressure worth having, but they
are a separate mechanism and this ticket is content.

## Acceptance criteria

- [x] At least three rock kinds with distinct yields and stuff factors
- [x] Mapgen bands rock kinds deterministically from the seed
- [x] Ore amount is per cell and depletes as it is mined
- [x] Flint is minable, and chalk yields it as a vein
- [x] A veined cell is visible in an overlay once seen
- [x] Determinism test passes

## 2026-09-26

DESIGN.md §6d (Depth) rules that rock is terrain. Build rock kinds and veins as solid [[terrain]] with a mine block on top of 8cc6252d-67a0-4652-b764-851f3e6bc72a, not as more rock things. Rock kinds per level come from [[stratum]] defs (3f90e043-bf62-48c8-ac67-d043dc755b6e). The seen bit for prospecting is part of 5689930d-2bd1-4838-b403-a72bc61c31e9.

## 2026-09-27

How rock kinds, veins and the seen state look is planned in the Rock face milestone (DESIGN.md §6g): patterns along the bed (da889091-a8c1-4f7c-96a2-89c6c19db2b7), unseen rock plain (2763e32b-d779-471e-9f74-1d24f810c4a8) and ore set into the stone (9259bafa-bbd4-49b6-ab51-3fac4a6e3a70). This item stays content: kinds, veins, amounts and the overlay.

## 2026-09-27

Built as content on engine mechanisms that name no rock.
Rock kinds: granite gives core's stone (hp 1.6, span 1.25). Limestone and wet limestone give the new limestone_blocks (hp 1.25, work 1.4, beauty 1.3, span 1.1). Chalk gives chalk_blocks (hp 0.75, work 0.9, insulation 1.6, span 0.9). Strata already band the kinds by level from the seed (criterion 2, from the depth work). The veins below are seeded the same way, and tests/mining.rs checks the same seed gives the same veins and another seed doesn't.
Ore: a vein is a stock field with levels = "all" and no rate terms. Such a field is now skipped by the per-tick step, so it costs nothing. A [[vein]] def (field, in = terrains, chance, size, amount) lays blobs of whole units at map generation, after every level's scripts. It hashes the seed, the vein's id and the cell, so the world's RNG is untouched.
Harvest draw: a harvest with draw = { field, per, most } takes its yield from the field at the cell, up to most a time, and isn't offered (harvest_ready) once the cell has less than per.
Content: core adds the Extract designation, the mine work type's take-without-destroying. Primitive adds primitive:flint_vein, a [[vein]] in core:chalk (chance 0.04, 3–7 cells, 4–10 flint each), and a second harvest on chalk that extracts up to 3 flint a time and leaves the rock. Mining chalk out still gives chalk blocks and 1 flint.
Open: criterion 5. The flint field is an overlay (the O cycle), but hiding unseen cells waits on quiet-field's Map::seen (#245, 5689930d), and they asked me not to add a seen bit of my own. Criterion 6 is ticked once the gate passes.

## 2026-09-27

Criterion 5 now builds on quiet-field's Map::seen (#245). A field with until_seen = true is left blank in the overlay and the hover card where no colonist has seen the cell. The sim still reads every cell. primitive:flint_vein sets it. tests/mining.rs checks that the flag is set, that rock below starts unseen, and that see_around reveals a cell. The drawing itself is the client's two-line check.
