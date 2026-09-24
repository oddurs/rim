---
id: 210
key: building
title: Building
type: milestone
status: planned
depends_on:
- 9
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
due: 2026-11-15
---

A one-week sprint inside Colony. Building stops being a fixed list of defs
and becomes a system: one `wall` def built out of whatever you have, where
the material you choose changes what the wall is worth. Rooms start caring
what they are made of, which is what makes a window a real thing rather
than a wall with a hole drawn on it.

Roofs stay out. 0054 settled it: enclosure plus the 400-cell cap is
shelter, and that does not change here.

## Sprint goal

Build a stone hut with a window and a wooden one next to it, and have them
behave differently: the stone holds its heat, the wooden one is warm faster
and burns, and both let daylight in through the glass. Add `mods/marble`
with a single item def and build every wall, door, bed and table in marble
without touching the engine.

## Why now

Today a "wooden wall" and a "stone wall" are two unrelated defs that happen
to look alike. Six materials times ten buildables is sixty defs, and a mod
that adds one material has to write ten of them. That is the opposite of
the thing this project is for.

## Plan

- **Days 1-2, the material system:** stuff on buildables and items, the
  material chosen at build time, `MadeOf` on the result, factors resolving
  against base stats.
- **Day 3, the seam:** core's `wall_wood`/`wall_stone` collapse into one
  `wall`; the build UI gains a material picker.
- **Days 4-5, rooms made of something:** the boundary spike, then room
  leak and daylight derived from what encloses the room. Windows land on
  top of it.
- **Days 6-7, furniture and looks:** interaction spots, the table/chair/
  stove set, wall autotiling and material colour.

## Definition of done

- One `wall` def in core, built in wood, stone or metal, and no
  `wall_<material>` defs left anywhere.
- A material is one item def. Adding `mods/marble` needs no engine change
  and no new buildables.
- Nothing in engine code names a material or knows what "insulation" means.
- A stone room and a wooden room of the same shape hold heat differently,
  and the difference is visible in the temperature overlay.
- A window passes daylight and leaks heat; a wall does neither.
- The game still loads and plays with core alone, and determinism holds.

## Open questions this sprint settles

- Do room properties come from the room's boundary, or stay field
  constants? (0211 -- the answer shapes windows, drafty rooms and 0188.)
- What is the v1 material list and factor set? Starting assumption below,
  to be confirmed in 0212.

## Assumptions, pending your say-so

- **Materials v1:** wood, stone, salvaged metal. Three is enough to prove
  the system without inventing a resource economy.
- **Factors v1:** max hp, work to build, insulation, flammability, beauty.
  Beauty is inert until mood (0088) and is declared now so material defs do
  not need editing later.
- **Table and chairs ship in this sprint** as furniture with an interaction
  spot, so pawns eat at them instead of standing. Their *mood* effect
  arrives with 0088; flagged on 0218 rather than held back.
- **Stove ships as a heat and light emitter only.** Cooking needs bills and
  recipes, which DESIGN.md 5 puts outside core.

## Out of scope

- Mood effects for furniture (0088, Mood).
- Cooking, bills and recipes (Crafting).
- Fire actually spreading (0200) -- flammability is declared, not burned.
- Wind shelter (0188) and drafty rooms (0190), which want the boundary
  answer from 0211 first.
