---
id: 7acef0c0-1bf8-49e2-963b-0cf24b06322b
title: 'Autotest: "no seam between joined walls" fails about one run in four'
type: bug
status: done
milestone: foundations
created: 2026-09-25
updated: 2026-09-25
closed_at: 2026-09-25
priority: p2
api: none
effort: s
layer: client
area: tests
---

## Why

`rim_client --seed 7 --autotest` failed this check once in four runs on the same tree (while building 049e2f73), then passed twice in a row:

```text
FAIL  no seam between joined walls ([0.4627451, 0.6117647, 0.63529414] vs fill [0.5882353, 0.40392157, 0.22745098])
```

The sampled pixel is pale blue, the colour of a blueprint or of UI chrome, not a wall's. So on that run, something other than the finished wall was drawn at the seam pixel. The sim is deterministic, so the difference is on the client's side: frame timing, a toast, or a worksite's live draw.

## Acceptance criteria

- [x] The cause is found and named in a note
- [x] The check passes 20 runs in a row

## 2026-09-25

First suspect: the row is chosen with no pawn within 4 cells, but pawns keep moving between the pick and the screenshot. A token walking onto the seam would explain a colour that isn't a wall's.

## 2026-09-25

Cause: the pale blue is the right-click acknowledgement ring (App::order_flash) from the right-click check earlier in the run. It fades by get_time(), which is wall clock, so on a slow frame it still covers the seam pixel when the walls are shot (08_materials.png shows it over the wall row). The autotest now clears order_flash before the shot.

## 2026-09-25

20 of 20 autotest runs green after the fix, with a full cargo test running alongside to slow frames.
