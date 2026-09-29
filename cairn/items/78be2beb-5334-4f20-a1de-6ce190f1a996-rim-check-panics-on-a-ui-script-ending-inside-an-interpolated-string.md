---
id: 78be2beb-5334-4f20-a1de-6ce190f1a996
title: rim check panics on a UI script ending inside an interpolated string
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: tooling
area: modding
---

## What

`member_refs` (crates/rim_ui/src/check.rs), the scanner `rim check` runs
over every UI script, recurses into an interpolated string's `{...}` with
`&src[start..i.saturating_sub(1)]`. When the brace never closes, `i` is the
end of the file, and the slice is either reversed or cut through a
character.

## How it fails

A UI script that ends in `` `{ `` (start past end: `slice index starts at N
but ends at N-1`) or `` `{é `` (end inside a UTF-8 character) makes
`rim check` panic instead of naming the file. Mid-edit files are exactly
what a modder runs `rim check` on.

## Fix

An unclosed brace scans to the end of the file and no further.
