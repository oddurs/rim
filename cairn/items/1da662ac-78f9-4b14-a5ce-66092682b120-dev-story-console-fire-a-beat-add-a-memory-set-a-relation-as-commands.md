---
id: 1da662ac-78f9-4b14-a5ce-66092682b120
title: 'Dev story console: fire a beat, add a memory, set a relation, as commands'
type: feature
status: backlog
milestone: story
depends_on:
- 6202120c-27a8-4bba-a1ce-e1960d5519e7
- 801c8f78-76af-4aa0-9394-23388a7d1d3c
- e641fbdb-1f13-4fe1-b382-c37623228885
created: 2026-09-26
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: tooling
area: tests
pillar:
- determinism
---

## Acceptance criteria

- [ ] Dev commands fire a beat, add a memory, add a relation reason, reveal a secret
- [ ] Each is a logged command, so replays and hot reload hold

## 2026-09-27

Workbench (DESIGN.md §11a) builds a general dev console (6202120c-27a8-4bba-a1ce-e1960d5519e7) with dev commands (cec3efdf-dc49-4fc0-a35e-5d649efe90e6). Build this item's commands as an extension of it rather than a separate console.
