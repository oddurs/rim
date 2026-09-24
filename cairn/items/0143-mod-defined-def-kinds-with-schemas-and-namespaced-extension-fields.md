---
id: b3df9f85-2cf7-4885-90e9-c736b606b486
title: Mod-defined def kinds with schemas, and namespaced extension fields
type: feature
status: backlog
milestone: plugin-api
depends_on:
- be8174f0-ff41-44fe-b788-2ffff60e0d19
created: 2026-09-23
updated: 2026-09-24
priority: p0
api: additive
effort: l
layer: engine
area: modding
pillar:
- plugin-first
---

## Why

Mood needs `thought` defs; crafting needs `recipe` defs. Neither belongs in the engine. And a mod can't hang its own data on another mod's def in a way its scripts can read.

## What

A mod declares a kind:

```toml
[[kind]]
id = "thought"
fields = { label = "string", mood = "int", days = "float", stack = { type = "int", default = 1 } }
```

Any mod can then write `[[mood:thought]]` entries and patch them, with conflict detection, like built-in kinds. Data attached to another mod's def goes under a table named after the attaching mod: `[creature.core:human.mood]` reads as `def.mood` in scripts.

## Acceptance criteria

- [ ] `[[kind]]` with typed fields, defaults and references to other kinds
- [x] Defs of mod kinds merge, patch and report conflicts like built-in ones
- [ ] Read-only access from Luau: `rim.defs("mood:thought")`, `rim.def("core:human").mood`
- [ ] Extension tables on other mods' defs are validated against the owning mod's schema
- [ ] Unknown top-level fields become a warning (extension tables replace them)

## 2026-09-24

0208 landed [[kind]] with typed fields and defaults, merge/patch/conflicts through the built-in path (criterion 2), and rim.defs(kind). Still open: references to other kinds in field types, rim.def("core:human").mood and extension tables on other mods' defs, and unknown-field warnings for built-in kinds (mod kinds already warn).
