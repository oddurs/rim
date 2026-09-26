---
id: 580f2fbf-4578-4f0e-9f0f-8a0793f4d15e
title: 'Screens as sheets: Work, Storage and mods'' screens share one place'
type: feature
status: review
milestone: interface
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: m
layer: engine
area: ui
---

## Why

Work, Stockpiles and the palette are floating windows. They cover the colony, stack on each other, and each mod screen invents its own size and place.

## What

- A window option `sheet = true`: the engine places a sheet between the docked columns, and only one sheet is open at a time; opening another closes the first.
- Work (`core:work`, ids kept) and Stockpiles become sheets.
- The status bar lists the screens with their keys, built from a registry (`screens.add{ id, label, key }`), so a mod's screen appears there without touching core.

## Acceptance criteria

- [x] Opening a sheet closes the one that was open
- [x] A sheet never covers the people or the Now column
- [x] A mod adds a screen with one `screens.add` call and gets a key and a button
- [x] The Work Board tests pass unchanged

## 2026-09-26

Sheets reuse window records: each frame, before the build, a sheet's record is set from last frame's rim:center band (so the chrome is built at the right size and hit-testing is unchanged). One open at a time in apply_window_op. Registry mods/core/ui/screens.luau: binding + dock button (core:dock.right). Work, Stockpiles, News are sheets.
