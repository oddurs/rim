---
id: d19b7e65-fc9c-4c0a-9ce6-19cc8020f217
title: 'rim_text: templates from intents, seeded picks, lazy expansion'
type: feature
status: backlog
milestone: story
depends_on:
- 17009725-a061-4445-829f-b1c73c6ce2bd
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: l
layer: client
area: ui
pillar:
- determinism
- performance
---

## Why

Words are presentation (DESIGN.md §4g, Telling). One writer interface serves
templates, hand-written lines and a connected writer, in the client and the CLI.

## Acceptance criteria

- [ ] A `rim_text` crate: intent in, line out, used by the client and the CLI
- [ ] Template picks seeded by the world seed and the event's sequence number; a replay shows the same words
- [ ] Recently used templates avoided per speaker
- [ ] Picks made as events arrive; text expanded only when seen
- [ ] Nothing about text is saved for the template writer
