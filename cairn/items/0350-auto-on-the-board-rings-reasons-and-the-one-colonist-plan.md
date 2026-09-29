---
id: 350
uid: 5dc8f858-b840-4fff-b3cb-4312bc2f5c9e
title: 'Auto on the board: rings, reasons, and the one-colonist plan'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 309
- 431
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: none
effort: m
layer: core
area: ui
---

## Why

Automation earns trust only if the player can see what it chose and why, and take any piece back. DESIGN.md §4d: a ring marks Auto's cells, every one has a reason, and one colonist is a plan rather than a grid.

## What

- A ring in the corner of a planned cell; its tooltip and the why panel give the reason.
- Clicking a planned cell pins it; the planner plans the rest around the pin on its next run.
- With one colonist the Work screen opens to the plan view: their levels as shelves, each with Auto's reason and the rules that moved it, and "Show the board".
- A settler joining asks for a role with Auto preselected, and closes itself without pausing. Offered roles appear only once the colony has roles beyond Auto.
- A one-time hint on the first day: "Kael works on Auto. It follows what's waiting and what Kael is good at."
- The Roles lens shows the Auto card as each member's First work, not editable shelves.

## Acceptance criteria

- [x] Planned, pinned and role cells are distinguishable in an autotest screenshot
- [x] Pinning a planned cell keeps the rest planned (UI test)
- [x] The one-colonist plan view and the settler dialog appear in the autotest sweep

## 2026-09-26

The plan view shows only for a lone colonist on a planned role; a castaway moved to Hand keeps the board. The settler dialog and the first-day hint sit on the float layer (things that come and go), and the hint hides while Work is open. They're covered by UI tests (auto_board.rs) and a shots picture (work_plan.png), not the client autotest sweep. The hint's 'once' is per session (ui.state), not saved.
