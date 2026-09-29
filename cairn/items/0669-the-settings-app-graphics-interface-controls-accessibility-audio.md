---
id: 669
uid: afc3e3b0-be25-4170-8d79-2f60b4c846b2
title: 'The Settings app: Graphics, Interface, Controls, Accessibility, Audio'
type: feature
status: backlog
milestone: rimos
depends_on:
- 586
created: 2026-09-28
updated: 2026-09-29
priority: p1
api: additive
effort: m
layer: core
area: ui
---

## Problem

Every setting is a palette command or a file key (settings.rs). There is no settings screen, and the title screen ignores setting actions.

## Proposal

Few options, on purpose. The user's direction: performance is the number one feature, and anything in its way goes; "in rare cases we give people options in settings". The fastest look is everyone's look, so a setting earns its place only when players' machines or bodies genuinely differ.

One app with a page per area and per mod (`ui.settings_page`), the same on the title and in a colony:
- Graphics: render scale, window mode.
- Interface: UI scale, reset window positions.
- Controls: scroll, and the keybind editor (48f96f56).
- Accessibility: reduce motion, text size, colour sets. These are the rare cases: low vision and colour blindness differ between players, and neither costs anything at runtime.
- Audio: its slots reserved until rim has sound.

Left out:
- Lighting: one lighting for everyone, the user's call (08a5d182).
- vsync, a toasts toggle, edge pan, camera speed: each has one right answer.

Changes apply at once and write settings.toml as today. Overlaps 05033e73 (Settings in 1.0); the user decides whether that item moves here.

## Acceptance criteria

- [ ] Every key in settings.toml is reachable from the app (autotest walks the pages)
- [ ] Changing a setting on the title takes effect (title.rs no longer drops them)

## 2026-09-28

Trimmed to the user's 'rarely options' direction (relayed by rim-c2): lighting, vsync, toasts, edge pan and camera speed are out; text size and colour sets stay as accessibility; see the proposal for why.

## 2026-09-29

PAUSED: not started; calm-forest's when work resumes. Seam agreed with green-forest (DESIGN §11b, docs/modding/ui.md): kind="screen" windows; title-layer acts new_colony/load/continue/delete_save/open/quit and views saves/starts/settings/loading. The engine delivery is green-forest's; the client side (main.rs, title.rs, play.rs) is calm-forest's. Waits on green-forest's title-layer delivery and leave-to-title (play.rs). Rule from the user: no per-frame cost when idle, a fast first frame, settings only in rare cases (#388 trimmed afc3e3b0).
