---
id: 560
uid: 08a5d182-e6ff-415c-8619-2c6accc5a55e
title: Flat is the default lighting, and today's look is an opt-in quality
type: feature
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-29
priority: p0
api: breaking
effort: m
layer: client
area: render
---

## Why

The user chose option B in the BARE METAL plan (3c65738f): the lighting is slow on the Mac. Medium drops the bench's later views from 120 to about 60 fps, while flat holds 120 in every view. So the default is `flat`, and today's look is a quality the player opts into, knowing its cost.

## What

- `flat` is the default preset: an unset `[lighting]` and a new install run it. It is the sim's `light` field, tinted by the sky and firelight colours, in one multiply (DESIGN.md §6e).
- ONE opt-in, `shadows` (today's medium look), labelled with its cost in the palette and the docs. `low`, `high`, `ultra` and `auto`, the per-setting tuning, and the multi-body sun pass only they used are deleted (the user's rule, via rim-c2: performance first, options only in rare cases).
- The render bench measures the default, and reruns each view under `medium` beside it, so both stay tracked on one runner.
- The autotest's lighting sections pin the preset they test.
- The sim is untouched: light stays the sim's field.

## Acceptance criteria

- [x] With no settings file, the game and the bench run `flat` (test)
- [x] The other tiers, auto and the per-setting tuning are gone, and there is no lighting choice in the game; `shadows` is reached only through the settings file, as a bridge (test)
- [ ] Before and after on a same-runner-class CI A/B, and a 5-minute foreground Mac run of the default, with the user's OK

## 2026-09-28

Scope, per the user's rule (performance first; options only in rare cases; via rim-c2): flat by default, ONE opt-in (`shadows`, today's medium look), and the rest deleted.
Gone: low, high, ultra; auto (the timer-query ring, the cost watch, its 6 tests); the six per-setting overrides; and the multi-body sun pass (four marches into RGBA and four direct terms), which only high and ultra used. Under shadows the pass marches the one brightest body: the sun by day, the moon by night (tested on core's real sky).
Settings: [lighting] quality is flat or shadows. An old tier name warns and runs flat; old tuning keys are ignored, so an upgrade keeps a valid quality, and save_lighting rewrites the table clean.
The bench measures the default and reruns each view under the other setting on the same runner. The autotest pins shadows for its lighting sections (agreed with green-forest) and adds flat_is_the_default.

## 2026-09-28

act.lighting now accepts only flat or shadows, a breaking change to the UI API, so ui_api moves from 0.6 to 0.7 (UI_API_VERSION, every mod's ui_api line, the docs), with no compat shim (rim-c2's call).

## 2026-09-28

Folded in, at rim-c2's call so ui_api is bumped once: act.lighting, the two palette bindings, the handler and save_lighting are removed; the user approved one lighting for everyone. [lighting] quality stays readable, a bridge to shadows until f05c5fa1 and 3a2b2c0d.

## 2026-09-29

Criteria 1 and 2 ticked with the PR: quality::flat_is_the_default_and_shadows_the_one_choice and the autotest's flat_is_the_default. Criterion 3 stays open: the CI side-by-side comes from this PR's queue run, and the 5-minute foreground Mac run needs the user's OK. Rebased over #386: api 0.8 and ui_api 0.7 in every mod.
