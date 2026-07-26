# Eldiron Official Ruleset v1

This is the first bundled official ruleset for Eldiron.

It defines the initial v1 gameplay baseline with explicit tables and dice-style
values: core attributes, Human and Warrior defaults, progression, damage kinds,
weapon and armor categories, starter equipment, cooldowns, abilities, spells,
actions, procedural FX presets, audio/message hooks, resource gathering,
multi-stage ritual crafting, and default humanoid visuals.

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
economy.toml            copper/silver/gold currency table and base unit
messages.toml           locale keys for rules-driven runtime feedback
locales.toml            English defaults for ruleset-owned locale keys
equipment.toml          equipment slots, categories, weapons, armor, clothing, resource nodes
fx.toml                 semantic procedural FX presets for spells and actions
icons.toml              shared action, intent, and item icon catalog
invocations.toml        optional token-sequence input schemes
conditions.toml         timed state, stacking, immunities, periodic effects, and FX
actions.toml            sandbox-facing action definitions
recipes.toml            skill-gated crafting and preparation recipes
abilities_spells.toml   abilities and spells
races_classes.toml      races, classes, unlocks, starting loadouts
```

At compile time, shared code embeds these parts and exposes them as one effective
official ruleset to Creator, clients, tools, and tests.

The current playable progression ends at level 10. Warrior, Cleric, and Ranger
gain their core kit at level 1 and one new action at levels 2, 4, 6, 8, and 10.
Class `unlocks.level_N` tables are the sole owner of ability/spell acquisition;
starting loadouts own items only. The same table shape extends to level 30
without a schema change.

`attributes.roles` maps the official `HP`, `MAX_HP`, `LEVEL`, `EXP`, `DMG`, and
`ARMOR` names onto generic health, maximum-health, level, experience,
weapon-damage, and armor semantics. Class
`progression.level.resource_gains` entries name arbitrary current/maximum
resource pairs, and action costs are arbitrary declared attribute maps, so
custom rulesets do not need HP-, MP-, or combat-stat-specific engine fields.

Equipment slots, category handedness, occupied-slot conflicts, and class
permissions are rules-owned and enforced through one resolved runtime policy.
Missing class permission arrays are unrestricted, while explicit empty arrays
permit none; classless sandbox actors do not acquire implicit official class
restrictions. The optional `equipment.avatar_anchors` table maps those
rules-owned slot ids onto the avatar renderer's main/off-hand anchors without
hardcoded slot aliases.

The default visual avatars are stored at:

```text
assets/humanoid.eldiron_avatar
assets/orc.eldiron_avatar
assets/skeleton.eldiron_avatar
```

The Skeleton file is a distinct copy of the humanoid avatar with its own id
and name. It is intentionally ready for replacing its frames by importing a
Skeleton atlas while keeping the stable ruleset asset id `skeleton`.

Explicit character `tile_id`, `avatar`, or `avatar_id` values override the
ruleset visual default. Explicit empty visual attributes can be used to disable
the inherited default.

Bundled UI/item icon masks are generated from `icons.toml` into:

```text
assets/icons
```

Regenerate them with:

```bash
cargo run -p eldiron-icon-builder -- crates/ruleset/rulesets/eldiron/v1/icons.toml crates/ruleset/rulesets/eldiron/v1/assets/icons
```

The generated attribution file in `assets/icons/ATTRIBUTION.md` tracks the
Game-icons.net sources and licenses.

Definitions do not each need their own icon or particle block. The official
ruleset uses `ui.action_icon_fallbacks` and `ui.item_icon_fallbacks` for shared
semantic glyphs, plus `fx.action_fallbacks` and `fx.condition_fallbacks` for
shared procedural presets. Explicit per-definition presentation wins, and a
custom ruleset may change or omit every fallback table.

The spell economy uses a deliberately small material vocabulary. Moonleaf,
Sun Shards, Grave Dust, and Ember Resin come from placeable resource nodes.
Recipes refine them into Moonwater, Consecrated Oil, Warding Salt, and Ember
Beads. Spells consume those prepared reagents, while higher recipes can invest
them in reusable Ritual Censers and Sunward Charms. Item and action presentation
reuses semantic masks, so this content expansion does not require a matching
batch of one-off icon artwork.

Successful recipes also exercise the optional `[crafting.skill_gain]` loop.
The official settings grant deterministic progress below a recipe's mastery
range, which lets repeated reagent preparation lead into the Censer and Charm
recipes without requiring a combat class or externally scripted skill awards.

Every official spell also binds to the optional `words_of_power` invocation
scheme. The lexicon and word meanings live in `invocations.toml`; exact
sequences remain action-owned beside costs, targeting, and reagents.

Bundled reusable tile assets live in:

```text
assets/tiles
```

These are serialized Eldiron tiles used by ruleset-backed interactive items,
such as the torch's unlit tile and four-frame lit animation.

The torch also demonstrates rules-owned durability. Its `[durability]` table
drains `condition` while `active`, measured in game minutes, and removes the
item when the condition reaches `0%`.
