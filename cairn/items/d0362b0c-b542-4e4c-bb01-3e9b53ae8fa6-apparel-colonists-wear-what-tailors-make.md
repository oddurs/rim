---
id: d0362b0c-b542-4e4c-bb01-3e9b53ae8fa6
title: 'Apparel: colonists wear what tailors make'
type: feature
status: backlog
milestone: crafting
created: 2026-09-27
updated: 2026-09-27
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

- [ ] A mod adds a garment as data, and a colonist puts it on and takes it off
- [ ] A worn garment is saved, hashed and drawn on the pawn
- [ ] Primitive's leather makes at least one garment

## 2026-09-27

Split from 0927f0af (tailoring). That item shipped hides from butchering and leather. Wearing is engine work the codebase doesn't have yet: no equip slot, no wear job, no insulation stat. It's filed here rather than built halfway. Pairs with 308074c6, which reads insulation.
