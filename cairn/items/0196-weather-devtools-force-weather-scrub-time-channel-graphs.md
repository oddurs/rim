---
id: dc32fa23-2b01-4682-b41a-2e18c0561516
title: 'Weather devtools: force weather, scrub time, channel graphs'
type: chore
status: done
milestone: sdk
depends_on:
- 7c50b502-5e27-4807-a36c-0654fe9aec97
- 6dd4891c-f65e-4707-96f9-e6d46c6cd446
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: none
effort: s
layer: tooling
area: tests
pillar:
- plugin-first
---

## Why

Tuning a year by playing it is hopeless. Modders and we need to force weather, scrub time and read a year in seconds.

## What

- F12 devtools "Weather" tab: force any regime, scrub hour and day of year, pause the sim clock while the weather runs, a 3-day graph of every channel, and the breakdown under the cursor.
- `rim --climate-report [--seed N] [--years N]`: runs headless and writes `target/climate/*.csv` (hourly channels, daily ground means, regime spans, plant counts) and a markdown summary.
- CI runs a one-year report and uploads it as an artifact.

## Acceptance criteria

- [x] Forcing a regime and scrubbing the hour work from devtools
- [x] The climate report runs a year in under 30 s in release
- [x] CI uploads the report

## 2026-09-23

Moved to SDK. The headless year report moved into 0198 (balance) as an example; what's left here is the devtools tab.

## 2026-09-24

F12 opens a Weather devtools panel (mods/weather/ui/devtools.luau): force any type for 12 h, skip ahead 1 h / 6 h / 1 day / 1 season, and a 72-hour strip per channel (cloud, rain, wind, fog) drawn from the forecast queue. Two new general mechanisms carry it: Command::ModEvent and act.send(name, table) (a mod's UI sends an event to its own sim scripts through a command, so it replays and stays in lockstep; namespaced by the calling UI code's mod) and act.advance(hours) (client-side, devtools). The weather plugin publishes its types (weather:types) and handles weather:force. Tests: the_weather_can_be_forced_by_a_command, a_mods_ui_sends_only_its_own_events; autotest clicks force storm and +1 day. The climate report is examples/year.rs (a year in about 8 s in release); CI now uploads it as the climate-report artifact. Hour scrubbing is forward-only: rewinding needs saves (0060).
