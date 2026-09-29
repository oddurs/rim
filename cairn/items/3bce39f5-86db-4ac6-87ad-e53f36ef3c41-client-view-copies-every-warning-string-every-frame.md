---
id: 3bce39f5-86db-4ac6-87ad-e53f36ef3c41
title: client_view copies every warning string every frame
type: perf
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: perf
---

Unmeasured. `client_view` (crates/rim_client/src/main.rs) builds
`warnings: s.warnings.iter().cloned().chain(app.ui.warnings()).collect()`
each frame, and `Ui::warnings` itself clones theme, VM warnings and errors;
only the profiler reads them. Build it when the profiler is open, or hand a
reference.
