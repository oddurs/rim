---
id: 548
uid: ead42976-531c-413e-bc4c-dd830f01d038
title: 'A mod store keyed by hash: old versions stay while a save uses them, and Open as saved fetches what''s missing'
type: feature
status: backlog
milestone: platform
depends_on:
- 154
- 152
- 530
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: l
layer: tooling
area: save
pillar:
- plugin-first
- determinism
---

## Why

"Open as saved" (d137353c) works only when every sim-side mod the save used
is installed as it was. After the player updates a mod, it never is: there's
one folder per mod.

Factorio's load menu has "Sync mods with save", which downloads the save's
mods (Ctrl-click for exact versions); joining a server does the same.
Dwarf Fortress copies mods into `installed_mods/`, so a world keeps its
versions.

## What

- Installs live in a **store** keyed by content: `<data>/mods/store/<id>/<version>-<hash>/`.
  `./mods` keeps working for development as today.
- The active set or lockfile chooses which stored version loads.
- A version stays in the store while any save's last epoch, any set or any
  lockfile names it. `rim gc` removes the rest, listing what it frees.
- "Open as saved": when a save's version is missing and the index has it,
  offer "Download Wildlife+ 0.1.0 and open as saved". It is the same
  function co-op join (0127) will use.
- Bug capture bundles (fe1c23b7) embed the sim sides of their mods, so a
  report reproduces without the index.

## Acceptance criteria

- [ ] `rim add` installs into the store, and two versions of one mod coexist (test)
- [ ] A save made before a mod update opens as saved from the store with no new epoch (test)
- [ ] A missing version is fetched from a fixture index and verified by hash before use (test)
- [ ] `rim gc` keeps every version something still names, and removes the rest (test)
- [ ] A bug bundle opens with its embedded mods on a machine without them (test)
