---
id: 69494df0-0770-44e4-85ae-6d30a5b8497e
title: Strings through one door
type: chore
status: done
milestone: colony
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: additive
effort: s
layer: core
area: ui
---

## Why

Every label is a literal in Luau. Localisation is not on the roadmap and
is not built here; routing text through one function costs an hour now
and a week in a year.

## What

- `t("core:toolbar.cancel")` returns the key's default for now; core's
  UI scripts use it for every user-visible string.
- The engine exposes `ui.t`; a language file can back it later.

## Acceptance criteria

- [x] Core's UI has no user-visible string literal outside `t(...)`
- [x] A test lists every key core uses

## 2026-09-24

Done. ui.t(key, default) is the door: each mod's ui/lang.toml is read in load order before any script runs, keys are flat strings, the last mod in load order wins and a second mod setting the same key is reported as a UI conflict like a theme token. Every user-visible string in hud, devtools and labels goes through it; formats stay whole so a translator gets a sentence. The kit takes text from callers and was left alone. Three headless tests: the scripts' 52 keys against what a frame asks for; a scratch mod overriding a string and two mods conflicting; a scan that fails on any bare literal a player would read. No regex crate. Found on the way: the typed UI surface (api.rs, types/ui.d.luau, docs/modding/api-ui.md, checked by tests/api_types.rs) already exists from the other agent's work, so ui.t is declared there and the generated files regenerated -- the Luau-types item is further along than the plan assumed. Also: the hot-reload test edited the literal 'Wealth ' in a scratch HUD, which this pass turned into 'Wealth %d'; the test now edits the format.
