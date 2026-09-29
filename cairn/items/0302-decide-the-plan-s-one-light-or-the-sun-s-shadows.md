---
id: 302
uid: 0779def9-134c-4e87-abdf-2e3472fb2801
title: 'Decide: the plan''s one light, or the sun''s shadows'
type: spike
status: done
milestone: lighting
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: s
layer: client
area: render
---

## Why

Houses gives every mass one fixed shadow, down and to the right, baked into the chunk mesh (ae5c3807, DESIGN.md §6c). Lighting casts shadows toward wherever the sun is (§6e). At dusk a wall would have two shadows pointing different ways. One of them has to give, and it's a question of look, so a person decides.

## Options

1. **Contact shadow that yields to the sun (proposed in §6e).** The fixed shadow shrinks to a short contact shadow and fades in proportion to the direct sky light reaching the cell. By day the sun's shadow reads; at night, indoors, underground and on `low` the plan convention remains. Cost: one multiply in compose.
2. **The plan wins.** Keep ae5c3807 as ruled; sky shadows only from trees and roofs. Cheapest, least atmosphere.
3. **The sun wins.** Drop the fixed shadow once sky shadows land; masses read by their sun shadow only, which vanishes at night.

## Acceptance criteria

- [x] An option is picked and DESIGN.md §6e and §6c record the ruling

## 2026-09-26

Decided (the user delegated the call): option 1. The fixed shadow becomes a contact shadow that fades as direct sky light reaches the cell. It is drawn in lighting's compose pass (8f4f1de8), not baked into the mesh, because the fade needs per-texel sky visibility. ae5c3807 keeps the top-and-left highlight and drops the shadow; the Houses session agreed to rewrite it. The screenshot criterion moved to 8f4f1de8, since no screenshot can exist before compose does.
