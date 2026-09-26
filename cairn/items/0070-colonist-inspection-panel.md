---
id: 4ad6b386-04fa-4237-bd8f-0075ddb6a53d
title: Colonist inspection panel
type: feature
status: done
milestone: colony
assignee: Oddur Sigurdsson
depends_on:
- c3c4d136-0740-4702-8fd9-8293fa4d65e8
created: 2026-09-22
updated: 2026-09-25
closed_at: 2026-09-25
priority: p1
api: none
effort: m
layer: client
area: ui
---

## Why

See needs, skills, health and current job.

## Acceptance criteria

- [x] Tabbed panel for the selected pawn

## 2026-09-25

Built on #95's inspector rather than a new panel: a colonist's panel gets tabs (kit.tabs, choice kept in ui.state), and other pawns keep the single view. Overview is the health and need bars plus gear (holding a tool, carrying a stack) and the mods' section slot; Skills is every skill in def order with its level, progress to the next, and which work types train it as a tooltip; Work is each work type's effective priority, with rules' why as the tooltip (view.effective from 0cb48faf). UI view.pawn gains skills, hand and carrying (additive, in the same UI API 0.6 as stances). kit.bar takes tooltip and label_width. Ui::tooltip_text is public so tests read a tooltip. Tests: crates/rim_ui/tests/inspector.rs (tabs, every skill, trains tooltip, the why tooltip under Siege); client autotest clicks through the tabs (04_inspector_skills.png).
