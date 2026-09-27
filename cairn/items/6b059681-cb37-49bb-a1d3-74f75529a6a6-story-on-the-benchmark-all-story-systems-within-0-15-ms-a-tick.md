---
id: 6b059681-cb37-49bb-a1d3-74f75529a6a6
title: 'Story on the benchmark: all story systems within 0.15 ms a tick'
type: perf
status: backlog
milestone: story
depends_on:
- 0721dbea-0a23-40ea-bcbf-8ac9bb5feec9
- 308d1aab-d9f9-4a74-aa5b-e7ed6698bbd9
- aa6b7157-7e7f-4339-8eec-8f21684a1516
- e641fbdb-1f13-4fe1-b382-c37623228885
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: none
effort: m
layer: tooling
area: perf
pillar:
- performance
---

## Acceptance criteria

- [ ] The bench's target map runs with mood, social, mourning and story on
- [ ] Per-system timings for perception, memories, interactions, relations, contagion, reminders, threads
- [ ] `bench --check` fails over 0.15 ms mean for story systems together
