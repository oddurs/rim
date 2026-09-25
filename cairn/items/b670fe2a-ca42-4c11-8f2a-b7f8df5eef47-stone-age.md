---
id: b670fe2a-ca42-4c11-8f2a-b7f8df5eef47
key: stone-age
title: Stone age
type: milestone
status: planned
created: 2026-09-25
updated: 2026-09-25
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
