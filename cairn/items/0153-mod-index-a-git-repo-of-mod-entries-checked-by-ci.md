---
id: 153
title: 'Mod index: a git repo of mod entries, checked by CI'
type: feature
status: backlog
milestone: platform
depends_on:
- 149
- 159
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Discovery without a closed store: adding a mod is a pull request (DESIGN.md §10, decided in 0120).

## Acceptance criteria

- [ ] Index repo layout: one TOML per mod id (repo URL, description, tags)
- [ ] Index CI on each PR: unique id, SPDX `license` present, `rim check` and `rim test` pass on the latest tag
- [ ] Mod ids are first come, first served; renames keep an alias
- [ ] Published as a static JSON snapshot the game can fetch
