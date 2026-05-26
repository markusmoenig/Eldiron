# Eldiron Official Ruleset v1

This is the first bundled official ruleset for Eldiron.

It defines the initial v1 gameplay baseline with explicit tables and dice-style
values: core attributes, Human and Warrior defaults, progression, damage kinds,
weapon and armor categories, starter equipment, cooldowns, abilities, spells,
actions, procedural FX presets, audio/message hooks, and default humanoid visuals.

Project-specific `Game / Rules` content is treated as an override on top of this
official base.

The ruleset is authored as several TOML files so it stays readable as the
simulation grows:

```text
ruleset.toml            metadata, schema, bundled assets, visuals, palette, skills, resources
identity.toml           default identity, dispositions, race relations, intents
attributes.toml         attributes and derived stats
progression.toml        XP, leveling, progression messages
combat.toml             damage kinds, combat timing, combat audio/messages
messages.toml           locale keys for rules-driven runtime feedback
locales.toml            English defaults for ruleset-owned locale keys
equipment.toml          equipment slots, categories, weapons, armor, clothing, resource nodes
fx.toml                 semantic procedural FX presets for spells and actions
actions.toml            sandbox-facing action definitions
recipes.toml            skill-gated crafting and preparation recipes
abilities_spells.toml   abilities and spells
races_classes.toml      races, classes, unlocks, starting loadouts
```

At compile time, shared code embeds these parts and exposes them as one effective
official ruleset to Creator, clients, tools, and tests.

The default visual avatar is referenced as `humanoid` and stored at:

```text
assets/humanoid.eldiron_avatar
```

Explicit character `tile_id`, `avatar`, or `avatar_id` values override the
ruleset visual default. Explicit empty visual attributes can be used to disable
the inherited default.
