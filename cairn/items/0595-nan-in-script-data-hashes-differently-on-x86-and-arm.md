---
id: 595
uid: 3454c7de-550e-4ef6-82e4-0c9fd7a6a5ae
title: NaN in script data hashes differently on x86 and ARM
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: engine
area: scripting
---

## What

`Data::hash` (data.rs ~150) hashes `n.to_bits()`. `rim.set_data("k", 0/0)` stores the hardware's default NaN, whose bits differ by architecture: x86 SSE makes `0xFFF8…`, ARM64 makes `0x7FF8…`. So `state_hash` differs between a Mac and an x86 machine running the same game. The text save form also rejects non-finite numbers (data.rs ~48), so `rim save unpack` fails for the whole save.

## How it fails

- Crosscheck and co-op agree runs disagree on any mod that stores a NaN.
- `savetext` can't unpack such a save.

## Reproduce

Not yet run; verified by reading. A unit test in data.rs: `Data::Num(f64::from_bits(0x7ff8 << 48)).hash(0)` should equal the hash for `0xfff8 << 48`.

## Fix

Canonicalise NaN (and -0.0, if the text form matters) where script numbers enter `Data`, or refuse non-finite numbers at `from_lua`.

## Acceptance

- [ ] One NaN hashes one way on every platform, and a save holding one round-trips its text form, or is refused at `set_data`
- [ ] A test that fails before the fix and passes after
