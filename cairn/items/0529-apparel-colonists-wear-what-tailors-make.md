---
id: 529
uid: d0362b0c-b542-4e4c-bb01-3e9b53ae8fa6
title: 'Apparel: colonists wear what tailors make'
type: feature
status: done
milestone: crafting
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: additive
effort: l
layer: engine
area: needs
---

## Why

Tailoring (0927f0af) got as far as leather: the engine has no way to wear anything, so a cloak could only be wealth. Clothes are what make winter survivable, and they are the other half of 308074c6's warmth.

## What

- A worn slot on a pawn: an item def with an `apparel` block (body part or layer, insulation, and the materials it's cut from) can be worn. It's held off the map like a held tool.
- Colonists put on what keeps them in their comfort range, and change as the season turns; a wear job, like fetching a tool.
- Apparel wears out with use and shows on the pawn (a look layer).
- Content: primitive's hide wrap and leather cloak from `leather`, made at a crafting spot, and a tailoring bench recipe later.
- Insulation feeds 308074c6's comfort range; this item is the wearing, that one the warmth.

## Acceptance criteria

- [x] A mod adds a garment as data, and a colonist puts it on and takes it off
- [x] A worn garment is saved, hashed and drawn on the pawn
- [x] Primitive's leather makes at least one garment

## 2026-09-27

Split from 0927f0af (tailoring). That item shipped hides from butchering and leather. Wearing is engine work the codebase doesn't have yet: no equip slot, no wear job, no insulation stat. It's filed here rather than built halfway. Pairs with 308074c6, which reads insulation.

## 2026-09-27

People (5d09b04e, DESIGN.md §6h): a worn garment draws as layers at the body's torso socket (535a1fb9), tinted and patterned by its material, so leather reads as leather. Until clothes exist, the torso takes the skin colour: the castaway is naked.

Apparel is engine mechanism plus content.
- apparel = { layer, insulation, wear_per_day } on a thing. Garments must have stack_limit 1, and loading enforces it.
- Pawn::worn holds the garments; the Worn component keeps them off the map and out of the stock and store index, like Held.
- NeedDef.insulated: core's warmth has its comfort's cold end lowered by worn insulation times the material's insulation factor.
- Dressing (ai::dress, after needs and before work) has hysteresis. When the insulated need's field outdoors is below comfort minus what's worn, the colonist fetches the warmest garment for a layer not already worn warmer (Job::Dress). Above comfort + 6° it takes the warmest off where it stands.
- Garments lose hp by the day and wear out with a message. Dead colonists drop what they wore; colonists who leave take it.
- Saves: an optional engine:worn section, no format bump, with its text form. Pawn.worn saves with the pawn. The hash covers what's worn.
- Client: a band of each garment's colour inside the pawn's disc, outer layers out.
Content: primitive's hide wrap (2 hides, body, +6°) and leather cloak (3 leather, outer, +10°), cut with a cutting tool at a crafting spot.
Tests (tests/apparel.rs): a mod's scarf is put on in the cold and taken off in the warmth; worn is saved and hashed and keeps the warmth need fuller at 7°; three leather make a cloak.
Not done: a tailoring bench, which is a later recipe; the pawn panel doesn't list what's worn yet. Wetness from rain is 308074c6, next.
