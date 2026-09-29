---
id: 600
uid: 3a2b2c0d-3433-4a14-bad6-5dc45a51819e
title: 'One lighting for everyone: delete the shadows setting, its firelight bake, moving-light shadows and room fill'
type: chore
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-29
priority: p2
api: none
---

## Why

The user approved one lighting for everyone (via rim-c2, 2026-09-28): flat, with the sky body's shadows drawn as shapes (f05c5fa1). Everything else under `shadows`, the bridge 08a5d182 kept, then has no reason to exist, and each piece of it is GPU time or code to keep.

## What

- Delete `Setting::Shadows` and `[lighting] quality`: the quality module goes, and an old settings file's `[lighting]` table is ignored.
- Delete what only `shadows` runs: the firelight bake and its soft shadows, moving-light shadows, room fill, the light-resolution texture and its texel ladder, the multiply pass, and the marched sun pass (SUN_FRAGMENT, SunKey, update_sun, sunlit), which f05c5fa1's shapes replace.
- The autotest's lighting sections move to flat; the bench's `other` rerun goes.

## Acceptance criteria

- [ ] One lighting path: no quality setting, no bake, march, fill or multiply pass left (test)
- [ ] The lines removed are stated in the PR, and the bench shows no view slower on the same runner class

## 2026-09-29

PAUSED: done: scope and criteria written (on feat/f05c5fa1-shadow-geometry); left: all of it, no branch yet; next step: after f05c5fa1 merges, branch chore/3a2b2c0d-one-lighting from main and delete Setting::Shadows with everything only it runs, including the marched sun pass (f05c5fa1's criterion 4).
