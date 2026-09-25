---
id: 29c323f5-8043-4b31-8464-f6f96e644bf7
title: Skills learned by doing
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: additive
effort: m
layer: engine
area: sim
pillar:
- growth
---

## Why

Colonists grow. Work speed and combat scale with skill.

## Acceptance criteria

- [x] Skill defs
- [x] XP from jobs

## 2026-09-25

[[skill]] defs in core (construction, mining, plants, melee); a work type's skill resolves to skill_r, and the one skill with melee = true is DefDb::melee_skill, so the engine names none. Pawn.skills holds experience per skill (sorted, saved, remapped, in the state hash); level L needs 1000*L*(L+1), capped at 20: about three days of steady work to level 5 at one experience a tick. People (intelligent creatures) arrive with every skill at a random level 0-6 from the world RNG; animals have none and fight as before. Work: Pawn::work_amount gives 60% + 10% per level (normal at 4, 260% at 20) as whole units a tick with the fraction carried on the pawn, and teaches one experience; World::work_on takes that amount. Melee: melee_base scales 80% to 160% with the melee skill and every swing teaches 20. A tool's speed (7016d86b) shrinks a job's total; skill raises the rate; they multiply. rim-c2's resume test now waits for the next unit, since a slow worker can go a tick without one. No UI here: the inspection panel and the Work Board show skills.

## 2026-09-25

Review: founder.rs passed only because seed 3 rolls melee 6; it now pins the skill, and the founder's bonus is added flat after the skill scales the creature's damage. A pawn from before skills (or missing one a mod added) loaded at level 0, a silent nerf; the load now gives people the middle starting level, 3, with no RNG. work_frac is in the state hash.
