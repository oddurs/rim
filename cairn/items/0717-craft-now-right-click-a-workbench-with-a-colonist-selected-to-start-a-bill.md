---
id: 717
uid: b8e4e8b1-3598-4657-aa2f-df156d9235ae
title: 'Craft now: right-click a workbench with a colonist selected to start a bill'
type: feature
status: backlog
milestone: carrying
created: 2026-09-29
updated: 2026-09-29
priority: p1
effort: m
layer: engine
area: ai
api: additive
---

## Problem

A crafting spot or workbench offers nothing on right-click. The user wants to right-click the crafting spot to tell a colonist to craft.

## Proposal

A building with recipes gets one order option per bill it can do now ("Craft: stone knife"). The colonist starts that bill at once, fetching the ingredients and the tool it needs. The option comes from `order::options`, driven by the building's recipe defs. Doc: https://claude.ai/code/artifact/349db585-8e04-44a3-a729-f2163bd8d5f4.

## Acceptance criteria

- [ ] A test: with a colonist selected, right-click a crafting spot and pick a bill; that colonist does it next
- [ ] A bill with no ingredients in reach shows greyed, with the reason
