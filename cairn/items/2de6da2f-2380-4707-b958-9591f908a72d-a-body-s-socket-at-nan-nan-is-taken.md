---
id: 2de6da2f-2380-4707-b958-9591f908a72d
title: A body's socket at [nan, nan] is taken
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
claimed: 2026-09-28
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: modding
---

`body()` in crates/rim_ui/src/body.rs reads a socket given as `[x, y]`
with `as_float` and no finiteness check, unlike a part's numbers (`num`
refuses NaN and infinity). TOML allows `nan` and `inf`, so
`sockets = { hand = [nan, 0.1] }` loads, and whatever is held there is
drawn at NaN (nowhere). Fix: the same finite check, and say which socket.
