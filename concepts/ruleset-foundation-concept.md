# Integrated ruleset foundation

Date: 2026-11-13

Status: Concept for review and extension. No implementation is authorized by this document. Example identifiers, fields, and TOML below illustrate proposed semantics; they are not a supported schema.

## Purpose and direction

Eldiron should provide one integrated default ruleset capable of supporting both a small Ultima IV/V-style adventure and an Ultima Online-style sandbox. A small game might expose Ranger and Warrior classes, a village, and a handful of recipes. A large game might offer an unrestricted Human class, thousands of items, and dozens or hundreds of professions. Both use the same world mechanics and content relationships.

The distinction is how much of the ruleset a game uses and exposes. Authors should not have to replace the adventure ruleset with a separate simulation ruleset to grow their game. Custom and standalone rulesets remain possible, but the bundled ruleset should already provide a coherent world to build upon.

The foundation must accommodate new materials, trades, learning paths, item properties, enchantments, and world interactions through shared capabilities. It must also make the resulting rules explainable, inspectable, and testable. Content that parses successfully but has no defined gameplay meaning is not a completed feature.

This concept establishes ownership and behavior through four examples: a harvestable tree, river water collected into a vessel, an openly trained sandbox character, and an enchanted crafted item. It proposes no changes to the ongoing CPU-to-GPU rewrite.

The companion [Ruleset Data Model and Bone Armor Production](ruleset-data-model-concept.md) proposes definition relationships, illustrative TOML, and a complete sourced production chain for reviewing this foundation.

The [ruleset authoring decision](ruleset-editor-concept.md) separates ordinary world building from internal ruleset maintenance. The earlier Choose/Customize/Extend editor proposal is superseded by the product decision below.

## RPG scope and ruleset authoring

Product direction agreed on 2026-11-14: Eldiron focuses on RPGs, adventures, and RPG sandbox worlds, from a small authored game to a complex persistent simulation. Supporting every gameplay genre is outside that scope. The engine's reusable capabilities serve this RPG core; a request for a new capability should be justified by an RPG use case rather than hypothetical universal game-engine coverage.

The setting remains flexible. A science-fiction RPG can use different materials, equipment, resources, and professions through the same foundation. An RTS would introduce separate central requirements such as formation control, large-scale unit orders, strategic AI, and faction production management. Those requirements are not promises made by ruleset customization.

Classless development, optional levels, unusual character attributes, and custom RPG mechanics remain important. Narrowing the supported gameplay scope does not turn the official fantasy names, classes, or damage types into engine constants. Custom and standalone RPG rulesets should use the same public capabilities as the bundled content.

### World building as the normal workflow

Product decision updated on 2026-11-17: Eldiron supplies a complete, integrated RPG ruleset. Ordinary creators build worlds from its character and item catalogue, place resources and encounters, and choose player starting options. They do not configure or enable each item family in a ruleset editor. Small adventures and open sandboxes use the same underlying definitions and mechanics.

Bone armor already belongs to the supplied content. A game introduces it through placed items or through obtainable materials and crafting knowledge. Omitting a placed cuirass alone does not exclude it if its recipe and ingredients remain accessible. This is an acquisition question in the authored world, not a global armor-family activation checkbox.

Choosing which classes players may select is distinct from changing class definitions or deleting classes used by NPCs. Likewise, resource placements choose existing profiles and may use their exposed local settings; this is world authoring, not general ruleset design. Content selection must never silently award stock, create materials, or restock vendors.

### Internal ruleset maintenance

The generic ruleset editor is retained as a development tool and hidden from normal Creator navigation, including its menus. There is no ordinary Advanced menu leading to it. An explicit maintainer build feature may expose it; standard debug and release builds do not.

The shared catalogue, domain metadata, definition validation, recovery and family-generation tools remain useful for maintaining and extending the supplied content. Rules remain project-owned, with the same serialization, stable identities and local/hosted runtime resolution. Hiding an editor must not remove, reset or migrate a project's rules document or existing items.

Custom and standalone RPG rules remain technically possible, but a comprehensive public ruleset editing interface is not the current product priority. Generic field forms do not establish new runtime capabilities merely by storing data.

### Design priority

Complete coherent default content and its world acquisition paths before expanding ruleset editing UI. The first implemented production slice connects skeleton remains to raw bones, prepared bone plates and supporting leather straps, all five armor pieces, and equipment use. It includes hand-gathered bootstrap supplies, tools, vessels and deployed stations. Every supporting ingredient, tool and station needs an obtainable world origin. Source diagnostics and explanations should help creators place and understand this content without requiring them to author its underlying recipes and actions.

## Existing foundations and design gaps

The [Ruleset Contract](../docs/docs/ruleset_contract.md), [Rules In Eldiron](../docs/docs/rules_in_eldiron.md), and [Capability Audit](../docs/docs/ruleset_capability_audit.md) describe the existing direction and implementation baseline. Current capabilities provide useful starting points:

- Projects own complete editable rules documents. The bundled official graph compiles into validated rules data used by shared consumers.
- Actions already resolve targeting, requirements, costs, and several effects. Derived attributes and conditions provide effective values without overwriting saved base attributes.
- Resource nodes already demonstrate supply, gathering, depletion, and respawn. Crafting demonstrates inputs, outputs, quality, and use-based skill advancement.
- Equipment has shared slot and permission policy, and existing combat paths aggregate some equipped item attributes.
- Characters and items already support ruleset-backed defaults and authored configuration, including starting possessions.

These capabilities do not yet establish the complete model below. In particular, existing item damage bonuses and equipment aggregation must not be mistaken for a universal item-property system. A common contract for arbitrary equipped bonuses, enchantments, granted capabilities, triggered effects, attribution, stacking, and removal is a foundation gap to resolve.

External package loading, inherited deltas, and upgrade workflows are separate unfinished capabilities. This concept must work with project-owned rules documents and must not depend on those distribution features being complete.

## Principles for one integrated ruleset

### Shared meaning at every scale

A tree, a log, a bow, or a skill has the same underlying meaning in a small adventure and a sandbox. A small game can expose few recipes and fixed class paths while retaining the same gathering and crafting semantics as a larger world.

Useful defaults should make ordinary authoring quick. Selecting a standard tree resource definition should establish its normal interactions, supply, recovery, and visual states. Authors adjust exceptional trees rather than configuring every tree from scratch.

Optional depth should emerge through additional definitions and relationships. Seasons, cultivation, unusual woods, ownership, and specialized professions can extend the model without making all those features prerequisites for gathering a branch.

### Explicit capabilities and composition

Definitions use stable identifiers independent of their display names. Properties have declared types, units, bounds, and runtime meaning. Tags classify content; rules explicitly consume those classifications. A `flammable` tag alone does not implement burning.

Ordinary extensions should combine shared properties, requirements, actions, effects, modifiers, and events. Scripts remain an extension mechanism for unusual behavior under the same authority and state-change rules. Genuinely new capabilities may require engine work; flexibility does not mean arbitrary metadata automatically executes.

### Independent gameplay and presentation

One semantic resource can be represented by a 2D tile, an isometric sprite, a 3D object, or a text description. Meshes, pixels, and animation frames are not the owners of supply, recovery, damage, or learning.

Gameplay decides that a tree is exhausted. Presentation selects its corresponding appearance. Collision or navigation changes, when required by felling or growth, are explicit world changes rather than accidental consequences of swapping artwork.

## Ownership and state

| Owner | Responsibility | Examples |
| --- | --- | --- |
| Ruleset | Reusable definitions, valid references, mechanics, and defaults | Wood items, tree resource profiles, gathering actions, recipes, skills, enchantments |
| Map placement | Resource binding and allowed local configuration | This tree supplies more branches; this well has limited water |
| Character or item setup | Ruleset selections and initial instance configuration | Class, talents, starting skills, equipment, possessions, item quality |
| Runtime instance | Mutable state belonging to one persistent individual or pool | Remaining supply, recovery phase, vessel contents, learned skills, enchantment charges |
| Presentation binding | Appearances and descriptions for semantic states | Healthy tree, bare tree, stump, full flask, flaming sword |

Definitions, authored initial values, and live state must remain distinct. Reloading artwork or refreshing rules must not refill a harvested tree, reset a trained character, or remove an item's enchantment.

Every persistent instance needs stable identity independent of runtime numeric IDs. Many placements can share a tile or object asset without sharing depletion. A large tree represented by several tiles should still have one logical resource instance. A river area can explicitly share a resource pool rather than creating an accidental independent supply for every visible tile.

Copying a placement creates a new instance unless a shared pool is explicitly selected. Editing a reusable asset or resource default must have a clearly visible scope; it must not silently overwrite every instance's live state.

## Materials and production

### Materials are ruleset items

Logs, branches, water, ore, herbs, and prepared reagents must reference item definitions in the effective project ruleset. Map TOML can select and configure permitted outputs, but cannot invent an unregistered material by typing an arbitrary name.

Authors add a new material through the ruleset first, then reference it from world resources and recipes. An unknown output is a validation error, not an instruction to manufacture a loosely defined item. This also applies to scripted or magical production: its outputs require valid definitions.

An ingredient or reagent describes a use of an item. Water can be drunk, cooked with, used in alchemy, or consumed by a workshop. It does not need separate item identities for every profession that uses it.

### Initial material families

The following is a proposed vocabulary to refine into concrete definitions, rather than a final content catalogue.

| Family | Natural sources | Initial outputs | Example transformations |
| --- | --- | --- | --- |
| Wood | Trees and fallen timber | Branches, logs, bark, resin | Boards, shafts, bows, furniture, charcoal |
| Mineral | Rock, deposits, riverbanks | Stone, ores, clay, sand | Blocks, ingots, pottery, glass |
| Water | Rivers, lakes, wells, sea | Fresh water, salt water | Cooking, washing, brewing, purification |
| Plant | Bushes, fields, wild plants | Fruit, grain, herbs, plant fibers | Flour, food, medicine, cloth, reagents |
| Animal | Animals and their remains | Meat, hide, feathers, wool | Food, leather, arrows, textiles |
| Creature remains | Appropriate creatures, including skeletons | Bones and other declared creature materials | Bone armor, tools, prepared reagents |

Distinguish materials when the distinction creates meaningful interactions. A small game can use generic wood definitions. Species, grade, provenance, contamination, and other distinctions can be added where useful without duplicating every recipe unnecessarily.

Recipes may accept an exact item or an explicitly declared material category. Category matching needs a defined selection policy and must preserve any properties the recipe says matter. Mixing ingredients must not silently erase an enchantment, contamination, or valuable quality difference.

### Connected acquisition paths

Every material, ingredient, and reagent needed by crafting, spells, enchanting, repair, or any other consuming system must have an obtainable source in the world. This is a requirement for the integrated default ruleset. It applies to supporting ingredients such as bindings, fuel, and prepared magical reagents as well as the main material in a recipe.

Wood comes from trees or timber, water from a water source, and bones from appropriate creatures or their remains. Magical ingredients also need defined sources: particular plants, minerals, creatures, places, or transformations of other sourced materials. A magical name, script, or spell is not sufficient sourcing by itself. Any magical production must declare the world source or material inputs that make its output available.

Sources, processing, recipes, consumption, repair, and trade form a traceable acquisition chain. An item can have several paths, but each required ingredient's chain must terminate in an actual world source or an explicitly authored initial supply with a defined origin. Mutually dependent recipes with no obtainable starting material do not establish a source. Starting supplies can seed a chain; they cannot justify unlimited replenishment.

### Vendors participate in world supply

Vendors acquire goods through gathering, production, purchases, or deliveries from declared world sources. They cannot sell an otherwise unobtainable reagent or generate stock merely because a shop restock timer elapsed. Selling an ingredient redistributes world supply; the vendor is not its unexplained origin.

Initial merchant stocks represent previously sourced goods with a declared origin. Restocking transfers goods from an actual supply or produces them from accounted inputs. A simple game can use scheduled deliveries from a named resource pool or producer, with quantities accounted for. A deeper simulation can expose the gathering, crafting, and transport activities. Both obey the same sourcing contract.

Treasure, gifts, and carried loot can redistribute existing materials or represent declared prior supplies. Creature-derived resources are direct world sources when appropriate to that creature. A skeleton yielding bones is coherent; assigning unrelated wood or magical reagents to arbitrary enemies is not the default material economy.

### Economic participation is authored gameplay

The same sourcing, production, inventory, and trade capabilities must work in solo and online multiplayer games. How a game keeps its economy active is an author-defined gameplay choice. NPC gathering, crafting, consumption, trading, and deliveries can use those shared capabilities, with the same material accounting as player activity.

An author may make NPC participation respond to demand, shortages, player activity, or player count. Another game may use fixed services, scripted deliveries, or a small economy. Adaptive NPC labor, automatic population scaling, and a universal economic controller are not requirements of the ruleset. A basic game must not need to configure those systems to use crafting or shops.

The foundation provides inspectable actions, requirements, outcomes, inventory and resource state, and script or behavior hooks from which those policies can be authored. The gameplay policy decides when and why an NPC acts. Every resulting operation still obeys sourced supply and authoritative state changes in either play mode.

### Skeleton bones and complete armor families

Skeletons can supply registered bone materials through their declared death loot or a corpse-processing interaction. The definition chooses where extraction happens and how much is available. Loot and corpse harvesting must share the same supply accounting so that the same bones cannot be awarded twice.

An armorer, blacksmith, or other appropriate trade can process those bones into armor. The precise profession name can be refined; the recipes, skill requirements, tools, and station capabilities define the work. Additional materials such as straps, lining, or binding resin require their own complete world acquisition chains.

The default catalogue must provide complete wearable sets for the armor families it introduces. Bone armor therefore needs actual definitions and craftable recipes for the full declared set, including:

- Head armor, such as a bone helm.
- Chest armor, such as a bone cuirass.
- Leg armor, such as bone greaves.
- Hand armor, such as bone gauntlets.
- Foot armor, such as bone boots.

Separate arm pieces and shields should be included where the ruleset's slots and that armor family's design call for them. Each piece has explicit slot occupancy, material quantities, protection, durability, repair inputs, and appearance. Shared family defaults can keep those definitions consistent. Set completeness follows declared gameplay slots across 2D, isometric, 3D, and text presentation.

The resulting chain is concrete: encounter skeletons, obtain bones once, process them, obtain any supporting materials, craft the desired pieces, equip them, and maintain them using sourced repair materials. A shop can sell the resulting bone armor when its supply follows that chain. Producing a whole set is a content and acquisition requirement, not an automatic extra bonus for wearing it; any set bonus requires an explicit rule.

### Acquisition completeness review

For every enabled recipe, spell, enchantment, or repair operation, inspection must trace all required inputs through processing and trade to obtainable world sources. It must also identify dependencies such as the required tools, stations, and learning opportunities so the chain can actually be started.

The full default catalogue needs source coverage for all of its required materials and complete coverage of each declared armor set. An authored game needs reachable sources or accounted deliveries for the systems it exposes. Unused catalogue entries do not force a small map to contain every material. An exposed crafting chain with a vendor-only ingredient of unexplained origin fails acquisition completeness review.

Required reports include missing sources, inaccessible source locations, production cycles without a starting supply, vendor restocking without supply accounting, double extraction from creature remains, missing armor pieces or recipes, and repair ingredients with no world origin.

## World resources and map authoring

### Resource definitions and local TOML

Use the existing selected-tile or selected-3D-object TOML workflow to bind a world feature to a ruleset resource definition and override allowed placement values. The same concept applies to isometric placements and resource areas.

The resource definition owns supported interactions, default supplies, valid output kinds, tool requirements, recovery policy, and state transitions. Local TOML can configure fields such as supply, yield, recovery duration, and initial state where that definition permits them. Definitions should expose a clear set of authorable fields rather than accept undocumented keys.

Binding precedence is: ruleset resource defaults, an optional reusable asset or prefab binding, then explicit placement overrides. Mutable runtime state is initialized from those resolved values and subsequently evolves independently. A later default change requires a deliberate reconciliation policy; it is not a reset command.

A tile asset may offer a reusable resource binding, but a tile image is not itself a global supply counter. The editor must distinguish editing asset defaults from editing a selected map placement. Painted maps need a defined way to identify resource instances, including resources spanning multiple cells.

Characters and inventory items continue to come from ruleset definitions through their existing setup workflows. Initial choices such as starting items remain instance configuration. This proposal does not move their mechanics into map-resource TOML.

### Supply and yield

A resource needs separate concepts for available stock and the amount obtained per interaction. A branch supply might hold twelve branch items and yield two per successful collection. A river can provide water without a meaningful depletion limit. A mineral deposit can be finite.

Stock must have an explicit unit. Some supplies count output items or liquid volume; other interactions consume one harvest opportunity and produce a recipe-like bundle. A single ambiguous `amount` must not mean all three.

The model should support several named supplies and interactions on one feature. Taking branches and felling a tree are different operations. Each declares which supply and lifecycle state it consumes or changes. A feature cannot retain a usable branch supply after being felled simply because the two operations were implemented independently.

### Recovery and lifecycle

Recovery replenishes stock. Lifecycle transitions change what the feature is and which interactions are available. A bare bush can refill its berries; a felled tree requires an explicit regrowth policy rather than a generic resource reset.

Recovery rules declare their clock and units. Proposed ecological recovery uses elapsed world time, allowing a tree to recover across several game days. World time advances while an area is unloaded according to the host's world-clock policy. Wall-clock time while the game or server is stopped is a separate optional policy and must not be silently assumed.

Store sufficient timing state to restore the same result after save/load. Where recovery is simple, calculate bounded catch-up from elapsed time rather than requiring every unloaded feature to tick. More complex growth or seasonal rules must define equivalent catch-up behavior.

Normal harvesting may produce a temporary bare state. Sustained exploitation may produce a damaged state lasting several world days. Those are explicit policies with recovery conditions, not visual guesses based on how frequently a sprite was drawn.

### Interaction execution

Every interaction identifies the actor, source feature, selected tool or vessel, target, relevant supplies, and the resolved rules revision. Requirements include reach, ownership where applicable, tools, skill, feature state, and output capacity.

For immediate harvesting, prepare and validate the complete change before committing supply reduction, item transfer, tool wear, and skill advancement. Failure leaves them unchanged. An action that intentionally charges for a failed attempt must declare that policy. Partial harvesting is allowed only when its quantity and costs are explicitly defined.

Long actions need separate start, completion, interruption, and reservation semantics. Two actors cannot both receive the final unit. Remaining supply and equipment must be revalidated or reserved authoritatively. A repeated network request must not duplicate the same committed harvest.

## Example one a harvestable tree

The default ruleset defines a mature tree resource, `wood_branch` and `wood_log` items, branch collection and felling actions, suitable tools, and recovery and regrowth policies.

A map author selects a tree placement and binds it to the resource. Proposed local configuration could look like this:

```toml
[world_resource]
definition = "mature_tree"

[world_resource.supplies.branches]
item = "wood_branch"
capacity = 12
initial_stock = 12
yield_per_use = 2

[world_resource.supplies.branches.recovery]
amount = 4
interval = { value = 1, unit = "world_day" }
```

In this example, branch stock, capacity, recovery amount, and yield are all measured in `wood_branch` item units. The profile supplies the collection action and other defaults. Its output reference must resolve to a permitted ruleset item. The proposed fields are illustrative and require a future schema decision.

The tree begins with twelve available branches. Six successful collections of two branches exhaust that supply. An attempted collection with no inventory capacity leaves the tree and tool unchanged. The tree becomes bare at zero stock. With no further harvesting, four branches recover per world day up to twelve. Appearance follows authored stock thresholds.

Felling is a separate interaction: it requires an appropriate tool, grants a declared log yield, changes the tree to a stump, and disables branch collection. The default log yield and any local override also reference registered ruleset items. Stump removal, planting, and regrowth are explicit extensions of that lifecycle. A simple game can expose only branch gathering; a sandbox can expose the whole chain.

This example should also support a local rule for longer damage after sustained overharvesting. The exact damage threshold and regrowth duration remain balance decisions rather than engine constants.

Review scenarios:

- Two neighboring trees sharing artwork deplete independently; a multi-tile tree depletes once.
- Changing views between 2D, isometric, and 3D does not create another supply.
- Full inventory, invalid tools, and competing final-unit requests preserve consistent state.
- Save/load and unloading preserve stock and recovery phase without replaying harvest rewards.
- Felling disables other living-tree supplies and updates the semantic state used by visuals and collision.

## Example two river water and a vessel

The ruleset defines `fresh_water`, `salt_water`, vessel capacity, and filling, pouring, drinking, and recipe-consumption capabilities. Water quantity uses a declared common volume unit. The representation of liquid batches is a design choice; its semantics must be consistent across gathering, inventory, and recipes.

A river area binds to a freshwater resource profile. Its ordinary supply is effectively unlimited for gameplay, while a well can use a finite recoverable pool. Many selectable river tiles may refer to one area binding. Water quality and source properties belong to the selected resource; purity must not be inferred solely from blue artwork.

An actor fills a suitable empty flask at the river. The requested amount is limited by available capacity and, for finite sources, remaining stock. The runtime validates contact or reach and vessel compatibility, then transfers the volume as one state change. Filling changes the vessel's contents; it does not create a loose, full-sized water item on every click.

The vessel and its contents retain separate identities and properties. An iron flask does not turn its water into iron. Pouring transfers contents, drinking consumes them, and recipes reserve the required volume. Proposed initial behavior rejects incompatible mixing unless a mixing rule is defined. Future contamination, temperature, and purification can extend the same content model.

A simple game can represent water with one standard measure and a full/empty flask. That is a constrained use of the same volume model; a sandbox can expose partial quantities and multiple vessel sizes.

Review scenarios:

- A full or incompatible vessel cannot extract more water or alter the source.
- Filling and pouring conserve volume for finite sources and vessels.
- A recipe uses only its required volume and preserves the vessel.
- Fresh and salt water remain distinguishable through transfer and save/load.
- Repeating the same committed fill request cannot fill two vessels or create extra water.

## Characters classes talents and learning

### One character model

Structured adventuring classes and open sandbox development use the same attributes, skills, actions, equipment, and learning rules. A class is a policy and starting foundation; it does not require every capability to originate in a class-level unlock.

A game can expose Ranger, Warrior, and Cleric. Another can expose a general Human class with chosen talents and unrestricted future learning. Human as a race describes biological identity; Human as the displayed class describes an open development path. Their identifiers and references must remain distinct even if both display the same word. Games can also omit classes entirely, as required by the existing contract.

| Concept | Owns | Example |
| --- | --- | --- |
| Race and traits | Biological defaults and reusable characteristics | Human, undead, needs, resistances |
| Class | Starting package, permissions, and optional structured progression | Ranger or general Human |
| Aptitude and talent | Initial strengths and declared ongoing advantages | Strength distribution or intellectual aptitude |
| Skill | Learned proficiency and advancement policy | Archery, carpentry, alchemy |
| Knowledge | Known recipes, techniques, spells, and other capabilities | Bow-making recipe or a healing spell |
| Profession | Economic identity, services, teaching, and related activities | Bowyer, carpenter, herbalist |

Initial talents should normally create advantages rather than permanent exclusions. Hard restrictions remain possible when deliberately authored. Attribute growth, talent effects, skill advancement, and maximums each need explicit owners; a classless or open path must not silently inherit adventuring-level requirements.

### Multiple learning paths

Class unlocks own what that class grants. Learning policy defines other acquisition paths: practice, teachers, books, discoveries, quests, and experimentation. One recipe or action retains one mechanical definition regardless of how it becomes known.

The model must distinguish permanent learning from a capability temporarily granted by equipment or a condition. Track acquisition sources so removing an enchanted item cannot erase independently learned knowledge. Multiple sources must not create duplicate action buttons or duplicate rewards.

Practice advancement is tied to a declared meaningful activity and its committed outcome. Failed or cancelled attempts follow explicit policy rather than accidentally granting training. No global profession-count limit is assumed; individual skills and learning policies can still have configured bounds.

## Example three an openly trained Human

At character creation, the player chooses the general Human class and distributes a bounded starting attribute budget or selects a talent preset. A strength-oriented and an intellect-oriented character therefore begin differently without being locked into unrelated rule systems. The exact budget and talent effects remain content decisions.

The character receives modest starting possessions and selected beginner proficiencies through ordinary setup. They can gather branches, learn basic woodworking, craft shafts, and later study bow-making with a bowyer. Practice improves the relevant skill according to recipe difficulty and advancement policy. They can separately learn alchemy without changing their class.

In the same ruleset, a Ranger can start with archery knowledge and gain selected techniques through class progression. Both characters use the same bow, bow-making recipe, resource sources, and execution rules. A Ranger's training can grant an advantage without inventing a separate Ranger-only crafting implementation.

Review scenarios:

- The Human can train and craft with no adventuring level or XP prerequisite unless a specific rule declares one.
- Starting allocations obey their budget and produce inspectably different effective attributes.
- A class grant, teacher, and book can grant the same knowledge without duplicating it.
- Removing a temporary grant preserves independently learned knowledge.
- New professions reuse skills, services, and recipes without a new engine branch per profession.

## Item properties bonuses and enchantments

### A shared contribution model

An item definition describes a kind of item. An item instance retains its own quality, condition, material choices, persistent identity, and attached properties. An enchantment is a reusable definition attached to an instance, with state such as charges or expiry where needed.

A contribution records its source, affected subject, applicability, operation, stacking policy, lifetime, and removal rules. Subjects can include the item itself, its wielder, an action performed with it, or the recipient of an effect. A sword's damage bonus must not automatically improve every action performed by its owner.

Equipped bonuses, material properties, enchantments, talents, and conditions should share compatible evaluation semantics. They need not all use the same authoring form, but must not independently mutate the same base attributes through unrelated paths.

At minimum, distinguish changes to a value, grants of a capability, and triggered effects. Carrying, equipping, wielding, using, and hitting are different activation contexts. Whether a broken item retains a magical bonus is explicit policy.

### Evaluation order and ownership

Each calculated value needs a declared pipeline. The proposed starting point is:

1. Establish the saved or definition-derived base and applicable contributions.
2. Resolve dependencies through the same effective-value evaluator.
3. Evaluate a derived formula, if present, using effective dependencies.
4. Combine modifiers targeting that result in declared stages.
5. Apply bounds and documented rounding, producing a trace of contributions.

Within a numeric modifier stage, a useful default is `(base + sum(additions)) * product(multipliers)`, followed by bounds. Override operations require explicit priority and tie handling. Formula cycles, invalid units, and unsupported combinations fail validation. An equipment contribution enters each relevant value once; a separate combat equipment sum must not add it again.

This is a proposed contract to review against existing condition, armor, damage, quality, and derived-stat behavior. Exact quality scaling, resistance ordering, rounding, and conflicting-bound policy must be settled before adopting a universal evaluator. For each value, one owner defines the stages so contributors cannot choose arbitrary points in the calculation.

### Triggered behavior

Triggers bind to documented semantic events such as a successful committed hit. They receive structured context: actor, item instance, target, action, source contribution, and outcome. Source identity remains available for damage attribution, explanations, and persistence.

Rules define whether triggers run once per action or per affected target, how chained effects behave, and what prevents recursion. Random behavior uses a reproducible random source in tests. Saving and loading must not replay a past hit or reapply a permanent base-stat increment.

## Example four an enchanted crafted sword

A smith crafts an ordinary sword from registered materials. The resulting instance has quality and condition and retains any material properties that its recipe explicitly carries forward. Enchanting attaches a ruleset-defined enchantment to that same instance rather than requiring a new item definition for every combination.

An illustrative ember enchantment provides two independent contributions:

- While the sword is wielded, grant its wielder a declared fire-resistance bonus.
- On a successful hit made with this sword, apply a declared fire-damage effect to that hit's target.

The fire effect uses the shared fire damage and resistance rules. An optional burning effect would reference the ordinary burning condition. The exact damage, chance, charges, and duration belong to the enchantment definition. A charge, if required, is consumed according to the trigger's success policy.

The sword's physical damage, its quality scaling, and the enchantment's fire damage remain separately explainable. The enchantment declares whether quality affects its magic. A temporary blessing can add another contribution without changing the saved sword or character base.

Unequipping the sword removes its wielded bonus and trigger eligibility. Disenchanting removes only that enchantment and its state. Transferring, dropping, repairing, breaking, and saving the sword preserve or end contributions according to explicit rules. Items with different enchantments, contents, quality, or other relevant state cannot silently merge into an indistinguishable stack.

Review scenarios:

- Equip, unequip, re-equip, and save/load never accumulate extra resistance.
- The fire trigger applies only to hits made with that sword and obeys its target-count policy.
- Removing a blessing or enchantment leaves unrelated contributions intact.
- Transferring the sword removes the former wielder's bonus and grants it to the new wielder only when eligible.
- Charges cannot be duplicated by interrupted actions, request retries, or save/load.
- Tooltips, effective attributes, damage results, and terminal inspection describe the same contributions.

## Shared execution and persistence contract

Actions should share a structured context identifying actor, source feature or item, target or target collection, requested quantity, and applicable rules. Requirements, costs, effects, and presentation use that context without ambiguous positional script payloads.

An immediate state transition follows prepare, validate, and commit stages. Domain handlers can specialize in liquid transfer, harvesting, crafting, or damage while preserving one outcome contract. Mixed effects need a defined failure boundary. External script notifications and presentation events follow the committed result; they cannot retroactively make a failed transaction appear successful.

Time-spanning effects cannot be one indefinitely open transaction. Their start costs, reservations, completion costs, interruption behavior, and any compensation must be explicit. Trigger chains need ordering and termination rules. Random outcomes are resolved authoritatively and cannot be rerolled by duplicate requests.

Persist live identities, quantities, clocks, learned knowledge and its sources, attached item properties, and other state necessary to resume. Derived values and transient visual effects should be reconstructed from authoritative state. Restoring an instance must not replay acquisition, training, harvest rewards, or on-hit events.

Definitions and runtime saves need version information. If a material, resource, enchantment, or skill changes incompatibly, validation and migration should report the affected instances. Silently discarding an unfamiliar enchantment or refilling an unrecognized resource is not a safe default.

## Explanation inspection and review

Every resolved definition should be inspectable with its effective values, references, defaults, and local overrides. Every calculated result should expose a structured explanation of inputs, applicable contributions, stages, and final outcome. Human-readable output is rendered from that information for Creator, terminal, and player-facing views as appropriate.

Examples of questions the tools should answer:

- Why can this tree currently supply only two branches, and when will more recover?
- Is this value inherited from the resource definition or overridden on this placement?
- Which sources in this authored map provide wood, and which recipes use it?
- Why can this character learn bow-making, and where did they acquire it?
- Which item grants this resistance, and why is another bonus inactive?
- How did this hit split into physical and fire damage?
- What changes if this material, skill, or enchantment definition is edited?

Explain both active and rejected contributions: an unequipped item's bonus may exist in its definition while being inapplicable to the character. Stable IDs should connect an explanation to the relevant definition or instance.

Factual catalogue tables and reference descriptions should be generated or verified from resolved rules. Handwritten material explains design intent and examples. Do not independently maintain different numeric rules in UI descriptions, documentation, and execution code.

## Validation and simulation strategy

| Level | Purpose | Representative checks |
| --- | --- | --- |
| Definition validation | Establish valid meaning before play | References, types, units, legal modifiers, clock policy, formula cycles, supported effects |
| Map and configuration validation | Check the authored game against its effective rules | Registered outputs, placement identities, resource bindings, starting items, valid local overrides, obtainable sources for exposed material chains |
| Scenario testing | Prove complete state changes and failure behavior | Four examples above, save/load, unload recovery, full inventory, interruption, competing actors |
| Simulation and balance review | Evaluate connected systems over repeated activity and time | Forest supply, consumption, skill advancement, production chains, sourced vendor stock, complete armor families, item power, economic loops |

Deterministic scenarios supply explicit time, inputs, and random seeds and inspect authoritative state without rendering. The same scenarios should apply to a small adventure configuration and a sandbox configuration of the integrated ruleset.

Structural errors should block play. Availability and balance reports require context: a recipe may be intentionally absent from a small map, and a production loop may be intentional. Reports should distinguish an invalid reference from unreachable content, an unusual economic loop, or a balance concern.

Scale review must include large catalogues and many placed resources. Search, categories, reusable definitions, cross-references, and impact inspection should keep authoring manageable. Resolution should avoid repeatedly scanning the entire catalogue for each interaction. Unloaded resources should not require individual continuous updates merely to recover simple stock.

## Proposed behavior and ownership specification

This specification gives the four examples concrete outcomes for review. Quantities, attribute budgets, yields, and damage numbers are scenario fixtures, not final balance values. Each scenario assumes its referenced definitions exist in the effective ruleset. Table rows are independent unless an explicit sequence is described. These are expected behaviors for future implementation and testing; no executable tests are introduced here.

### Ownership at the interaction boundary

| Fact | Canonical owner | Allowed instance configuration or state |
| --- | --- | --- |
| Material identity, quantity unit, stack compatibility | Ruleset item or material definition | Instance quantity and declared material properties |
| Supported resource interactions and output constraints | Ruleset resource definition and referenced actions | Placement selects the definition and permitted output references |
| Default stock, yield, recovery, and lifecycle | Ruleset resource definition | Placement overrides explicitly exposed fields; runtime stores remaining stock and timing |
| Which visual cells or objects represent one resource | Authored map binding | Persistent resource identity or explicit shared-pool reference |
| Harvest requirements, cost, and training opportunity | Shared action and advancement definitions | Actor and tool state determine whether requirements are met |
| Character starting package and learning policy | Ruleset class, talents, skills, and knowledge definitions | Character setup selects permitted starting choices; runtime records learned state |
| Bonus magnitude, subject, activation, and stacking | Item property or enchantment definition | Item instance retains attachment identity, charges, and expiry |
| Result of an interaction | Authoritative runtime | One committed outcome with source identities and an explanation |

An output override such as `item = "wood_branch"` must satisfy both checks: the item exists in the effective ruleset, and the selected resource permits it. A valid sword item is not automatically a valid output for a branch supply. Extending permitted outputs is a ruleset edit. Local TOML cannot redefine the referenced item, action, or enchantment inline.

Changing an authored starting value affects newly initialized instances. It does not overwrite an existing instance's stock, contents, skill, or charges. Editing live definitions requires validation and explicit reconciliation; incompatible state stays associated with its identity for migration or repair rather than being silently reset.

### Shared outcome rules for these scenarios

Immediate actions check requirements, calculate their complete changes, and commit once. Their output capacity is checked after accounting for inputs the same action will consume. Inventory limits and stack compatibility come from the resolved rules. A failed action reports a structured reason and makes no changes to stock, contents, inputs, tool condition, charges, or skill. Independent recovery or expiry due at that world time is settled before validation; rejecting an action does not undo elapsed time.

The tree and crafting fixtures require their full output quantity and do not partially succeed. Water filling and pouring deliberately allow a reduced positive transfer. A transfer of zero is rejected as a no-op and grants no advancement. Other actions may declare different policies, but must expose them in their explanations.

For a timed version of these actions, the fixture policy is no start cost and no reservation. Completion revalidates the source, target, tool, and capacity, and then commits atomically. Interruption commits nothing. Competing completions are serialized against the authoritative current state. A future reservation policy can extend this contract without changing the immediate fixtures.

Repeated delivery of the same request identity returns the recorded outcome and makes no additional changes. A genuinely new request is evaluated against current state. Restoration reconstructs state without replaying successful actions. These are gameplay requirements; the networking design determines the mechanism and retention of request outcomes.

### Tree behavior contract

The fixture is a living tree with capacity twelve branches, initial stock twelve, yield two, and recovery of four per world day. Collection requires a declared pruning tool, costs one tool-condition point, and grants one woodworking point while the fixture's advancement rule remains eligible. Partial branch collection is disabled. Felling yields six logs and costs one tool-condition point. Those numbers belong to the fixture's rules definitions or permitted placement overrides.

Tree lifecycle and stock appearance are distinct. A living tree with eight to twelve branches appears healthy, one to seven appears sparse, and zero appears bare. A stump has no available living-tree supplies regardless of the previous branch count. Picking an appearance never changes these values.

| Scenario | Before | Request | Expected after |
| --- | --- | --- | --- |
| One harvest | Stock 12; carried branches 0; tool condition 100; woodworking 10 | Collect branches | Stock 10; branches 2; condition 99; woodworking 11; healthy appearance |
| Exhaustion | Stock 2; branches 10; condition 95; woodworking 15 | Collect branches | Stock 0; branches 12; condition 94; woodworking 16; bare appearance |
| Insufficient stock | Stock 1; otherwise eligible | Collect the normal yield of 2 | Rejected; stock remains 1; no output, wear, or training |
| No output capacity | Stock 12; no room for 2 compatible branch items | Collect branches | Rejected; all state unchanged |
| Invalid tool | Stock 12; required tool absent or broken | Collect branches | Rejected; all state unchanged |
| Competing harvests | Stock 2; actors A and B can each receive 2 | Both complete collection | One commits: stock 0 and winner gains 2; the other receives exhaustion failure with no costs or training |
| Felling | Living tree; stock 10; room for 6 logs; axe condition 100 | Fell tree | Stump; branch stock 0 and recovery disabled; logs +6; axe condition 99 |
| Harvesting a stump | Stump; any prior branch stock | Collect branches or fell again | Rejected; no extra branches or logs |

Recovery starts its phase at the first successful depletion from full stock. Further harvesting does not restart that phase. Each elapsed full interval adds four up to capacity. Reaching capacity ends the phase and discards unused recovery; a full tree cannot bank future branches. A new depletion from full begins a new phase. Pending recovery is settled before an interaction at that same world time.

For a tree exhausted at world day 0, with no later harvest, day 1 has stock 4 and sparse appearance, day 2 has stock 8 and healthy appearance, and day 3 has stock 12. Loading it directly at day 3 must give the same result as observing every recovery interval. Saving at day 0.5 and loading at the same world time preserves stock 0 and the half-day remaining until recovery. If the host clock advances to day 1 while the region is unloaded, loading yields stock 4. Pausing the world clock advances no recovery.

To exercise sustained exploitation, a separate fixture adds a declared policy: the third transition to empty within a rolling three-world-day window damages the tree and suspends collection and recovery for three world days. A tree emptied on days 0, 1, and 2 becomes damaged on day 2 with stock 0. Damage ends on day 5; the first new recovery interval completes on day 6, producing stock 4. The depletion history and damage deadline survive save/load. The basic tree need not enable this policy.

The felling fixture has no automatic regrowth. Adding a regrowth policy must explicitly define stump and growth transitions and when supplies become available again. It must never reuse branch recovery to turn a stump immediately into a mature tree.

Required explanation example: "Collected 2 Wood Branches. Tree supply 12 → 10; pruning tool condition 100 → 99; woodworking 10 → 11. Next recovery in 1 world day." Failures identify the failed requirement and confirm that no transfer committed. A terminal and a graphical client report the same outcome.

### Water behavior contract

The fixture uses integer volume units without choosing their physical size yet. A flask holds four units, a cup holds two, and both remain ordinary persistent item instances. Their liquid contents reference the registered `fresh_water` or `salt_water` definition and retain relevant content properties. The river is unlimited; a well in a separate scenario has three units remaining. Source capacity and vessel capacity are independent facts.

Filling and pouring transfer the minimum of requested quantity, available contents or source stock, and destination free capacity. A positive transfer is one atomic change. Compatible contents have the same liquid definition and the same properties required by their merge rule. Incompatible mixing is rejected in full. An empty vessel clears its content association so that it can subsequently receive a different allowed liquid.

| Scenario | Before | Request | Expected after |
| --- | --- | --- | --- |
| Fill at river | Empty flask, capacity 4 | Fill 4 fresh-water units | Flask contains 4 fresh-water units; river remains unlimited; flask identity unchanged |
| Top up | Flask contains 3 fresh-water units | Fill 4 more | Transfer 1; flask contains 4; no overflow |
| Full vessel | Flask contains 4 | Fill again | Rejected as full; no state change or training |
| Finite source | Well stock 3; empty flask capacity 4 | Fill 4 | Transfer 3; well stock 0; flask contents 3 |
| Pour between vessels | Flask contains 4; cup contains 1 compatible unit, capacity 2 | Pour 3 into cup | Transfer 1; flask contents 3; cup contents 2; total remains 5 |
| Incompatible contents | Flask contains 2 salt-water units | Fill from freshwater river | Rejected; flask remains 2 salt-water units |
| Use in recipe | Flask contains 4 fresh-water units; other inputs and output space valid | Craft a recipe consuming 1 unit | Flask contents 3; vessel preserved; other inputs and outputs change exactly as the recipe declares |
| Recipe cannot complete | Same inputs; insufficient output capacity after planned consumption | Craft | Rejected; water and all other inputs unchanged |

Save/load preserves vessel identity, contents, quantities, and properties. A repeated fill request cannot reapply its transfer after the flask has been poured out. A new request can fill the flask again under normal rules. Containers representing the same well all reference its one finite pool.

Required explanation example: "Transferred 1 Fresh Water unit. Requested 3; cup free capacity limited transfer to 1. Flask 4 → 3; cup 1 → 2." A recipe explanation identifies the actual vessel and volume consumed rather than reporting that an entire flask disappeared.

### Sandbox character behavior contract

The creation fixture uses five attributes: strength, dexterity, intellect, wisdom, and vitality. Each begins at 8; ten additional points may be allocated, with a creation maximum of 14 per attribute. This temporary budget allows a precise review of creation behavior without choosing final balance or lifetime caps.

A strength-oriented allocation of 14, 10, 8, 8, 10 and an intellect-oriented allocation of 8, 8, 14, 12, 8 both total 50 and are valid. An allocation spending eleven extra points or exceeding 14 is rejected before the character is created. Chosen talent effects are resolved contributions with their own definitions; they do not introduce undocumented changes to the budget.

The general Human class grants beginner shaft-making knowledge. The character begins at woodworking 10 with two branches, enough output capacity, and any tool required by that recipe. The fixture recipe consumes two branches, produces four shafts, and advances woodworking by one on eligible success. Bow-making knowledge is initially absent. No action in this fixture requires character level or XP.

| Scenario | Before | Request | Expected after |
| --- | --- | --- | --- |
| Beginner practice | Woodworking 10; branches 2; shafts 0; shaft-making known | Craft shafts | Woodworking 11; branches 0; shafts 4; no class change or XP requirement |
| Missing materials | Same knowledge and skill; branches 1 | Craft shafts | Rejected; woodworking remains 10 and branch remains 1 |
| Unknown recipe | Bow-making unknown; otherwise eligible | Craft a bow | Rejected for missing knowledge; no inputs consumed |
| Teacher instruction | Bow-making unknown; teacher eligible; no lesson cost in this fixture | Complete instruction | One permanent bow-making acquisition recorded with teacher provenance |
| Another permanent source | Bow-making already learned from teacher | Read a valid bow-making book | Knowledge remains available once; book provenance may be added; no duplicate action or first-learning reward |
| Temporary grant | Bow-making available only from an equipped item | Unequip item | Bow-making becomes unavailable; skill values are unchanged |
| Learning while temporarily enabled | Item grants bow-making; teacher then teaches it permanently | Unequip item after instruction | Bow-making remains available from permanent learning |
| Another profession | Woodworking already trained; alchemy prerequisites met | Learn beginner alchemy | Both skill paths remain available; Human class unchanged |

Permanent acquisitions are historical learning records. Removing their original teacher or book from the world does not revoke knowledge. Temporary grants are active sources and disappear with their applicability. Deliberate forgetting or retraining would be a separate authored operation. Repeating the same instruction event does not create further records or rewards.

A Ranger fixture may receive bow-making from its class instead. Its recipe execution and material requirements are identical to the Human's. The class source changes acquisition and any declared advantages, not the recipe implementation. Disabling level progression leaves the open learning path valid.

Required explanation example: "Bow Making is available: learned from the village bowyer. An equipped charm also grants it; removing the charm will preserve learned knowledge." Attribute inspection separately reports the chosen starting allocation and any currently applicable talent or equipment contributions.

### Item bonus and enchantment behavior contract

Each contribution has a stable identity built from its owning source instance, attachment or application instance, and contribution definition. Two swords carrying the same enchantment are distinct sources. Re-evaluating the same equipped sword does not create a second source. Definitions separately declare whether distinct sources stack.

A contribution explicitly selects its subject and activation context. An item-local modifier changes that item; a wielder modifier changes its eligible wielder; an action modifier applies only to matching actions using the source item. Merely carrying the sword activates neither a wielded bonus nor an on-hit trigger. A visible weapon model is not evidence that the runtime equipped or wielded it.

For this specification, additive and multiplicative contributions in one stage combine as `(baseline + sum(additions)) * product(multipliers)`. For example, baseline 10, additions 2 and 3, and multiplier 1.5 produce 22.5 in any definition order. Rounding happens only where the owning value contract requires it. A bonus to a formula dependency is evaluated there; it is not added again to the formula's output unless a separate contribution explicitly targets that output.

A replacement operation, when enabled, replaces the stage baseline before additions and multipliers. The highest declared priority wins. Equal winning replacements with different values are a conflict, not an ordering tie to resolve by file position. Fixed conflicts fail validation; a conflict arising from a proposed equip or effect application rejects that change transactionally. Equal replacements with the same value coalesce. Bounds use the strongest minimum and maximum; if these conflict, maximum is applied last and the explanation reports the conflict, matching the existing condition policy. Content validation should flag combinations it can establish in advance.

The ember sword fixture has one attached enchantment with two contributions: +3 fire resistance while wielded, and a four-point fire effect on an eligible successful hit, costing one charge. The passive resistance does not require charges. The on-hit contribution does. Both require sword condition greater than zero. The wielder's saved base fire resistance is 5; an independent blessing adds 2. Distinct additions stack in this fixture.

| Scenario | Before | Request | Expected after |
| --- | --- | --- | --- |
| Carry only | Base resistance 5; blessing active; sword in inventory | Inspect resistance | Effective resistance 7; sword contribution listed as inactive |
| Wield | Same state | Equip and wield sword | Effective resistance 10; saved base remains 5 |
| Re-evaluate | Sword wielded; effective resistance 10 | Refresh UI or recalculate | Resistance remains 10; no duplicate contribution |
| Unequip | Sword wielded; blessing active | Unequip sword | Resistance 7; sword trigger ineligible; saved base still 5 |
| Remove blessing | Sword wielded; blessing active | End blessing | Resistance 8; enchantment unchanged |
| Disenchant | Sword wielded; blessing active | Remove ember enchantment | Resistance 7; only ember contributions and their attachment state removed |
| Transfer | A wields sword; B does not; both have base 5 and no blessing | Transfer sword to B's inventory | A resistance 5; B resistance 5; B becomes 8 only on a valid wield operation |
| Broken sword | Sword wielded; blessing active; condition becomes 0 | Recalculate after breakage | Resistance 7; both ember contributions inactive |
| Repair | Same broken sword; blessing active | Restore condition above 0 through a valid repair | Resistance 10; trigger eligible only if charges remain; repair does not replenish charges |
| No charges | Sword wielded, condition positive, charges 0; blessing active | Inspect or attack | Resistance 10; no ember on-hit effect |

The hit fixture begins with two enchantment charges and a target at 20 health. Its physical calculation is deliberately simple: rolled physical damage 10, quality factor 0.8, condition factor 0.5, then physical reduction 1, yielding 3. The independent fire effect starts at 4 and undergoes fire reduction 1, yielding 3. The target ends at 14 health, with the loss reported as two components; the enchantment has one charge left. These factors specify a test calculation, not the final damage or durability curve. In this fixture quality and condition do not scale magic; condition zero disables the enchantment as stated above.

The event policy is one trigger opportunity per original committed successful hit and target, made with this sword. A miss or cancellation consumes no charge. A landed hit reduced to zero physical damage still counts as a successful hit. The secondary fire effect does not emit another eligible weapon-hit trigger, preventing recursion. Repeated delivery of the original hit event produces no second fire effect or charge cost.

The fire effect and its charge decrement commit together after the primary hit. If the primary hit removed the target or the effect otherwise fails validation, the trigger is skipped with no charge consumed; the primary hit stays committed. A legally applied effect fully reduced to zero still consumes its charge. Multi-target actions must declare whether charges are per target or per action; this fixture grants no implicit multi-target behavior.

Saving while wielded preserves the sword attachment and remaining charges. Loading reconstructs resistance 10 with the blessing, or 8 without it, and never replays a hit. If a sword additionally grants knowledge, its grant follows the same temporary-versus-permanent learning contract as the character example.

Required explanation example: "Fire resistance 10 = saved base 5 + ember sword 3 + blessing 2." The hit explanation separately lists physical 10 × 0.8 × 0.5 − 1 = 3 and fire 4 − 1 = 3, identifies the source sword and enchantment, and records charges 2 → 1. Tooltips must distinguish this passive bonus from the charged trigger.

### Review gate before schema design

The proposed contracts are coherent only if all four examples can share identity, quantities, references, action outcomes, contribution sources, and persistence semantics. Review should resolve any contradictory outcomes before fields or runtime types are selected.

The next schema draft should map each owned fact and transition above to an authoring location, resolved meaning, saved state where needed, and inspection output. It should explicitly identify remaining extensions, such as reservations, mixed liquids, seasonal recovery, and more elaborate damage stages. Their absence must not leave the specified basic outcomes ambiguous.

## Relationship to the rendering and networking concepts

The [native WGPU rewrite](native-wgpu-modular-rewrite.md) separates authoritative runtime state from graphical presentation. This foundation follows that boundary: rules execution, recovery, learning, bonuses, and scenario testing remain usable without GPU initialization. Resource bindings may be authored through existing TOML UI, but their schema and semantics must not depend on a particular widget implementation.

The [persistent networking concept](networking-mmo-concept.md) supplies the relevant host, identity, and persistence boundaries. Resource contention, request duplication, item transfer, and clock policies must have the same meaning in offline play and a shared world. This concept defines the gameplay requirements without prescribing a wire protocol or implementing either rewrite.

## Decisions to refine before implementation

The following are proposed starting points for review, rather than final schema commitments.

| Topic | Proposed starting point | Remaining decision |
| --- | --- | --- |
| Default content | One integrated adventure and sandbox ruleset | Initial material, profession, and recipe coverage |
| World authoring | Ruleset resource binding plus allowed local TOML overrides | Exact syntax and selection scope for painted tiles, areas, and prefabs |
| Output identity | All outputs reference registered effective-ruleset items | Category-based output selection and property propagation |
| Material sourcing | Every required ingredient traces to an obtainable world source; vendors redistribute accounted supply | Initial source catalogue, delivery policies, and acquisition completeness reports |
| Armor families | Complete declared wearable sets with sourced crafting and repair inputs | Bone armor definitions, slot coverage, recipes, and profession naming |
| Supply | Explicit units, finite or effectively unlimited pools | Standard volume unit and multi-output harvest representation |
| Ecology | Recovery in world time, explicit lifecycle changes | Recovery curves, sustained damage, regrowth, seasons, stopped-world policy |
| Characters | Structured classes and open Human class share skills and knowledge | Starting attribute budget, talents, growth, advancement bounds |
| Learning | Permanent acquisitions and temporary active grants with provenance | Review the behavior contract; define forgetting, retraining, and final advancement policy |
| Item bonuses | Shared contributions with explicit subjects, activation, source identity, and deterministic stages | Review replacement and bound conflicts; reconcile with existing combat calculations and choose rounding policies |
| Item magic | Attached properties with independent passive and charged contributions | Review hit and charge contract; define broader chaining and enchanting acquisition |
| Transactions | Immediate effects prepare and commit consistently | Mixed effects, long actions, reservations, partial success, and script contracts |
| Editing live worlds | Separate authored defaults from saved live state | Hot-reload reconciliation and explicit reset or migration workflows |

Before implementation, review the proposed behavior and ownership specification, then define the smallest shared schema capable of expressing its state transitions and explanation outputs. Check that schema against existing behavior, and expand the catalogue only after those semantics are coherent. The result should let authors start with a small playable world and deepen it without replacing its foundations.
