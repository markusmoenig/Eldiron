---
title: "Rules"
sidebar_position: 6.49
---

Eldiron rules are documented from five perspectives.

## Editing Your Ruleset

Read [Ruleset Editor](./creator/docks/ruleset_nodes) for the Creator workflow:
changing values, adding or removing definitions, replacing icons, editing particle
effects, and restoring the original ruleset.

## Official Rules

Read [Official Rules](./official_rules) if you want the rulebook view: races,
classes, attributes, combat, weapons, spells, progression, visuals, and the
future crafting model.

This is the player-facing and creator-facing gameplay guide.

## Rules In Eldiron

Read [Rules In Eldiron](./rules_in_eldiron) if you want the implementation
view: how project rules are stored and resolved, how **Game / Rules**
edits definitions and restores checkpoints, how item templates are
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

**Game / Rules** opens a searchable catalogue and forms for editing definitions.
Projects own their complete current rules document and preserve their original
rules for restoration. Inherited deltas and upgrade workflows remain future work.

During ruleset development, `test_projects/Hideout2D.eldiron` follows the
current representation. The starter copy remains frozen for the released
version and is replaced from the tested project only as part of a release.
