---
id: 71
title: 'Prioritize: right-click to force a job'
type: feature
status: doing
milestone: shelter
assignee: Oddur Sigurdsson
claimed: 2026-09-23
created: 2026-09-22
updated: 2026-09-23
priority: p1
api: none
effort: l
layer: engine
area: ui
---

## Why

Point a colonist at a specific task. Until now the only direct control is
drafting, which turns off work entirely: a drafted pawn can move and fight
and nothing else. Everything productive goes through area designations and
the pawn's own job search, so the player can mark a forest but never say
"you, that tree, now". Right-click on a selected colonist closes that gap
and is the control scheme players arrive already knowing.

## Design

One sim function resolves a right-click into an order, and both the client
hint and the command go through it, so what the cursor promises is exactly
what the click does. Orders read the defs, so a mod that adds a harvestable
thing gets a working right-click with no engine change.

Priority when resolving: creature under the cursor, then fixture, then item,
then bare ground. A drafted pawn only ever gets move and attack.

An order is one job. When it finishes the pawn returns to normal AI; it does
not become a standing designation.

## Acceptance criteria

- [ ] `Command::Order` assigns a contextual job to one pawn
- [ ] `command::order_at` resolves a click read-only; client and command share it
- [ ] Harvest orders work on undesignated things, labelled from the designation def
- [ ] Blueprint orders deliver the missing material, then build
- [ ] Creature orders hunt wild animals and attack hostiles
- [ ] Bare ground orders move, drafted or not
- [ ] An order steals the reservation and interrupts the pawn that held it
- [ ] Cursor shows what the right-click will do before it lands
- [ ] Pawn panel names the current job from defs, not a generic label
- [ ] Sim tests cover chop, build, move, hunt and the drafted restriction
- [ ] Determinism test still passes
