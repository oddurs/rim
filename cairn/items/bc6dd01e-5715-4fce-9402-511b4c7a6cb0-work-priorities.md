---
id: bc6dd01e-5715-4fce-9402-511b4c7a6cb0
key: work
title: Work priorities
type: milestone
status: planned
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
---

Who does what, without a spreadsheet. New colonists start on Auto and the colony plans its own work. The player takes control one rung at a time: Focus for the colony, Urgent for one job, a pin for one cell, work roles for a group. Design: DESIGN.md §4d.

## Goal

On the default install, a new player who never opens the Work screen still has a castaway who forages when food is low and builds the shelter that is waiting, and a colony of six that moves someone onto hauling when loose items pile up. An expert pins cells, makes a Builder role from two colonists, marks a wall urgent before a raid and switches the default orders off, all on one board where every cell says who set it and why. Every piece is data or a replaceable script: a mod adds work types, roles, stances, orders, a planner or a lens.

## Order

1. **Named levels and pins.** First, Soon, Later, Spare time; pins drawn as the player's; clicking back hands a cell back. Worth shipping alone.
2. **Lens registry.** The board, the ranked view and later lenses register like screens.
3. **Work roles.** `[[work_role]]`, seeded into the save, the commands, the role part in `explain`.
4. **Roles on the board.** Rows grouped by role, the Roles lens, "make a role from these two".
5. **Colony readings and standing orders.** `rim.set_reading`, `when.reading` with a band, `SetRuleEnabled`, events.
6. **Auto, the mechanism.** Planned roles, planned levels with the two-plan rule, `rim.planner` and `rim.work_board`, `auto` on work types.
7. **Core's planner.** `mods/core/scripts/auto.luau`: need, gap, fill, rest.
8. **Core's default orders.** Food is low, loose items piling up, firewood for winter, and the readings behind them.
9. **Auto on the board.** Rings and reasons, the one-colonist plan view, a settler joining.
10. **The orders panel and Focus on the HUD.**
11. **Urgent marks.**
12. **Does Auto play well?** A balance run against flat defaults and a tuned grid.
13. **Modding docs** for all of it.

## Not in this milestone

- Passion, the schedule strip, dragging columns for the tie-break, and hover lighting a column's jobs: filed under Mood as Work Board items, and they fit this board unchanged.
- Rules gated on alerts and Luau predicates (`99398b15`): Plugin API. Readings here are the general mechanism it can build on.
- Headcount as a sim rule: a lens can show headcounts as a mod; an engine solver waits until players ask for one.

## Due

Not set. Where it sits against Colony and Mood is a person's call.
