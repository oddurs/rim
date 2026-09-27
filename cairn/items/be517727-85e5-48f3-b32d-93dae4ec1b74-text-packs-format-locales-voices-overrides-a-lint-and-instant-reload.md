---
id: be517727-85e5-48f3-b32d-93dae4ec1b74
title: 'Text packs: format, locales, voices, overrides, a lint and instant reload'
type: feature
status: backlog
milestone: story
depends_on:
- d19b7e65-fc9c-4c0a-9ce6-19cc8020f217
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Acceptance criteria

- [ ] `text/<locale>/*.toml`: rules keyed by act, emotion and bond, nested grammars, slots
- [ ] Voices: register, words and a never-say list, by trait and background
- [ ] Hand-written lines override templates for an exact moment
- [ ] `rim check` lints packs: unknown slots, no fact slot, over the length cap, deny-listed phrases
- [ ] Editing a pack re-renders lines in a running game without a replay
