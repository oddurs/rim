---
id: a2e82495-9ffb-4c7e-94a3-674e41cc206c
title: Cmd with a letter types the letter into a text box on macOS
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

Unverified on a Mac (traced through miniquad 0.4.11). miniquad's macOS
`key_down` sends a `char_event` with `[event characters]` whatever the
modifiers (apple_util.rs `get_event_char`), so Cmd+K yields 'k'.
`RawInput::gather_ui` (crates/rim_client/src/main.rs) keeps every char
`typed_char` allows (it drops control characters, which is what Ctrl+K
gives, but Cmd+K gives a plain 'k'). With a text input focused, bindings
don't fire, so Cmd+K with the palette's query focused types "k" rather
than doing anything, and Cmd+A, Cmd+C and Cmd+V type letters.

Fix: in `gather_ui`, drop typed characters while Cmd or Ctrl is down (the
`ctrl` flag it already computes). main.rs is calm-forest's; route it.
