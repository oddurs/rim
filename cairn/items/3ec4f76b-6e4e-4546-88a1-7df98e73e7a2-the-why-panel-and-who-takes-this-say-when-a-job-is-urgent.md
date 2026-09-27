---
id: 3ec4f76b-6e4e-4546-88a1-7df98e73e7a2
title: The why panel and who-takes-this say when a job is urgent
type: feature
status: backlog
milestone: mood
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: additive
effort: s
layer: engine
area: ui
---

## Why

A job marked urgent jumps the queue, but `explain_work` shows only the pick, and `who_takes` only who and when. A player asking "why is Bo on the wall?" can't see that the mark did it.

## What

- `explain_work` rows gain `urgent`, and `why_text` reads "urgent: a level sooner" for a pick that came from a mark.
- `rim.who_takes` rows and the map's hover forecast say "urgent" when the job is marked.

## Acceptance criteria

- [ ] With a Later wall marked urgent, the why panel's pick names the mark (UI test)
- [ ] The hover forecast over the marked wall says urgent
