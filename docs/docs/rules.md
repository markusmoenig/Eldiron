---
title: "Rules"
sidebar_position: 6.49
---

Eldiron rules are documented from four perspectives.

## Official Rules

Read [Official Rules](./official_rules) if you want the rulebook view: races,
classes, attributes, combat, weapons, spells, progression, visuals, and the
future crafting model.

This is the player-facing and creator-facing gameplay guide.

## Rules In Eldiron

Read [Rules In Eldiron](./rules_in_eldiron) if you want the implementation
view: where the ruleset TOML lives, how it is embedded, how **Game / Settings**
selects a version, how **Game / Rules** overrides work, how item templates are
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

**Game / Settings** selects the ruleset:

```toml
[ruleset]
id = "eldiron.official"
version = "1.0.0"
schema_version = "1"
source = "official"
update_policy = "compatible"
```

**Game / Rules** is normally an empty override layer. Add only project-specific
changes there.

During ruleset development, `test_projects/Hideout2D.eldiron` follows the
current representation. The starter copy remains frozen for the released
version and is replaced from the tested project only as part of a release.
