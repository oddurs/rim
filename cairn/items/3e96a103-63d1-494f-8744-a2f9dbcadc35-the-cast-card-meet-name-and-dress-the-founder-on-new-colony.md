---
id: 3e96a103-63d1-494f-8744-a2f9dbcadc35
title: 'The cast card: meet, name and dress the founder on New colony'
type: feature
status: backlog
milestone: people
depends_on:
- 39915ec5-4ff9-40a7-af6b-aa6d5dbd39a0
- 73751f4f-cd52-467a-9098-55d8033e4b4b
- 8469a7ff-bee8-4237-9fc6-a424aea6c362
- bea51754-c7d0-4353-b763-1a6a11e6f383
- e874ca2d-e8d5-4af2-af7f-59817a5a8da4
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
pillar:
- growth
- plugin-first
effort: m
layer: client
area: ui
---

## Why

A new colony should let the player meet their founder: see them, name them,
change how they look, and, if the premise allows, roll a different person.
DESIGN.md §6h rules that looks are free and cosmetic, that the person (traits
and skills) comes from the premise, and that nobody edits a stat (§1).

## What

The cast card, shown for each cast member in the premise picker's slot on
the New colony screen (73751f4f; e874ca2d):

- The figure, turning slowly; drag to turn it, and a Standing/Walking toggle.
- **Name**, editable.
- **Looks:** skin, build, hair style and hair colour, from the body's
  appearance slots and core's palettes. **Clothes** follow the premise; the
  castaway's reads "None".
- **Roll appearance:** new looks, the person unchanged.
- **Roll another warrior:** only when the premise allows it
  (`reroll = true`). It rolls a whole person from the premise's pool, never
  one stat. The roll number joins the seed code (47ec1fa0), so a shared code
  brings the same founder.
- **Traits** (the premise's fixed ones marked, like Founder) and **skills**,
  read-only.
- What the player changed goes into the pawn's picks and name at Start.

## Acceptance criteria

- [ ] The card shows each cast member with the live figure, name, looks, traits and skills
- [ ] Changing a look updates the figure at once and changes no trait or skill (test on the resulting pawn)
- [ ] Roll another warrior appears only for premises that allow it, and the same seed code and roll number give the same person (test)
- [ ] The founder in the started colony has the chosen name and looks (autotest)
