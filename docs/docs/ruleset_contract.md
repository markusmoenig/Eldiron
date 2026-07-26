---
title: "Ruleset Contract"
sidebar_position: 6.51
---

This document defines the design contract for Eldiron rulesets. It describes
the target architecture that future ruleset work must preserve. It is not a
claim that every capability described here is implemented today.

The accompanying [Ruleset Capability Audit](./ruleset_capability_audit)
records the current implementation status.

## Purpose

Eldiron must support games that:

- use the bundled Eldiron Official Ruleset unchanged
- override a few official values inside one project
- derive a new ruleset from an existing ruleset
- use a completely standalone ruleset
- omit fantasy RPG concepts that the game does not need
- add unusual mechanics without requiring an Eldiron engine fork

The official ruleset is a default content package. It must not have privileged
gameplay capabilities that are unavailable to another ruleset.

## Core Principles

### The Engine Is Generic

The engine provides reusable capabilities such as attributes, resources,
actions, effects, targeting, equipment, progression, conditions, inventory,
crafting, factions, presentation, and script hooks.

A ruleset chooses and configures those capabilities. The engine must not assume
that every game has classes, races, mana, levels, crafting, or fantasy damage
types.

### Rules Stay Authorable

Rules remain editable, split TOML with stable string identifiers, comments, and
project-local assets. Ruleset authors do not edit Rust types or a compiled
runtime representation.

The official ruleset must be authored through the same public format and
capabilities available to custom rulesets.

### Runtime Meaning Is Explicit

Every stable ruleset field must have an implemented and documented meaning.
Fields that are only roadmap ideas must not appear as fully supported rules.

New content should normally require rules data, assets, and possibly scripts.
Adding a normal item, spell, condition, recipe, race, class, or creature should
not require a new hardcoded runtime branch.

### One Fact Has One Owner

Targeting, cost, cooldown, and effects must not be independently duplicated
across an action and the spell or ability that exposes it. The ownership model
below defines where each kind of information belongs.

### Documentation Is Part Of The Contract

Schema reference and factual rulebook tables must be generated or verified from
the same resolved rules used by the runtime. A ruleset change is incomplete when
its validation, tests, or applicable documentation are out of date.

## Authoring Modes

Eldiron supports three conceptual authoring modes.

### Official With Project Overrides

This is the normal path for a project that mostly wants the default rules:

```toml
[ruleset]
id = "eldiron.official"
version = "1.0.0"
schema_version = "1"
source = "official"
update_policy = "compatible"
```

**Game / Rules** stores only project-specific differences. Table overrides merge
onto the selected base ruleset.

### Derived Ruleset

A derived ruleset starts from another ruleset and changes or removes selected
definitions. The target package syntax is:

```toml
[ruleset]
id = "example.hard_mode"
name = "Example Hard Mode"
version = "1.0.0"
schema_version = "2"
extends = "eldiron.official@1.0.0"
```

Derived rulesets require explicit inheritance, removal, conflict, and version
rules. They must not rely on undocumented merge behavior.

### Standalone Ruleset

A standalone ruleset supplies its own definitions and enables only the modules
it needs:

```toml
[ruleset]
id = "example.classless_survival"
name = "Classless Survival"
version = "1.0.0"
schema_version = "2"

[modules]
attributes = true
actions = true
items = true
crafting = true
classes = false
races = false
levels = false
```

Schema 1 already supports project-owned standalone TOML with
`source = "project"`. A distributable standalone package with its own asset
resolution is a target capability, not a completed one.

## Ruleset Package

The target portable package layout is:

```text
my-ruleset/
  ruleset.toml
  attributes.toml
  actions.toml
  invocations.toml
  items.toml
  progression.toml
  locales/
    en.toml
  assets/
    icons/
    avatars/
    tiles/
  scripts/
  README.md
```

The manifest declares:

- stable ruleset id and display name
- ruleset content version
- schema version
- minimum engine version
- optional base ruleset
- enabled modules
- source parts, locales, assets, and scripts

Ruleset packages must be loadable by Creator, runtime clients, terminal tools,
tests, and documentation tools through one shared loader.

## Optional Modules

The schema must allow these domains to be absent when a game does not use them:

- races and biological traits
- classes and class progression
- levels and experience
- abilities and spells
- professions and skills
- equipment slots
- crafting and recipes
- factions and reputation
- loot and economy
- conditions

An absent optional module is valid. Tools should hide its UI instead of
inventing official defaults.

Fundamental runtime concepts such as identity, health, level, or action energy
must use semantic roles or settings rather than fixed official identifiers such
as `HP`, `LEVEL`, or `MP`.

## Extensibility Levels

### Declarative Data

Ruleset authors can define ordinary content without scripts:

- attributes and resources
- damage and resistance kinds
- races, traits, classes, professions, and factions
- actions, abilities, spells, and conditions
- items, slots, recipes, resources, loot, and economy
- messages, icons, visuals, FX, and audio references

### Composable Effects

Executable behavior resolves to a shared action and effect vocabulary. The
initial target effect kinds are:

- damage
- heal
- apply or remove condition
- modify resource or attribute
- consume or give item
- gather
- craft
- move or teleport
- spawn or despawn
- revive
- dispel
- invoke a script hook

Effects define their requirements, target, timing, stacking, and failure
behavior through data.

### Script Hooks

Scripts provide an escape hatch for mechanics that do not fit the shared effect
vocabulary. Hooks may participate in:

- requirement checks
- target validation
- cost calculation
- action execution
- effect modification
- completion and failure events

The engine remains authoritative for networking, persistence, inventory
transactions, cooldowns, and event delivery.

## Canonical Ownership

| Domain | Owns | Does not own |
| --- | --- | --- |
| Attribute | identity, semantic role, default, bounds, presentation | class-specific starting value |
| Race | biological identity, traits, languages, visual defaults | political allegiance or combat progression |
| Trait | reusable immunities, vulnerabilities, needs, modifiers | a particular creature instance |
| Class | combat role, progression, permissions, level unlocks, starting item loadout | duplicate ability/spell lists or spell execution details |
| Profession | services and economic identity | combat class progression |
| Faction | allegiance, relations, reputation policy | biological race |
| Creature template | NPC composition, behavior, level/scaling, loot references | global race or class definition |
| Action | target, requirements, costs, cooldown, timing, effects, presentation, invocation bindings | whether a class learns it |
| Invocation scheme | available tokens and sequence matching policy | action mechanics or a mandatory UI |
| Ability or spell | identity, learning category, school/tags, action reference | a duplicate action implementation |
| Item | physical, economic, equipment, durability, and presentation data | actor-specific state outside the item |
| Recipe | input/output transformation, station, skill, difficulty, and its opportunity to advance that skill | a separately duplicated craft implementation |
| Resource | world-node behavior, availability, yield | a separately duplicated gather implementation |
| Condition | duration, stacking, modifiers, periodic effects, immunity tags | hardcoded special handling for one class |

The authoring format may provide convenient inline forms. Resolution must still
produce one canonical definition for each fact.

## Resolution Pipeline

The public TOML is resolved in a deterministic pipeline:

```text
package sources
  + selected base ruleset
  + project overrides
  -> RawRuleset
  -> inheritance and removal
  -> default application
  -> reference resolution
  -> schema and semantic validation
  -> ResolvedRuleset
```

`RawRuleset` preserves authoring constructs such as optional values,
inheritance, aliases, and references.

`ResolvedRuleset` is complete, strongly typed, validated, and ready for runtime
use. Runtime, Creator, inspectors, tests, and documentation consume this same
resolved representation.

### Current Resolver Boundary

The first typed resolver boundary is implemented:

- `RulesetSelection` represents the project setting.
- `RawRuleset` parses and merges the existing schema-1 TOML.
- `ResolvedRuleset` carries typed metadata, the effective table, and the
  validation report.
- every current action resolves to `ResolvedAction`, including typed kind,
  target, range, requirements, resource and item costs, effects, cooldown,
  recipe reference, invocation bindings, and presentation
- conditions resolve to `ResolvedCondition`, including duration, stacking
  policy, maximum stacks, tags, trait immunities, deterministic additive,
  multiplicative, and bounded attribute modifiers, plus typed periodic damage,
  healing, and numeric modifications
- derived stats resolve to typed, bounded formulas with cycle-validated
  dependencies
- optional `identity.defaults.race` and `identity.defaults.class` resolve as
  typed references; absence does not invent an official identity
- equipment resolves to a typed policy containing weapon and armor slots,
  category handedness and occupied slots, and optional per-class category
  permissions
- optional token-sequence invocation schemes resolve into the same cached
  action catalogue; invalid schemes, tokens, lengths, and duplicate phrases
  fail validation
- the existing string-based resolver temporarily delegates to this path while
  runtime consumers migrate to typed definitions
- unsupported selected schema versions and selection/source schema mismatches
  fail before runtime resolution
- malformed or unsupported `min_engine_version` requirements fail before
  runtime resolution
- official, project-owned standalone, project override, and the development
  Hideout2D project are tested through this boundary
- each running region resolves and caches its action, condition, and
  derived-stat catalogues once; replacing region rules invalidates and rebuilds
  that cache, and invalid definitions are reported during region startup

The ordinary attack intent is the first fully covered typed execution slice.
The action that declares `intent = "attack"` owns ordinary and follow-attack
behavior, while every action whose resolved kind is `attack` uses the same
public executor. Intent bindings are unique and case-insensitive, so a custom
ruleset can name that action freely and does not need Eldiron's `basic_attack`
identifier. Player
selection, intent targeting/range/cooldown derivation, follow-attack damage,
weapon damage source selection, ammunition, item consumption, and action
cooldowns consume the resolved action model.

Damage, healing, and condition spells use action-owned targets, range, resource
and item costs, cooldown, results, FX, and invocation bindings. Spell
definitions own identity, school/kind, learning, and referenced roll data.
`classes.<id>.unlocks.level_N` is the sole owner of when a class learns an
ability or spell; starting loadouts own items only.

Gathering actions now use the resolved action for their skill requirement,
target, range, item costs, give-item effect, and cooldown. Resource definitions
still own node identity and depletion/respawn behavior. The action effect is the
canonical output; an existing resource `produces` value remains a temporary
migration fallback until schema 2 removes that duplicate field. Craft actions
use typed kind, recipe reference, item costs, and cooldown but still delegate
recipe execution to the existing recipe domain branch. Optional
`crafting.skill_gain` policy applies deterministic use-based advancement only
after that recipe succeeds, observes the declared skill maximum, and can stop
teaching beyond a configurable mastery margin. Other non-attack action kinds
continue through their existing branches.

A project-owned sandbox regression ruleset deliberately contains skills,
items, resources, and actions but no classes or progression table. It proves
that a skill-gated world action can execute without the official class/level
model. This is a permanent architecture constraint: expanding the official
ruleset must not make classes or levels prerequisites for action execution.

The runtime action target is now a tagged value rather than an ambiguous
numeric id. It distinguishes entities, items (including their optional owner),
and world positions. Selecting an action and then clicking an entity or item,
using the current 2D terrain click, or supplying text-command coordinates
provides the corresponding target. Resolved targets currently include entity
dispositions, ground/inventory/any items, resource nodes, and world positions.

### Optional Action Invocations

An invocation is an input binding for an ordinary action, not a second spell or
effect system. Rulesets may omit invocation schemes entirely and present
actions with icons, menus, hotkeys, text commands, or custom UI.

```toml
[invocation_schemes.words_of_power]
name = "Words of Power"
kind = "token_sequence"
tokens = ["LO", "VI", "FUL", "YA", "IR", "SAR"]
separator = " "
max_tokens = 4
case_sensitive = false

[actions.minor_heal]
kind = "spell"
target = "friendly_or_self"
invocations = [
  { scheme = "words_of_power", sequence = ["LO", "VI"] },
]
```

Clients submit the selected phrase as
`intent.invoke:words_of_power:LO VI`. The authoritative runtime resolves it to
`action:minor_heal` before targeting and execution. Requirements, costs,
cooldowns, targets, and effects therefore remain owned by the action. Tokens
are text by default and may be rendered as words, runes, icons, gestures, or
other game-specific controls without requiring icon assets.

The initial public custom-effect escape hatch is an action result script event:

```toml
[actions.inspect_item]
name = "Inspect Item"
kind = "interaction"
target = "any_item"
range = 3
result = { script = "inspect_target" }
```

The event is queued on the acting entity after requirements, range, item costs,
and cooldown checks succeed. Its numeric payload carries the target entity/item
id or position; item payload `y` is the owner entity id or zero for a world
item, and the string payload is the action id. Typed `take` effects use the same
item target and preserve the world item when inventory insertion fails.

Script-event actions may also declare a non-consumed source tool:

```toml
[actions.unlock_chest]
name = "Unlock Chest"
kind = "interaction"
target = "any_item"
range = 2
source = { item = "lockpick", condition_cost = 5, destroy_on_empty = true }
result = { script = "unlock_target" }
```

The runtime validates that the source is an owned inventory or equipped item
matching the ruleset item id. Scripts may call
`use_action(action_id, target_id, source_item_id)` to select an exact instance;
without the third argument, equipped items are preferred before inventory
items. For entity/item targets, event payload `z` carries the selected source
item id. Source wear happens only after target and range checks succeed, and
`destroy_on_empty` removes the selected instance at zero condition.

This first source-item slice is deliberately limited to script-event actions
with entity or item targets. Applying source tools to built-in effects and
world-position targets requires a richer event payload and remains future
work.

Actions may be gated by arbitrary actor attributes without introducing a
class, level, or hardcoded engine concept:

```toml
[actions.haunt]
name = "Haunt"
target = "hostile_entity"
requires = { attributes = [
  { id = "mode", equals = "spirit" },
  { id = "karma", at_least = 10 },
  { id = "traits", contains = "undead" },
], target_attributes = [
  { id = "traits", not_contains = "warded" },
] }
result = { script = "haunt_target" }
```

`attributes` checks the actor and `target_attributes` checks the selected
entity target. All predicates are combined with logical AND. Each entry names
one attribute and exactly one predicate: `equals`, `not_equals`, `at_least`,
`at_most`, `contains`, or `not_contains`. Equality supports strings, numbers,
and booleans. Numeric bounds accept numeric attributes. Membership predicates
accept string arrays and comma-separated strings. A missing attribute never
satisfies a predicate, including a negative predicate.

Target predicates require an entity-targeting action and are enforced for
normal actions, direct spell execution, and repeated Basic Attack damage. The
client can disable an action immediately for failed actor predicates. Because
the target may not be selected yet, target predicates are instead shown in the
action description and enforced authoritatively once an entity is supplied.

Actions can directly modify numeric resources and arbitrary attributes:

```toml
[actions.intimidate]
name = "Intimidate"
target = "any_entity"
range = 2
requires = { attributes = [
  { id = "stamina", at_least = 2 },
] }
result = { modify = [
  { recipient = "actor", resource = "stamina", add = -2, minimum = 0 },
  { attribute = "karma", add = -5, minimum = 0 },
  { attribute = "mode", set = "frightened" },
] }
```

Each `result.modify` entry defines exactly one `attribute` or `resource` and
exactly one `add` or `set` operation. `recipient` is `target` by default and
may be set to `actor`. A target recipient requires an entity-targeting action;
for `target = "self"`, the action target resolves to the actor without an
explicit target id.

Numeric modifications may use `minimum`, `maximum`, or
`maximum_attribute`. The last form reads the clamp from the recipient, which
supports rules such as stamina capped by `max_stamina`. Numeric additions
preserve an existing integer or floating-point representation; a missing
numeric value begins at zero. Adding to an existing nonnumeric value or
referencing a missing maximum attribute rejects the action without applying
any modification, item cost, source wear, or cooldown.

All modifications are prepared before any are committed. They therefore apply
as one state transition after requirements, target, and range checks. The
initial implementation allows state modifications alone or together with a
script completion event. Combining them with damage, healing, item transfer,
gathering, crafting, or recipes remains deliberately unsupported until the
general multi-effect failure policy is defined. The action UI describes each
resolved modification.

### Derived Stats

Derived stats are typed ruleset definitions evaluated as effective attributes:

```toml
[derived_stats.POWER]
formula = "base + floor(max(0, INT - 10) / 4)"
minimum = 0
```

`base` names the saved value of the stat, `level` names the configured level
attribute, and other identifiers name ordinary or derived attributes.
Dependencies must be acyclic. Formulas support arithmetic, comparisons,
boolean conjunction/disjunction, parentheses, and the `min`, `max`, `clamp`,
`abs`, `floor`, `ceil`, and `round` functions.

The calculated value is not persisted over its base. Server gameplay and
client action availability consume the same formula contract. Condition
modifiers apply to dependencies and to the final derived stat. This permits
both level-based rules and classless sandbox rules without assigning special
meaning to the official stat names.

### Optional Progression

`progression` and `classes` are optional modules. When progression is present,
`progression.level.max_level` is the authoritative cap. A ruleset may provide
explicit `progression.xp_table.level_N` thresholds or a validated
`progression.level.xp_for_level` formula. Both use the same runtime grant path;
the formula form is therefore able to extend a level-10 ruleset to level 30
without changing representation. XP rows above the cap are invalid, and
formula-driven leveling stops at the cap.

Semantic attribute names belong to the ruleset, not Game Settings:

```toml
[attributes.roles]
health = "VITAL"
max_health = "VITAL_CAP"
level = "RANK"
experience = "RENOWN"
weapon_damage = "HARM"
armor = "WARD"
```

Roles are optional and their values must reference declared attributes.
Runtime damage, healing, respawn, XP grants, formulas, graphical placeholders,
terminal placeholders, equipment summaries, and stat widgets consume the same
resolved mapping. Omitting level and experience roles does not create implicit
`LEVEL` or `EXP` state in a classless sandbox.

Action resource costs are an arbitrary map of declared attribute id to amount,
for example `cost = { FOCUS = 4 }`. Spell execution prepares and applies the
entire typed cost map; the engine does not single out mana or an `MP` field.

Per-level resource growth is also name-independent:

```toml
[classes.Mystic.progression.level]
resource_gains = [
  { attribute = "VITAL", maximum_attribute = "VITAL_CAP", per_level = 2 },
  { attribute = "FOCUS", maximum_attribute = "FOCUS_CAP", per_level = 3 },
]
primary_attribute_gain = 1
```

Each resource pair is validated against declared attributes. Adding levels
11–30 therefore adds unlock rows or changes the cap/table; it does not require
a new representation or new HP/MP-specific fields.

### Equipment Policy

Equipment is a ruleset capability, not an official-game branch. A ruleset may
declare its slots and categories under `[equipment]`:

```toml
[equipment]
weapon_slots = ["main_hand", "off_hand"]
armor_slots = ["head", "torso", "legs", "hands", "feet", "shield"]

[equipment.avatar_anchors]
main_hand = "main_hand"
off_hand = "off_hand"
shield = "off_hand"

[equipment.weapon_categories.bow]
handed = "two_handed"

[equipment.armor_categories.shield]
occupies_slots = ["off_hand"]

[classes.Ranger]
allowed_weapons = ["bow", "sword", "axe"]
allowed_armor = ["cloth", "leather"]
```

Weapon categories may use `handed = "one_handed"` or
`handed = "two_handed"`. A two-handed weapon occupies every declared weapon
slot, regardless of the item's storage slot. Any weapon or armor category may
also declare `occupies_slots`; this lets a shield live in the `shield` slot
while preventing simultaneous use of an item that needs `off_hand`. The
item's own slot is always occupied.

Slot arrays preserve authored order and reject duplicates. The first occupied
weapon slot supplies the ordinary attack source. Server combat, host scripts,
graphical UI totals, and terminal statistics consume this resolved order;
`[game]` does not own a parallel slot list and the engine does not infer
official slot names.

`equipment.avatar_anchors` is optional presentation data. Its keys are declared
equipment slots and its values are the avatar renderer capabilities
`main_hand` or `off_hand`. This lets an arbitrary slot such as `grip` use the
main-hand frame anchor and lets a separately stored shield use the off-hand
anchor. Text-only or non-equipped-avatar rulesets may omit the map.

Class permissions are optional. If `allowed_weapons` or `allowed_armor` is
absent, that class does not restrict the corresponding category. An explicit
empty array permits none. Clothing uses the armor-category permission table.
Actors without a class, and actors whose class has no ruleset definition, are
not given an implicit official class restriction. They still obey declared
slot, category, and handedness conflicts, which keeps classless sandbox games
physically coherent without requiring a class module.

The validator rejects unknown slots/categories, invalid handedness, and
conflicting class starting equipment. Runtime checks use the same resolved,
cached policy for startup loadouts, UI drag/drop, host commands, and scripted
equip operations. A rejected change is transactional: the item remains in its
original inventory or equipment location.

### Conditions

Conditions are reusable entity state rather than ability-specific branches:

```toml
[conditions.guarded]
name = "Guarded"
duration = 2.0
stacking = "refresh"
max_stacks = 1
tags = ["stance", "beneficial"]
immune_traits = []
modifiers = [{ attribute = "ARMOR", add = 2 }]

[conditions.weakened]
duration = 4.0
stacking = "stack"
max_stacks = 3
modifiers = [
  { attribute = "POWER", add = -0.5, multiply = 0.9, minimum = 1 },
]

[conditions.guarded.fx.apply]
preset = "hit_burst"

[conditions.guarded.fx.active]
preset = "holy_glow"

[conditions.poisoned.periodic]
interval = 1.0
initial_delay = 1.0
effects = [
  { damage = 2, damage_kind = "poison" },
  { resource = "STAMINA", add = -1, minimum = 0 },
]

[conditions.poisoned.fx.tick]
preset = "poison_burst"

[actions.guard]
kind = "stance"
target = "self"
cooldown = 3.0
result = { apply_condition = "guarded" }
```

`stacking` accepts `replace`, `refresh`, `stack`, or `ignore`. A duration of
zero means persistent until removed. Actions use `apply_condition` and
`remove_condition`; either may use a string id or
`{ id = "...", recipient = "actor|target" }`. Application checks the
recipient's `traits` against `immune_traits`.

The server exposes active condition ids through the `conditions` entity
attribute and mirrors each condition as `condition_<id>_remaining` and
`condition_<id>_stacks`. Additive modifiers are read as effective attributes
by the migrated combat formula and structured damage-reduction paths without
overwriting base character attributes.

Each static modifier targets one attribute and defines at least one of `add`,
`multiply`, `minimum`, or `maximum`. Multipliers must be non-negative, and a
modifier's minimum cannot exceed its maximum. Active modifiers combine without
depending on definition order:

```text
effective = (base + Σ(add × stacks)) × Π(multiply ^ stacks)
effective = clamp(effective, strongest minimum, strongest maximum)
```

This keeps base attributes serializable and makes stacking deterministic.
Combat formulas and numeric actor/target action requirements consume the
effective value, and the action UI mirrors the same calculation. If bounds
from separately authored active conditions conflict, the strongest maximum is
applied last.

A condition's optional `periodic` table requires a positive `interval`, accepts
an optional non-negative `initial_delay`, and contains one or more effects.
Each effect defines exactly one of `damage`, `healing`, `attribute`, or
`resource`; numeric modifications use `add` and may specify `minimum` and
`maximum`. Periodic values scale with the current stack count, damage uses the
normal typed damage-kind rules, and the applying actor remains the source for
attribution.

The optional condition `fx` table supports `apply`, `active`, `tick`, and
`remove` stages. Each references a normal `[fx.presets]` entry. `active` is a
persistent replicated particle emitter that follows the affected entity and is
torn down when the condition ends; the other phases are one-shot emitters.
Scripts can observe `condition_applied`, `condition_tick`, and
`condition_removed`; payload `x/y/z/string` contains source id, stacks,
remaining seconds, and condition id.

Condition persistence uses serialized entity attributes as its public boundary:

- `conditions`
- `condition_<id>_remaining`
- `condition_<id>_stacks`
- `condition_<id>_tick_remaining` for periodic conditions
- `condition_<id>_source`, containing the applying entity's stable
  `creator_id`

The runtime reconstructs private condition state from those attributes when a
region starts. Remaining time and periodic phase resume exactly, meaning
condition time pauses while a region is unloaded. Source identity is mapped to
the entity's new runtime id; if the source is unavailable, the affected entity
becomes the safe fallback source. Invalid, immune, unknown, or already-expired
saved conditions are discarded.

Procedural FX items are transient presentation state. Serialized
`is_ruleset_fx` items are removed during startup, then one persistent `active`
emitter is rebuilt for each restored condition. Restoration does not emit
`condition_applied` or replay `apply` FX. Explicit action/condition stages take
precedence over optional ruleset-authored semantic fallbacks in
`fx.action_fallbacks` and `fx.condition_fallbacks`; omitting both means no
particle is spawned.

Items, progression, races, classes, spell definitions, recipes, and most
presentation data remain generic TOML inside `ResolvedRuleset`. They migrate
incrementally after the action boundary is proven.

This boundary is an implementation bridge, not a backwards-compatibility
promise. Eldiron may change the project and ruleset representation when a
cleaner public model requires it.

## Development Project And Release Policy

`test_projects/Hideout2D.eldiron` is the canonical migration fixture for the
new ruleset architecture. Representation changes must update that project and
its tests in the same change.

`starters/projects/Hideout2D.eldiron` remains frozen at the format shipped by
the current release. Development tests for the new model must not require that
frozen starter to load in the development engine.

At release time, after the migrated test project passes the complete runtime,
Creator, validation, asset, and documentation gates, it is deliberately copied
into `starters/projects/Hideout2D.eldiron`. Until that promotion, the two files
are expected to diverge.

## Overrides And Inheritance

Ruleset override behavior must be documented and testable for:

- nested tables
- scalar replacement
- arrays
- map entries
- explicit removal
- inheritance cycles
- missing bases
- incompatible versions
- asset and locale replacement

Creator should be able to show the origin of every resolved value and produce a
minimal project override.

## Identifiers And References

- Machine identifiers are stable and independent from localized names.
- References are validated before play starts.
- Renaming an identifier requires a migration or alias.
- User-facing names and descriptions can be localized.
- Ruleset ids use a namespace suitable for distribution.
- Runtime state stores enough version information to detect incompatible data.

## Versioning

Three versions have different meanings:

| Version | Meaning |
| --- | --- |
| `ruleset.version` | Content and balance release |
| `schema_version` | Public representation and semantics |
| `min_engine_version` | Minimum runtime capability |

Adding ordinary content usually changes the ruleset version. Changing public
field structure or meaning changes the schema version. Depending on a new
runtime effect or capability can change the minimum engine version.

Schema migrations must be explicit, testable, and able to report changes that
cannot be performed safely.

## Assets And Icons

Ruleset assets use the same selection, inheritance, override, and version rules
as TOML definitions.

An item or action does not require unique commissioned artwork. Action icons
resolve through:

1. explicit action `ui.icon`
2. an icon whose id matches the required ability, required spell, or action
3. `ui.action_icon_fallbacks` by healing role, condition role, action kind, then
   `default`

Item icon textures resolve an explicit icon first, then
`ui.item_icon_fallbacks` by item kind and `default`. Tiles, avatar channels, and
visual templates remain richer item-presentation paths and can take precedence
over drawing the icon texture. Fallback maps are optional: a minimalist or
text-only game may omit them. Every authored fallback reference is validated,
while many definitions can deliberately share one semantic glyph.

## Documentation Contract

Documentation has two layers.

### Ruleset Format Documentation

This explains the generic package format, schema, modules, effects, scripting,
validation, migration, and Creator workflow.

### Individual Ruleset Documentation

This explains one ruleset's gameplay. Factual catalogues and tables are
generated from `ResolvedRuleset`, including:

- attributes and resources
- races, traits, classes, and progression
- actions, abilities, spells, and conditions
- items, equipment, recipes, loot, and economy
- XP and level unlocks
- icon and asset coverage

Handwritten design explanations and tutorials remain manual. TOML examples are
parsed in tests or extracted from real rules.

## Definition Of Stable

A public ruleset capability is stable only when applicable columns are complete:

- public schema
- resolver
- runtime
- Creator authoring or inspection
- validation
- automated tests
- documentation
- migration behavior

The official ruleset must not label a capability as complete merely because its
TOML parses.

## Initial Delivery Sequence

1. Maintain the current capability audit.
2. Decide canonical ownership for existing schema 1 fields.
3. Add schema-1 `RawRuleset` and `ResolvedRuleset` types.
   **Implemented as the initial resolver boundary.**
4. Resolve official and project-owned rules through that typed path.
   **Implemented, including schema/engine gates and Hideout2D regression
   coverage.**
5. Migrate Basic Attack end to end without changing gameplay.
   **Implemented, including region-level resolved-action caching.**
6. Generate factual documentation from the resolved model.
7. Add icon inheritance and fallbacks. **Implemented for action/item icons and
   action/condition procedural FX through optional ruleset-authored mappings.**
8. Introduce schema 2 unified actions and effects with a migrator.
9. Add traits, conditions, factions, and creature templates. **Traits and the
   first typed timed-condition slice are implemented; factions and creature
   templates remain.**
10. Use Skeleton/Undead as the first complete third-party-equivalent vertical
    slice.
