---
id: 99648d58-585a-45e2-a26e-88d6f1022310
title: 'Palette and text input: padding is lost in layout, arrows type garbage'
type: bug
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: none
---

## What happens

## What should happen

## Reproduction

Seed: any
Mods: core
Tick: any

1. Press Ctrl+K, type, press Down.

## Why

The command palette showed two problems on first use: the query box was
only a line tall, so its padded text was clipped, and pressing Down typed a
stray character instead of moving through the list.

## What

- A text leaf measures with its padding and draws inside it, so an input
  has room around its line and a padded label is a padded label.
- The client drops the private-use characters macOS sends for arrow, home,
  end and function keys through the text path.
- Up and Down reach an input as `on_key`; the palette moves a selection
  with them and Enter runs it. `ui.set_input` resets the query on open.

## Acceptance criteria

- [x] An input box is taller than its line by its padding, and the text starts after it
- [x] Down in the palette moves the selection and types nothing; Enter runs the selected row

## 2026-09-24

Root causes: layout.rs measured text leaves by their shaped size alone (taffy adds nothing for a measured leaf), and paint drew inputs offset by the padding the box never had; the client filtered control characters but macOS reports function keys as U+F700 range characters in the same char stream. Padding now applies to every text leaf, in layout and paint alike; a wrapped text wraps inside it.
