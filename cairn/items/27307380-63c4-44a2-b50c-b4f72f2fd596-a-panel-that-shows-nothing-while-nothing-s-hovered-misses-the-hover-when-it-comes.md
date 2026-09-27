---
id: 27307380-63c4-44a2-b50c-b4f72f2fd596
title: A panel that shows nothing while nothing's hovered misses the hover when it comes
type: bug
status: done
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

Moving the pointer from a panel onto the map, the hover readout (core:hover) can take up to its rebuild period to appear. `Vm::build` records which mounts read the hover only when the mount returned a node. core:hover returns nothing while nothing is hovered, so it isn't recorded as a hover reader, and when the pointer arrives the incremental rebuild (#192) skips it until its cadence comes round. Found by the Chalkline autotest, whose timing exposed it: "hovering the world shows the readout" failed.

## What

- `Vm::build` notes what a mount read whether or not it returned a node. A mount whose component was removed never runs, so it reads nothing either way.

## Acceptance criteria

- [x] A test in rim_ui: a mount that returns nothing without a hover shows the hover on the very next frame that has one, inside one rebuild period
- [x] The client autotest's "hovering the world shows the readout" passes on the Chalkline zones branch
