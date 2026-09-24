---
id: b7f5cde1-a8c4-4d7d-b93d-bdf0ba0d523b
title: 'Mod index: a git repo of mod entries, checked by CI'
type: feature
status: backlog
milestone: platform
depends_on:
- be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
- eb2c9422-7ed6-4060-806c-6d1cee5a0ba1
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
