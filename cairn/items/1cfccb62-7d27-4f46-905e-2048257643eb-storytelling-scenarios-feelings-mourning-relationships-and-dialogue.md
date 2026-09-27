---
id: 1cfccb62-7d27-4f46-905e-2048257643eb
title: 'Storytelling: scenarios, feelings, mourning, relationships and dialogue'
type: spike
status: done
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p3
api: additive
effort: l
layer: plugin
area: storyteller
---

## Question

How does a run become a story? That covers how scenarios are set up and
stack, how people feel, mourn and relate, and how dialogue is written so it
sounds human. The discussion is in docs/ideas/storytelling.md.

## Options

- Scenarios: none (DESIGN.md today), or premises that set the opening but
  never the difficulty, with stackable threads paced by the storyteller
- Social plugins: one plugin, or mood, relationships, mourning and dialogue
  as separate plugins that meet through core's names
- Dialogue: templates only, or one writer interface for templates,
  hand-written lines and an optional model the player connects

## Decision

Recorded in DESIGN.md §4g. A premise sets the opening and never the
difficulty, amending §1 and §2. Stories stack as one premise, any number of
threads that offer beats to the storyteller, and one writer. Witnesses,
memories, relations, interactions, gatherings, bodies and traits are engine
mechanisms; mood, social, mourning and story are plugins. Words are
presentation, written from intents by templates by default, or by an external
process the player connects. Built as the Story milestone.


## Acceptance criteria

- [x] "No scenarios" in DESIGN.md §1 and §2 amended or kept, by a person
- [x] Decided whether witnesses and notability are core mechanisms
- [x] Split into items under the mood milestone and after it

## 2026-09-26

The person delegated the decisions ('make all the decisions for me'), so criterion 1 is ticked on that authority.
