---
title: "Rules"
sidebar_position: 6.49
---

Eldiron rules are now split into two docs.

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
