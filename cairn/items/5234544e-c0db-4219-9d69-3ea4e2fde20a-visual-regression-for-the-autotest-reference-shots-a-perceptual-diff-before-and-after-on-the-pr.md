---
id: 5234544e-c0db-4219-9d69-3ea4e2fde20a
title: 'Visual regression for the autotest: reference shots, a perceptual diff, before and after on the PR'
type: feature
status: backlog
milestone: proving-ground
depends_on:
- 3f381240-d490-4a6a-beb0-27f201dddb79
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: m
layer: client
area: tests
---

## Problem

The autotest takes 42 screenshots and uploads them; nothing compares them to anything, so a rendering regression passes CI.

## Proposal

- Reference images on a `baselines` branch (or Git LFS), one per shot, keyed by shot name.
- A perceptual diff per shot with its own tolerance, failing the client job over it.
- When a shot changes on purpose, a label `update-shots` accepts the new images, and the PR gets a comment with before and after.

## Acceptance criteria

- [ ] A deliberate one-pixel-wide change to a wall's contour fails the client job naming the shot (linked)
- [ ] An unchanged PR passes on two runs in a row with no false positives (linked)
- [ ] `update-shots` updates the references and the comment shows both images (linked)
