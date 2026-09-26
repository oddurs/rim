---
id: b670fe2a-ca42-4c11-8f2a-b7f8df5eef47
key: stone-age
title: Stone age
type: milestone
status: done
assignee: Oddur Sigurdsson
created: 2026-09-25
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: additive
---

Hands first. The colonist wakes with nothing (pillar 93f291d5), and today nothing asks what they hold: a naked warrior fells an oak and quarries granite bare-handed. This milestone makes the first days a climb. You gather branches, fibre, stones and berries with your hands. You find flint and knap it into a flake and a hand axe on the ground. Branch walls and a campfire see you through the first night. A digging stick opens the clay banks, for warm cob walls and fired pots. Then the axe fells trees and a stone maul quarries rock.

The engine gets four mechanisms and no content:
- several harvests on one thing (4e9d5a12);
- tools held in hand, which gate work, speed it up and wear out (7016d86b);
- work orders that bring things to a place and then work there, a generalisation of blueprints that is open to Luau (74b6fa7e);
- selecting and inspecting things, so a station can show its bills (5305a161).

Core owns only the shared names (ccd44902). Crafting is a plugin: recipes, stations and bills (049e2f73). The stone age is a plugin: materials, tools, recipes and gates (e3846c47). A seed sweep proves the first three days still work (4bd94457).

Core without the plugins plays as it does today (DESIGN.md §5). Design: DESIGN.md §4e.

## 2026-09-26

Landed on 2026-09-26. Each item was its own PR, reviewed before it went up and gated with the CI commands:
- 4e9d5a12 (#89), ccd44902 (#91), 5305a161 (#95), 7016d86b (#101), 74b6fa7e (#104) and 049e2f73 (#107): the mechanisms, core's shared names and the crafting plugin.
- 3ccab46f (#110), 4675019b (#113), af17cc1d (#116), 89f6d138 (#119), 61c93a4e (#120), e3846c47 and 4bd94457 (#122): carried materials, the stone age and its balance.
Measured, seeds 1 to 20: shelter before the first night in 90%, a flint tool by day 2 in 100%, a felled tree by day 3 in 100%, cob by day 4 in 90%. Follow-ups filed elsewhere: 21fc9ea3 (lone colonies die by day 45), 8b2e05db (urgency in work choice), ca22f222 (ingredient filters) and ff97096d (building over items).
