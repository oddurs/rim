---
id: f42fd04a-9454-40b7-99ec-7603c00def95
title: 'GitHub Action for mod repos: check and test against supported engine versions'
type: feature
status: backlog
milestone: sdk
depends_on:
- 6d24a1ec-85be-4317-a62a-4d6f049f8f2d
- be5845a1-bb0d-4c13-afa7-ac5d87f62d5d
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: modding
pillar:
- plugin-first
---

## Why

Every mod repo gets CI on day one without writing any YAML (DESIGN.md §10).

## Acceptance criteria

- [ ] `uses: <org>/rim-mod-action@v1` installs the CLI and runs `rim check` and `rim test`
- [ ] Matrix over the engine versions allowed by the mod's `api`
- [ ] Annotates failing defs and scripts inline on the PR
- [ ] Included in the `rim new` template
