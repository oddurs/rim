---
id: db7f1e06-6ca0-4f80-8967-ca3e15b72653
title: 'Mining that rewards looking: rock kinds, veins and prospecting'
type: content
status: backlog
milestone: crafting
depends_on:
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
- e3846c47-f425-46f5-8e96-fec056074052
created: 2026-09-24
updated: 2026-09-25
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

- [ ] At least three rock kinds with distinct yields and stuff factors
- [ ] Mapgen bands rock kinds deterministically from the seed
- [ ] Ore amount is per cell and depletes as it is mined
- [ ] Flint is minable, and chalk yields it as a vein
- [ ] A veined cell is visible in an overlay once seen
- [ ] Determinism test passes
