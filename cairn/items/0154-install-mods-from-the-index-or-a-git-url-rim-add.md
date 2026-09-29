---
id: 154
uid: 76a0bc45-13c1-4725-b5ce-47957109ac41
title: 'Install mods from the index or a git URL: rim add'
type: feature
status: backlog
milestone: platform
depends_on:
- 152
- 153
created: 2026-09-23
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Players never need git; developers can point at any repo or branch.

## Acceptance criteria

- [ ] `rim add <id>` and `rim add github.com/<owner>/<repo>[@tag]` download release archives over HTTPS
- [ ] Resolves dependencies, verifies each archive against the index's pinned hash, installs into the mod store (ead42976), updates `mods.lock`
- [ ] `rim update` and `rim remove`
- [ ] The in-game mod manager uses the same code

## 2026-09-27

Installs go into the content-addressed mod store (ead42976), not one folder per mod, so a save can keep the versions it was made with.
