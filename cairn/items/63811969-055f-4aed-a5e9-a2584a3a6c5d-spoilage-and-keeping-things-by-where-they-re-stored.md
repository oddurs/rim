---
id: 63811969-055f-4aed-a5e9-a2584a3a6c5d
title: Spoilage, and keeping things by where they're stored
type: feature
status: backlog
milestone: crafting
depends_on:
- 282efef8-78c1-4d2d-ad5d-4a3dd1246589
- d77d9e1f-f0ae-4e30-ae9c-95cd35c1346b
created: 2026-09-26
updated: 2026-09-26
priority: p3
api: additive
effort: m
layer: engine
area: needs
---

## Why

Storage keeps things (DESIGN.md §4f), but nothing spoils or gets wet yet, so `shelter` and a pot's keeping do nothing. Waits for its first readers: ground wetness (0187) and cooking.

## What

- Food with a spoil rate; a store's `keeps` factor (terms, so a mod can make cold storage from `temperature`); `shelter` keeps contents dry.

## Acceptance criteria

- [ ] Berries in a pot outlast berries on the ground (test)
