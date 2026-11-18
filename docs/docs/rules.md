---
title: "Rules"
sidebar_position: 6.49
---

Eldiron rules are documented from five perspectives.

## Build With The Supplied Rules

Eldiron supplies one integrated RPG ruleset for small adventures and open
sandboxes. Browse the supplied characters and items, place content and resources
in the world, and choose player starting options. You do not need to enable each
equipment family or edit its recipes. Content enters the game through placed
items and obtainable crafting materials and knowledge.

The generic ruleset editor is internal maintenance tooling and is hidden in
standard Creator builds. Project-owned rules still save, load and run normally.

For ruleset-driven artwork, [Appearance](characters_items/appearance) explains
how to equip characters and edit MZ item icons, models, clothing and independent
On/Off states. Gameplay definitions stay in the supplied ruleset; appearance recipes use the
project palette and their own editors.

## Official Rules

Read [Official Rules](./official_rules) if you want the rulebook view: races,
classes, attributes, combat, weapons, spells, progression, visuals, and the
future crafting model.

This is the player-facing and creator-facing gameplay guide.

## Rules In Eldiron

Read [Rules In Eldiron](./rules_in_eldiron) if you want the implementation
view: how project rules are stored and resolved, how internal maintenance
preserves definitions and checkpoints, how item templates are
created, and how to test rules from the terminal or Creator console.

## Ruleset Contract

Read [Ruleset Contract](./ruleset_contract) for the target public ruleset
architecture: official, derived, and standalone rulesets; optional modules;
canonical data ownership; composable effects; packages; assets; migrations; and
documentation synchronization.

## Ruleset Capability Audit

Read [Ruleset Capability Audit](./ruleset_capability_audit) for the
implementation baseline. It distinguishes rules that are complete, functional,
partial, catalogue-only, or missing across runtime, Creator, validation, tests,
and documentation.

## Short Version

New projects use the bundled `eldiron.official` ruleset by default.

The generic ruleset editor is hidden in standard Creator builds. Projects own
their complete current rules document and preserve their original
rules for restoration. Inherited deltas and upgrade workflows remain future work.

During ruleset development, `test_projects/Hideout2D.eldiron` follows the
current representation. The starter copy remains frozen for the released
version and is replaced from the tested project only as part of a release.
