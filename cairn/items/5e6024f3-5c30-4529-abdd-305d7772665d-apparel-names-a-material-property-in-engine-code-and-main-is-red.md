---
id: 5e6024f3-5c30-4529-abdd-305d7772665d
title: Apparel names a material property in engine code, and main is red
type: bug
status: done
milestone: crafting
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
priority: p0
api: none
---

## What happens

#267 (apparel) had the engine read a garment's warmth by a material factor it named itself, the word for a material's resistance to heat: in the ApparelDef field, `World::insulation`, and two lookups by that literal. `factors::the_engine_does_not_know_the_word` guards DESIGN.md's rule that the engine names no material property, and it fails on every platform.

## What should happen

The engine uses a neutral name (`warmth`), and the apparel def names the factor that scales it as data (`factor = "..."`). Primitive's garments name the material factor.

## Acceptance criteria

- [x] `the_engine_does_not_know_the_word` passes
- [x] Apparel tests pass with the factor named in data

## 2026-09-28

ApparelDef.insulation became warmth (degrees), plus factor: Option<String>, the material factor that scales it, named in data. World::insulation became warmth_worn and garment_warmth, which read a.factor by name, or 1.0 without it. Primitive's garments say factor = "insulation". No engine src names the word, and factors::the_engine_does_not_know_the_word passes. My own gate on #267 had run only the touched tests, so it never ran this tree-wide guard. rim-c2's merges now run the guards on every PR.
