---
id: 456
uid: 276c8888-a17c-40f7-a671-8fa5c35e86f6
title: Core declares its bare tags; plugins namespace their own
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: breaking
effort: m
layer: core
area: modding
pillar:
- plugin-first
---

## Why

Tags are how plugins meet without depending on each other, but ownership is
unclear:

- Core's vocabulary tags are bare: `bed`, `fire`, `seat`, `bulky`,
  `chopping`, `digging`, …, listed by hand in docs/modding/vocabulary.md.
- Plugins add bare tags no vocabulary lists: primitive's `knappable`,
  `fibre`, `cordage`, `vessel`, `stone_raw`; iron's `ore`, `metal`, `part`;
  `fuel` in both; core's own `water` isn't listed either.
- Other plugin tags are namespaced: `crafting:fire`, `crafting:hand`,
  `iron:forge`.
- A misspelt tag (`choping`) matches nothing, silently.

Minecraft namespaces tags, and Fabric's `c:` convention tags became the
shared vocabulary every mod meets through. That is core's role in rim
(DESIGN.md §5 rule 2).

## What

- A new built-in kind, `[[tag]]`: `id` (bare), `label`, `means`. Only core
  declares bare tags.
- Core declares every tag in its vocabulary, plus `water`, and promotes the
  cross-plugin ones: `ore`, `metal`, `fuel`.
- Plugins namespace the rest: `primitive:knappable`, `primitive:fibre`,
  `primitive:cordage`, `primitive:vessel`, `primitive:stone_raw`,
  `iron:part`. Their defs, scripts, order needs, stock queries and filters
  update with them.
- `rim check` warns on a bare tag core doesn't declare, naming the file and
  suggesting a near match.
- docs/modding/vocabulary.md's tag tables are checked against the `[[tag]]`
  defs by a test, so they can't drift.

## Acceptance criteria

- [ ] `[[tag]]` loads; a bare tag declared outside core is a load error (test)
- [ ] Core declares its vocabulary tags plus `water`, `ore`, `metal` and `fuel`
- [ ] Primitive's and iron's other tags are namespaced, and `rim check --strict` passes on every first-party mod
- [ ] A misspelt bare tag warns with a suggestion (test)
- [ ] A test fails if vocabulary.md's tag tables and the `[[tag]]` defs disagree
- [ ] The determinism test and every mod's `rim test` pass
