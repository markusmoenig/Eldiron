---
title: "Ruleset Capability Audit"
sidebar_position: 6.52
---

This audit records what the current ruleset data, runtime, Creator, validation,
tests, and documentation actually support. It is the implementation baseline
for the [Ruleset Contract](./ruleset_contract).

Audit date: **2026-07-26**

Catalogue, internal editor visibility, world resources, bone production, repair and fauna status updated:
**2026-11-18**. This is a scoped update; other sections retain their existing
audit baseline.

For v0.96, standard Creator projects use the bundled ruleset for gameplay.
They store a selection and author the world that makes its content available.
The override and standalone cases below describe resolver capabilities exercised
by engine tools and tests. See [Rules In Eldiron](./rules_in_eldiron#source-of-truth)
for the project loading behavior.

Bundled baseline: **`eldiron.official` 1.0.0, schema 1, draft**

The supplied leather family now covers five slots with world-sourced hides,
permanent advanced patterns, reusable manuals, identity-preserving repair and
Fox inscription. Bear and Fox apply to compatible registered equipment and
coexist on one item. Focused coverage exercises both host modes, request replay,
world reload, MZ appearances and public-client journal metadata parity.


Fauna now supplies seven placeable species: deer, pigs, cattle, sheep, goats,
horses and chickens. **Add Animal** creates a character template with an
**Animal** entity node. Authors can add wandering in **Behavior Nodes**. The
shared MZ assets supply their meshes, rigs and six motion clips. Actual animal deaths
produce finite species-shaped carcasses. A focused solo/dedicated scenario covers
hunting, skinning with a world-crafted knife, tanning, stable carcass identities
and exhausted stock after reload. Breeding and renewable live-animal produce
remain outside this baseline.

The bundled ruleset passes its current structural/reference validator with zero
errors and zero warnings. That result does not mean every declared field is
executed by the runtime.

## Delivery Goal

Deliver a coherent, documented, maintainable Ruleset v1 that supports both
sandbox and class/level-based games through level 10, contains no
official-game-specific runtime branches, keeps `test_projects/Hideout2D.eldiron`
as the canonical integration project, and can grow to level 30 without another
representation redesign.

The goal is reached when:

- the official level-1-to-10 progression and content form a playable,
  internally balanced vertical slice
- standalone resolver fixtures can omit races, classes, levels, spells, or
  other optional modules without inheriting official assumptions
- ordinary ruleset extensions require data and assets, not Rust branches
- icons and procedural FX have reusable fallbacks and explicit validation
- the typed runtime contract, Creator-facing descriptions, tests, and
  documentation describe the same behavior
- the frozen released starter remains unchanged until the canonical test
  project is promoted for a release

## Status Legend

| Status | Meaning |
| --- | --- |
| Complete | Implemented across the applicable surfaces and covered by meaningful tests |
| Functional | Main runtime path works; authoring, validation, or coverage still has gaps |
| Partial | Some behavior exists, but important declared behavior is missing |
| Catalogue only | Data can be stored, displayed, or inspected but is not enforced |
| Missing | Target capability does not exist |

## Loading, Packaging, And Overrides

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Bundled official selection | Config selects an embedded id/version; official parts and assets are compiled into `eldiron-ruleset`; rules and shared asset lookup now use typed `RulesetSelection` | Functional | Route selection through the future shared package loader |
| Overlay resolution | The engine resolver merges nested TOML tables over its base and replaces scalars and arrays. Creator ignores legacy project gameplay snapshots. | Functional in resolver | Document deletion and array semantics if packaging is expanded |
| Standalone resolution | Engine tools and tests can load a complete custom rules document that omits RPG domains. | Functional in resolver | Define packaging before exposing standalone rules in Creator |
| External ruleset package | Only the bundled official package is registered; official assets use compile-time lists | Missing | Add manifest discovery, package asset loading, import/export, and version checks |
| Derived distributable ruleset | No public `extends` package contract or explicit removal mechanism | Missing | Define schema-2 inheritance, removal, conflict, and migration rules |
| Schema and engine compatibility enforcement | Resolution rejects unsupported selected/declared schema versions, selection/source mismatches, invalid minimum versions, and engines older than the declared minimum | Functional | Add explicit schema migration and compatibility-range policies |
| Update policies | `update_policy` is present in project config but is not consumed by the resolver | Catalogue only | Define pinned/patch/compatible/latest behavior or remove the field until implemented |
| Typed resolved representation | `ResolvedRuleset` now exposes typed actions with kind, unique intent bindings, target, range, requirements, costs, effects, cooldown, recipe, invocation bindings, and presentation; typed optional identity defaults and semantic attribute roles; typed class resource gains; typed invocation schemes; typed conditions with duration, stacking, immunity traits, tags, modifiers, and periodic effects; typed derived stats with parsed dependencies and bounds; and a typed equipment policy with slots, categories, handedness, occupied slots, class permissions, and avatar anchors. Other domains remain generic TOML | Partial | Add typed items, complete progression/race/class definitions, and field-origin data incrementally |

## Identity, Characters, And Progression

| Capability | Runtime | Creator/tools | Validation/tests/docs | Status |
| --- | --- | --- | --- | --- |
| Global, race, and class defaults | Applied in documented order while preserving authored keys | Rules can be edited and inspected | Runtime default/loadout/progression tests and docs exist | Complete |
| Starting loadouts | Class equipment and inventory are applied unless startup attributes are explicit | Ruleset item templates sync into projects | Reference validation and runtime tests exist | Complete |
| Class level gains and unlocks | Generic `resource_gains` pairs and primary attributes grow per level; `unlocks.level_N` is the sole gameplay owner of abilities/spells; Warrior, Cleric, and Ranger have complete 1/2/4/6/8/10 paths | Class inspector derives its full ability/spell list from unlocks and action bars expose future locked entries | Validation covers resource attribute references, unlock ranges, acquisition actions, action-bar references, and duplicate ownership; tests cover custom resource names, level-10 capstones, and runtime authorization | Complete |
| XP thresholds, kill XP, and level-up | Explicit XP table and level-up runtime paths exist | Terminal and Creator inspectors expose XP | Monotonic/max-level validation and progression tests exist | Functional |
| Derived stats | Typed, cycle-validated formulas for `MAX_HP`, `MAX_MP`, `INIT`, `DMG`, `POWER`, and `SPEED` compose ordinary/derived attributes and condition modifiers | TOML remains directly authorable; action UI evaluates the same formulas | Server tests cover dependencies, condition composition, action predicates, and regeneration caps; schema and official-rule docs define syntax | Functional |
| Configurable semantic attributes | Optional typed `[attributes.roles]` mappings drive health, maximum health, level, experience, weapon damage, and armor across damage/healing/respawn, XP, formulas, equipment summaries, graphical placeholders, avatar damage feedback, and terminal widgets; class growth and action costs name arbitrary resources | Role mappings, resource gains, and typed cost maps are directly TOML-authorable; duplicate Game Settings ownership and the spell-specific MP path were removed | Validation checks role, class-growth, and action-cost references; tests use `VITAL/VITAL_CAP`, `RANK/RENOWN`, `HARM/WARD`, and `FOCUS`, and prove classless rules do not invent official attributes or race | Functional |
| Class equipment permissions | A cached typed policy enforces weapon and armor categories for classes while leaving classless actors unrestricted | Class summaries and text widgets expose them | Validator checks references and starting loadouts; runtime tests cover startup, drag/drop, commands, and classless actors | Complete |
| Optional races/classes/levels | Standalone runtime fixtures may omit identity defaults, classes and progression. The bundled ruleset supplies identity defaults. XP formulas respect the authored cap. | No module capability UI; some inspectors still assume official-shaped identity | Resolver/runtime/UI contract tests and docs exist | Partial |
| Creature templates | Characters can carry race/class/faction attributes | No ruleset creature catalogue | No reusable creature-template schema | Missing |

## Disposition And Factions

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Race relations | Human/Orc/Skeleton relation tables drive base disposition | Functional | Keep as a configurable or legacy fallback |
| Reputation thresholds | Per-race/general reputation modifies disposition | Functional | Generalize target and storage semantics |
| Faction attribute | Same named faction is friendly; different or missing factions fall back to neutral behavior | Partial | Add ruleset-defined faction relations and faction reputation |
| Separation of biology and allegiance | Documentation says race is not a faction, but official hostility still begins with race relations | Partial | Introduce faction definitions and make race relation an explicit fallback |

## Actions, Abilities, And Spells

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Typed attack actions | The unique action bound with `intent = "attack"`—regardless of id—and custom/named attack actions share player execution, targeting/range/cooldown, weapon or rules-path damage, ammunition, item costs, follow-attack damage, client state/description, and icon fallback; resolved actions are cached per region and refreshed when rules change | Functional | Remove remaining raw-table helpers from non-attack branches |
| Damage/healing spells | Known-spell checks and application work; target, range, arbitrary typed resource/item costs, effect source, cooldown, and FX consume the cached resolved action, while spell definitions own identity and roll data | Functional | Type spell identity/roll data and retire the raw spell adapter |
| Legacy resource-item gathering | Skill, target, range, item costs, output effect, and cooldown consume the resolved action; the resource branch owns node identity, depletion, respawn, inventory transfer, and messages | Complete for this path | Remove duplicated action/resource output ownership in schema 2; new map-bound resources use profile-owned outputs instead |
| Craft actions | Typed kind, recipe reference, item costs, and cooldown select the existing recipe executor; recipe requirements, inputs, outputs, quality, and messages work | Functional | Consolidate action/recipe ownership and validate acquisition paths |
| Classless sandbox actions | A standalone runtime fixture with no classes or progression executes skill-gated mining, item/world-position targeting, typed take, script events, lockpick-on-chest source/target actions, actor/target attribute predicates, and transactional actor/target state modifications | Functional in runtime | Add broader world-state effects |
| Runtime action target | Tagged entity, item-with-optional-owner, and world-position targets flow from selected-action entity/item clicks, 2D terrain clicks, and text commands | Functional | Add 3D position picking, area/shape targets, and explicit multi-target collections |
| Source item plus target | Script-event actions declare an owned item source, optionally select its exact instance through `use_action`, expose its id in the event payload, and apply success-only condition wear/destruction | Functional | Generalize sources to built-in effects and world-position/multi-target payloads |
| Generic action executor | Typed policy selects attack, spell, gather, craft, take, and script-event branches; domains still have specialized application code | Partial | Unify damage/healing/item application while retaining domain-specific world-state handlers |
| Generic effect vocabulary | Resolved actions type damage, healing, give-item, take, script-event, attribute/resource modification, and apply/remove-condition effects; conditions type stack-scaled periodic damage, healing, and numeric changes; authoritative state application executes through typed paths | Partial | Add movement, spawn, and a general multi-effect failure policy |
| Guard | The official action applies the typed `guarded` condition to self; it grants effective `ARMOR +2` for two seconds, expires authoritatively, keeps base ARMOR unchanged, appears on Warrior/Cleric action bars, and reuses the round-shield icon | Complete | Extend the same condition path to additional official content |
| Fire Spark | A generic damage action, roll, particles, and `FUL` words-of-power binding execute through the normal spell path; no official class unlocks it | Functional example | Add it to a game-specific acquisition path only when that game wants fire magic |
| Critical hits | Critical roll and multiplier are declared and documented | Catalogue only | Define roll timing, rounding, UI/event payload, validation, and tests |
| Global/default cast cooldown | Defaults are declared | Partial | Define precedence and prove runtime use with tests |
| Cast time | Spell and legacy projectile cast-time paths exist, but the official resolved action path does not have one clearly canonical owner | Partial | Move timing into the unified resolved action |
| Script extension hooks | Items and characters run scripts; actions can be invoked by script and may queue a validated named script event with a typed target payload after policy checks | Functional | Add requirement/target/cost hook phases with explicit success/failure semantics |
| Optional invocation schemes | Typed token-sequence schemes and per-action bindings resolve phrases server-side into the cached ordinary action catalogue; validation rejects unknown schemes/tokens, excess length, and phrase collisions; Stonefall uses `LO VI` without a script translation | Functional | Add schema-driven generic token widgets and Creator inspection |

## Combat And Equipment

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Dice rolls and attribute bonuses | Weapon, ability, spell, and unarmed rolls are parsed and executed | Complete | Centralize roll evaluation in the resolved rules layer |
| Damage kinds and reduction | Physical, arcane/spell, and fire tables apply outgoing modifiers and defense reduction | Functional | Validate complete damage-kind semantics and add scenario coverage |
| Weapon category range/cooldown | The ordered typed `equipment.weapon_slots` list selects the primary equipped weapon; category data drives attacks, host script source context, graphical UI, and terminal stats without `[game]` slot duplication or official slot-name guesses | Complete | Type the remaining weapon item/category data to reduce repeated table lookups |
| Ammunition | Matching stack quantity is required and consumed | Complete | Add content-level acquisition/availability checks |
| Armor aggregation | Equipped armor attributes contribute to reduction, graphical UI, and terminal stats through the same typed `equipment.armor_slots` policy used by equip validation | Functional | Type armor item/category attributes and add broader damage-kind scenarios |
| Item quality/condition damage | Both scale weapon damage and crafted items receive quality/condition; bone armor opts into broken-item activation and immediate targeted repair | Functional | Broaden maintenance content and document additional quality/condition policies |
| Handedness and slot conflicts | Typed category policy expands two-handed weapons across weapon slots and supports category-owned `occupies_slots` such as shields occupying `off_hand` | Enforced transactionally at every production equip path, with conflict and inventory-preservation tests | Complete |
| Damage formula ownership | Weapon and unarmed damage tables execute; an optional ruleset-mapped `weapon_damage` attribute and class progression damage remain additional fallback/data sources | Partial | Define canonical weapon/unarmed/spell formula precedence |

## Items, Economy, Crafting, And World Rules

| Capability | Runtime | Validation/tests | Status |
| --- | --- | --- | --- |
| Ruleset item templates | Definitions sync into normal project item templates with identity and presentation | Extraction/sync tests exist | Complete |
| Stacks and quantities | Materials, ammunition, reagents, inputs, and outputs decrement correctly; stacks merge only when quality and condition also match, while aggregate queries total matching stacks | Unit/runtime coverage exists | Functional |
| Containers and corpse loot | Bags/corpses use containers; death drops inventory/equipped items and despawns empty corpses | Death/loot tests exist | Functional |
| Loot tables | Corpse transfer exists, but reusable weighted treasure/drop tables do not | Roadmap only | Missing |
| NPC respawn | Delay, health restoration, loadout reset, and corpse cleanup are rules-owned | Runtime tests exist | Functional |
| Economy and currency pickup | Base currency, display conversion, starting wealth, and monetary items work | Unit/runtime coverage exists | Functional |
| Shop price policy | Item worth exists; complete buy/sell/vendor modifiers are not part of the official ruleset | Limited | Partial |
| Recipes, quality, and use-based skill gain | Requirements, skill, materials, output, quality calculation, deterministic success-based advancement, skill caps, and mastery margins work | End-to-end ritual-chain and skill-gain tests exist | Functional |
| Crafting stations and tools | Capability/range station records require a visible, usable deployed world item; owned tools pay declared condition only on successful crafting. Legacy station strings remain descriptive hints | Atomic crafting, inventory/range and full-chain tests | Functional for explicit requirements; legacy hint migration remains separate |
| Durability | Torch demonstrates time-based condition drain and destruction | Durability tests exist | Functional |
| Shared passive item bonuses | One evaluator combines saved/derived baselines, explicit equipment projections, passive enchantments and conditions; combat, action availability and text/HUD values share it. Source explanations identify active/inactive items and attachments | Supplied Strength cuirass, break/repair, custom attributes, arithmetic, UI and ownership/persistence tests | Functional passive slice; advanced policy and trigger stages remain separate |
| Equipment wear and repair | Opt-in category wear charges a declared slot after matching positive health damage; broken categories can disable equipment contributions. Targeted repair recipes preserve exact owned item identity and properties while staging materials/tool costs. Bone armor provides the default example | Wear/rollback tests, shared stats, full local/dedicated save/reload and encoded-host repair/ownership/retry coverage | Functional for immediate declared policies; universal contributions and broader maintenance content remain separate |
| Map-bound world resources | Typed profiles, exposed placement overrides, independent logical sources and cross-region shared pools; trees, rivers, finite wells and remains | Definition, state, Authoring, local runtime and dedicated-host tests | Functional |
| Resource extraction and lifecycle | Atomic stock/output/tool-wear/skill updates, vessel filling, running simulation clock, recovery, expiry and optional exploitation suspension; persisted source identity and public resource views | Save/reload, contention, retry and lifecycle tests | Functional |
| Complete world-to-bone-armor production | Supplied hand-gathered bootstrap sources, knife/bowl/stations, hide and bone processing, liquid vessel debits and all five armor recipes/actions; no prepared starting materials required | Complete local/dedicated save/reload flow, wire-level host production with retries, rollback tests | Functional vertical slice |
| Persistent recipe knowledge | Versioned acquisition journal with UUIDs and class/character/teacher/book provenance; one-time starting grants, NPC Teach node, owned manuals and separate active equipment grants. Advanced Boneworking unlocks batch shaping; beginner recipes remain open | Focused definition, journal, lesson rollback, UI and Creator storage tests; local/dedicated runtime replication, book transfer, save/reconnect and durable retry scenario | Functional recipe-learning slice; general talent grants and other learning services remain separate |
| World-sourced gameplay enchanting | Bear inscription knowledge, Inscriber/manual acquisition, bone/resin/water inlay production and exact owned-cuirass mutation. Thematic names retain stable definition IDs; attachment identity, quality, condition and opaque properties survive repair and transfer | Focused schema/rollback/UI/icon tests and one empty-character solo/dedicated production, owner-replication, transfer, save/reconnect and receipt replay scenario | Functional immediate passive-enchantment slice; disenchanting, charges and triggers remain separate |
| Crafting and knowledge journal | GPU HUD with searchable known recipes, craft filters, source/lifetime explanations, exact owned-target menus, planner-backed stock/result previews and persistent action-bar pins. Shared authoritative checks run on isolated inspection snapshots; requests revalidate at the host | Focused planner parity/read-only, large-list model, native input and GPU retention tests; one owner-replica solo/dedicated pin/craft, stale preview, save/reconnect and receipt replay scenario | Functional immediate-crafting journal; no global recipe activation UI or new networking DTO |
| Resource lifecycle visuals | Optional authored tile mappings and semantic phase/supply replication | Runtime replication coverage | Partial; mesh/collision replacement and reusable asset binding inheritance remain planned |
| Resource regeneration | Arbitrarily named `[resource_regen.<attribute>]` entries run in real-time seconds and name their maximum attribute; the official rules use MP | Runtime tests cover configured resources, caps, and active-state gating | Complete |

## Conditions, Traits, And Creatures

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Timed conditions | Typed definitions, apply/remove effects, permanent or timed duration, replace/refresh/stack/ignore policies, maximum stacks, active-state mirrors, expiry, deterministic stack-scaled additive/multiplicative/bounded modifiers, periodic damage/healing/numeric changes, stable source attribution, lifecycle events, apply/active/tick/remove particle phases, serialized timing/stack/source restoration, transient-FX rebuilding, validation, UI descriptions, and runtime tests are implemented | Functional | Add derived-stat expressions and an optional wall-clock/offline expiry policy |
| Immunities and vulnerabilities | Race traits materialize as entity string lists; action predicates consume them and condition application checks `immune_traits` without race-specific code | Partial | Add trait-driven damage vulnerabilities/resistances and periodic-effect policies |
| Creature type | Race/class attributes can approximate it | Missing | Add creature type/tags independently from race |
| Undead/Skeleton | Distinct editable MZ skeleton body, skeletal traits, fitted clothing and shared equipment sockets; generic Turn Undead predicate/condition remains ruleset data | Functional vertical slice | Expand gameplay composition, loot and vulnerabilities; validate more live scenes |

## Presentation, Assets, And Localization

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Official avatar/icon/tile assets | Bundled MZ bodies, clothing/item surfaces, 32×32 item icons and native world models; bitmap avatar/action/UI assets remain supported | Functional for official | Package manifest discovery and broader platform acceptance |
| Project asset override | Project-owned MZ recipe/palette bindings and geometry edits survive synchronization/save/reopen; bitmap overrides remain supported | Functional | Apply equivalent discovery and override policy to third-party packages |
| Item visual fallback | Explicit artwork and state bindings coexist with editable MZ item icons/models; semantic PNG and generated fallbacks remain available | Functional | Expand coverage diagnostics and third-party artwork tooling |
| Action icon fallback | Rules actions resolve an explicit icon, a matching required ability/spell/action icon, then validated semantic healing/condition/kind/default mappings | Functional | Add optional icon composition and Creator previews |
| Semantic procedural FX fallback | Explicit action/condition stages win; validated ruleset-authored role/stage mappings provide shared presets, state actions now spawn their action FX, and conditions rebuild persistent active particles | Functional | Add travel-path execution beyond the existing spell path and Creator previews |
| Icon artwork and attribution | Official item icons use editable MZ recipes and project-palette bindings; action/UI PNG libraries retain their attribution/import workflow | Functional for official | Convert remaining action/UI artwork where useful; retain image-source support |
| Icon registration | Official textures are listed manually through compile-time macros | Partial | Generate the asset registry from the package manifest |
| Project palette | One editable project palette seeded with Endesga 64; MZ index bindings, Lospec/local switching, perceptual/ramp matching and canonical bitmap fitting | Functional | Broader authored-art review across very small/large target palettes |
| Localized runtime messages | Official English defaults merge with project locale overrides | Functional | Package locale discovery and completeness reports |
| Localized definition names/descriptions | Most names/descriptions are hardcoded English in rules TOML | Partial | Add locale keys with display fallbacks |

## Creator And Tooling

| Capability | Current evidence | Status | Required next step |
| --- | --- | --- | --- |
| Internal rules authoring | Explicit `internal-rules-editor` builds expose **Game / Rules** and the Rules dock; standard builds omit their navigation and ignore legacy open commands. Shared catalogue/domain metadata, forms, validation and recovery are retained | Functional internal tooling | Complete supplied RPG content and acquisition paths; public generic editing is not a prerequisite |
| Internal guided armor family | Maintainer **Armor Family** tool previews and atomically creates five bone-pattern pieces, recipes, actions, optional missing ingredient definitions and selected class grants; template UUIDs survive Undo/Redo and save/reload. Bone armor is already supplied | Functional internal tooling | Add further coherent supplied content; creators should not configure each family |
| Resource binding | Authoring profile/pool selectors and **Bind Resource** for sectors, linedefs and Geometry Objects; validated TOML overrides and Undo/Redo | Functional | Add reusable asset inheritance and policy migration tooling |
| Ruleset selection | **Game / Settings** stores the bundled ruleset selection used by Creator and the host. Legacy project-source settings resolve to the bundled ruleset. | Functional | Package selection is future work |
| Inspectors | Console/terminal list, show, summarize, roll, class, item, recipe, XP, and validate commands exist | Functional | Make all inspectors consume `ResolvedRuleset` |
| Item synchronization | Creator creates/refreshes ruleset-backed item templates | Functional | Generalize asset/package origin and conflict handling |
| Override origin/diff | Effective TOML can be inspected, but field origin and minimal override are not visualized | Missing | Add inherited/effective/source views and “override this value” |
| Fork/import/export | No first-class ruleset package workflow | Missing | Add Create From, import, export, and package validation |
| Ruleset refresh | Opening or playing an authored project resolves the bundled gameplay rules. Maintainers can preview edits in the internal Rules dock. | Partial | Define package and asset hot reload |

## Validation And Testing

### Existing Strengths

The current validator checks several important relationships:

- dice syntax and positive bonus divisors
- XP table keys, monotonic values, and maximum-level coverage
- weapon/armor categories and slots
- damage-kind references
- item, icon, ability, spell, recipe, skill, class, and resource references
- crafting skill-gain booleans and non-negative numeric policy values
- race defaults and relations
- class unlock and loadout references
- selected visual references

Runtime regression tests cover:

- character defaults, loadouts, progression, and explicit overrides
- the development Hideout2D project loading, resolving the current ruleset
  model and typed attack/damage-spell/healing-spell actions, retaining the
  authored Skeleton, resolving official visuals, and synchronizing item
  templates without rewriting the project
- race hostility, faction behavior, and attack targeting
- weapon damage, cooldowns, range, ammunition, and armor
- spell costs, targets, cooldowns, healing, damage, and events
- a standalone classless, level-less sandbox fixture with skill-gated mining
- classless item/world-position actions, script payloads, typed take, range
  rejection, and inventory-full rollback
- explicit and automatic source-tool selection, wrong/missing tool rejection,
  source payload identity, durability wear, and destruction
- custom actor predicates for scalar equality, numeric bounds, and string-list
  membership, including matching client disabled state
- target predicates with entity-target validation, server enforcement, direct
  spell/Basic Attack coverage paths, and UI requirement descriptions
- transactional actor/target attribute and resource modifications, scalar set,
  numeric add, fixed/dynamic clamps, self targeting, and failed-requirement
  rollback
- optional invocation-scheme parsing, case/whitespace normalization, token and
  length validation, collision rejection, and server intent resolution
- death, corpse loot, and NPC respawn
- resource gathering and respawn
- crafting inputs, outputs, stack merging, quality, use-based skill gain, and
  the official raw-material → reagent → reusable-equipment ritual chain
- resource regeneration and durability

The focused `rusterix` ruleset test suite is a required gate for this audit.
Crafted arrows with different quality or condition intentionally remain
separate stacks; inventory totals aggregate those stacks rather than treating
the first matching stack as the total.

### Missing Validation Layers

The validator does not yet prove:

- that every declared field has a runtime consumer
- that an ability or spell has an executable action
- that every item has an acquisition path
- that every class has useful level coverage
- that icons/locales/assets resolve through all fallbacks
- that recipes are reachable and non-circular
- that custom modules can be omitted safely
- that balance stays within defined envelopes

## Current Duplication And Ambiguity

These ownership conflicts should be settled before schema 2:

1. Runtime still accepts legacy spell-side execution/FX fields even though the
   official ruleset now assigns spell identity/roll data to spells and
   targeting, costs, cooldown, result, invocation, and FX to actions.
2. Runtime still accepts legacy ability-side execution fields even though the
   official ruleset now assigns ability identity/roll data to abilities and
   execution to actions.
3. Resource and gather action both describe output and timing-related behavior.
4. Recipe and craft action both describe the craft operation.
5. Weapon category, item attributes, item damage table, class progression
   damage, and the mapped weapon-damage attribute overlap.
6. Race relations and the faction attribute both participate in disposition
   without a full faction relation model.
7. Rich item visuals and icon glyphs have separate precedence chains. Creator
   now exposes editable On and Off frame rows and ruleset-default loading, but
   it does not yet preview the full resolved presentation report.

## Priority Backlog

### Foundation

1. Add schema-1 `RawRuleset` and `ResolvedRuleset` types. **Implemented as the
   initial migration boundary; domain typing remains incremental.**
2. Enforce schema and minimum engine compatibility. **Implemented for exact
   schema support and minimum engine versions; compatibility-range policy
   remains.**
3. Add a shared loader interface for official and future external ruleset
   packages.
4. Record source origin for every resolved definition and field.

### Canonical Execution

5. Define typed requirements, costs, targets, rolls, and effects. **Implemented
   for current action targets, ranges, requirements, resource/item costs, and
   the initial damage/healing/item/take effects.**
6. Migrate Basic Attack end to end without behavior changes. **Implemented for
   server execution, intent derivation, client state/description, and icon
   fallback, with resolved actions cached per region and refreshed when rules
   change.**
7. Consolidate spell/ability/action ownership. **Implemented for official
   schema 1: abilities/spells own identity and roll data; actions own target,
   range, requirements, costs, cooldown, result, invocation, and FX. Legacy raw
   adapters remain until schema 2.**
8. Migrate gather and craft through resolved actions. **Execution selection
   and action policy are migrated; schema-2 generation and removal of
   action/resource/recipe duplication remain.**

### Authoring And Distribution

9. Add explicit inheritance/removal semantics and a schema migrator.
10. Add package manifests, assets, locales, import/export, and Creator package
    selection.
11. Add icon inheritance/composition and presentation completeness checks.
    **Implemented for explicit, matching-definition, semantic role/kind, and
    default icon/FX fallbacks with reference validation; composition and a
    Creator coverage report remain.**
12. Generate factual rulebook/reference tables from resolved rules.

### First New Vertical Slice

13. Add typed factions, traits, conditions, and creature templates.
    **Traits and the first typed condition runtime are implemented.**
14. Add Skeleton with undead/skeletal traits, faction behavior, combat
    interactions, loot, avatar, icon fallbacks, tests, and generated docs.

## Milestone Acceptance Gates

### Typed Schema-1 Resolver

- Official schema-1 TOML resolves without behavior changes.
- The resolver's official-plus-overlay fixture resolves through the new model.
- Standalone resolver fixtures resolve without an official base.
- `test_projects/Hideout2D.eldiron` is migrated and remains green.
- Unknown and unsupported schema versions fail clearly.

### Typed Basic Attack

- Target, disposition, range, requirement, cooldown, ammunition, roll, quality,
  condition, damage reduction, event, message, audio, and FX data have one
  resolved path.
- Existing official attack behavior and tests remain unchanged.
- A standalone runtime fixture defines its own Basic Attack in the typed schema.

### Classless Sandbox Actions

- Standalone runtime fixtures can omit classes and progression.
- Skill-gated world actions resolve and execute without a class or level.
- Entity, item, and world-position targets retain their target kind through
  execution.
- A source-tool action can distinguish and wear the exact owned item instance
  used against its target.
- The action owns its targeting, range, costs, effects, script event, and
  cooldown.
- Domain definitions such as resource nodes describe world state rather than
  making the official progression model mandatory.

### Documentation Synchronization

- Factual class, item, action, spell, recipe, and progression tables are
  generated from `ResolvedRuleset`.
- Embedded TOML examples are parsed in tests.
- Documentation generation/checking runs in CI.

### Skeleton Vertical Slice

- Skeleton is composed through public race/type/trait/faction/template
  capabilities.
- No Skeleton-specific Rust branch is required.
- A third-party ruleset can use the same capabilities.
- Runtime, Creator, validator, tests, icons, localization, and generated
  documentation are all complete.

Current progress: the public race definition, editable bundled avatar copy,
race-default trait materialization, race relations, trait-consuming target
predicates, and trait-aware condition immunity are implemented. Creature
templates, faction composition, deeper combat interactions, loot, localization,
and final atlas art remain.

## Maintenance and contribution verification (2026-11-18)

The 2026-11-17 baseline reported 593 passing tests, six failures and one ignored test. Rechecking reproduced all six. The following corrections restore the intended checks:

- Gate's compilation test checks its current Player/Guard catalogue instead of a stale count of five.
- Hideout2D's advice test starts a new conversation instead of replacing an open stock choice session.
- Hideout2D's Torch no longer retains the legacy script already replaced by its existing behavior graph. Other archive entries and graph identities are preserved.
- The door look check expects the current private WorldText delivery.
- The startup fixture supplies a message channel and a real startup behavior graph.
- The character-default check expects the current Human hair index of 4.

The full headless run with shared contributions passes **609 tests, with zero failures and one ignored test**. Final focused verification passes: the new action-UI requirement case, 12 crafting/contribution cases, the solo/dedicated transfer/reconnect/save scenario, and the encoded hosted production/repair/ownership/retry/resync scenario. Creator compilation also passes. The documentation build succeeds. These results establish the corrected workspace baseline; they do not claim the six failures predated all ongoing workspace work.

The shared passive slice covers saved/derived bases, explicit category projections, enchantment instances, existing condition stacking arithmetic, source explanations, broken activation and exact repair. Ownership transfer, client state, reconnect and durable restoration are exercised in solo/dedicated sessions. Gameplay enchanting recipes, charged triggers, talent grants, item/action-local modifier stages and advanced stacking remain outside this implementation.

## Audit Sources

The audit is grounded primarily in:

- `crates/ruleset/src/lib.rs`
- `crates/ruleset/src/cli.rs`
- `crates/ruleset/rulesets/eldiron/v1/*.toml`
- `crates/rusterix/src/server/region.rs`
- `crates/rusterix/src/server/region_host.rs`
- `crates/rusterix/src/client/widget/mod.rs`
- `creator/src/docks/data.rs`
- `creator/src/docks/console.rs`
- `crates/shared/src/rulesets.rs`
- the current Rules, Official Rules, Rules In Eldiron, localization, audio, and
  player-input documentation

This document should be updated when a capability changes status. A feature is
not complete merely because one table or code path exists.

## MZ Documentation Coverage (2026-11-07)

The public guides now describe implemented authoring/runtime behavior rather
than treating MZ as only a future concept. Source checks covered the procedural
host/widget, map-composition targets, appearance workspace, Equipment drops,
settings controls, ruleset recipe library, runtime appearances and screen packages.

| Area | Documentation entry points |
| --- | --- |
| Discovering MZ | Features, Architecture, Getting Started, Creator Introduction, repository READMEs |
| World materials | [Procedural Materials](building_maps/procedural_materials), Working With Tiles, Tile Picker, Rect tool, 2D/3D maps and geometry |
| Rows and preview | Execution order, row inputs/connections, PBR outputs, palette controls, source previews and saved tiling modes in Procedural Materials |
| Map composition | Shape grid, road parts, wobble/soft edges, stacked layers, bitmap inputs and reusable copies |
| Characters/items | [Appearance](characters_items/appearance), Project Tree, Entity Nodes, Attributes and [Rigging](characters_items/rigging) |
| Settings | Material density and tile backgrounds separated from character/map-item 2D/3D pixelization in [Game Configuration](configuration/game#tile-and-appearance-settings) |
| Palette/ruleset | Palette Tool, Official Rules, Rules in Eldiron and Ruleset Contract; removes separate fixed-palette and PNG-only item claims |
| Screen artwork | Screens, Widgets and Project Files; color/opacity, animation, continuous coordinates and dependency-carrying screen export |
| Developer orientation | Embedded-core README, official asset READMEs, examples and separate character-generator status |

Bitmap tiles and avatars remain documented as supported paths. Historical blog
posts, release notes and dated design/benchmark sections retain their historical
meaning; imported upstream core docs are snapshots, not a claim of complete
standalone MaterialZ UI parity. The new guides link the current workflows.

The docs do not claim a live per-pixel material VM, automatic quadruped locomotion,
physical bow projectiles, universal action-icon migration or an unconditional
30 FPS guarantee. Native release idle benchmarks now clear the 30 FPS budget;
their settings and limits are recorded in the unified appearance integration
history. Browser/mobile interaction and broader live combat/equipment validation
remain separate work.
