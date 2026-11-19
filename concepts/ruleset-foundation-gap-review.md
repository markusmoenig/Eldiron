# Ruleset foundation capability gap review

Date: 2026-11-13

Status: Historical read-only source review and proposed design directions. Test definitions were inspected; builds, tests, gameplay scenarios and multiplayer sessions were not run during the original review. The comparison below records that baseline, rather than the current implementation status.

Implementation update (2026-11-17): the shared catalogue/domain metadata, guided five-piece armor family and initial world-resource layer now exist. Map bindings, named supplies, shared pools, recovery/expiry, tool wear, vessel filling, death-linked remains, saved identity and atomic extraction are covered by Authoring, local runtime and dedicated-host tests. See the current [capability audit](../docs/docs/ruleset_capability_audit.md), [editor guide](../docs/docs/creator/docks/ruleset_nodes.md) and [resource guide](../docs/docs/creator/authoring.md#bind-world-resources). Resource operations use shared authoritative execution and the existing player request journal; a universal operation framework is not implied. The complete supplied bone-armor production slice is now implemented, including liquid recipe inputs, owned tools and deployed station requirements; [runtime](../crates/runtime/tests/bone_production.rs) and [encoded-host](../servers/dedicated/tests/bone_production.rs) tests cover it. Immediate targeted repair, opt-in damage wear and broken-equipment activation now extend that slice. Unified item contributions and enchantment attachments remain gaps. The dated findings below must be read with this update.

Implementation update (2026-11-18): passive equipment, enchantment and condition values now share one evaluator and source trace, with a supplied Strength cuirass and ownership/persistence coverage. Item/action-local contributions, talent grants, charged triggers and advanced policy groups remain gaps; the original source review below records the earlier baseline.

Source baseline: working tree at Git HEAD `f8d88011`. The ongoing rendering rewrite can move these source locations; the named functions and behaviors identify the evidence being reviewed.

Learning update (2026-11-18): recipe knowledge now has permanent acquisition records, one-time class/character starting grants, NPC teaching, reusable owned manuals and separate temporary equipment grants. Advanced Boneworking demonstrates the same knowledge through all sources while leaving the bootstrap recipes open. See the [implemented contract](../docs/docs/rules_in_eldiron.md#persistent-learning). The historical learning gaps below are resolved for this recipe slice; general talents and other service policies remain separate.

Enchanting update (2026-11-18): the Bear inscription slice now provides gameplay acquisition of a passive attachment, with sourced reagents, learned knowledge, atomic exact-item mutation and persistence/ownership coverage. The [current contract](../docs/docs/rules_in_eldiron.md#targeted-enchanting-recipes) supersedes the historical passive-attachment acquisition gap for this scope; trigger, recharge and advanced-policy gaps below remain.

## Main finding

Product decision updated on 2026-11-17: generic ruleset editing is internal maintenance tooling, hidden in standard Creator builds. Ordinary creators build worlds from supplied content and choose starting options; the historical public guided-editor recommendations below are superseded by [the current authoring decision](ruleset-editor-concept.md). The first complete bone-armor production slice is supplied; further production chains remain a content priority.

Eldiron already has substantial reusable gameplay infrastructure: resolved actions, attribute predicates, derived formulas, timed condition modifiers, equipment policy, item stacks, transactional recipe inventory updates, skill advancement, behavior services, and inspection tools. The proposed world model should extend those foundations rather than introduce another independent rules engine.

The main missing contract is a consistent way for a persistent source to contribute values, capabilities, or triggered effects to an item, actor, or action. Existing item bonuses are real, but their meanings are distributed between effective attributes, source-item fields, equipment sums, and combat-specific calculations. Generalizing them requires settling ownership before adding another bonus path.

The [foundation concept](ruleset-foundation-concept.md) defines desired behavior. The [data model and bone armor chain](ruleset-data-model-concept.md) illustrates how that behavior might be authored. This review identifies which proposed features reuse current mechanisms, which extend them, and which require a new shared contract. Those classifications describe the inspected paths, not production-readiness claims.

The subsequent [effective values and source contributions contract](ruleset-contribution-contract.md) develops the first recommended decision with explicit armor, POWER, condition, and enchantment examples.

The [world resource and lifecycle contract](world-resource-lifecycle-contract.md) develops the resource-instance decision with explicit map identity, supply, time, contention, and corpse-extraction semantics.

Subsequent product direction on 2026-11-14 scopes these recommendations to RPGs and RPG sandbox worlds. The [editor concept](ruleset-editor-concept.md) consolidates the author-facing workflows and distinguishes engine guarantees, selectable policies, and content. Shared or generic capabilities below serve that scope; they are not a promise to support every gameplay genre. The dated source observations remain unchanged by this clarification.

## Capability comparison

| Proposed capability | Existing support | Gap | Recommended direction |
| --- | --- | --- | --- |
| Project-owned rules and shared resolution | Rules document, validation, typed action and condition resolution, cached runtime catalogues | Several domains still use raw tables; proposed fields are not current schema | Extend the existing document and resolver with explicit domain typing |
| Numeric modifiers | Derived attributes and condition add, multiply, and bounds | Equipment, talents, and attachments do not feed one common contribution path | Generalize effective-value evaluation and source attribution |
| Equipment bonuses | Weapon, armor, and all-equipment sums; source-item combat bonuses | No universal activation, lifetime, or dependency contract | Normalize wearer contributions and action-local contributions separately |
| Enchantments and charges | Attributes and conditions can represent pieces of state | No inspected typed item-attachment, charge, and hit-trigger contract | Add reusable attachments over shared modifiers and effects |
| Map resources | Ruleset-backed resource items with gather, depletion, and respawn | Tile or geometry metadata does not establish named persistent supplies | Bind existing map identities to one resource-instance model |
| Quantities and vessels | Integer stacks and item containers | No inspected liquid-volume, content compatibility, or vessel-input contract | Extend quantities with volume and separate contents from vessel identity |
| Crafting transactions | Recipe execution clones actor inventory, consumes inputs, adds outputs, then commits | Not one cross-domain plan for resources, liquids, tools, targets, and attachments | Reuse prepare-and-commit semantics with domain-specific operations |
| Tool requirements and wear | Explicit owned source item and condition cost on script actions | Restricted effect combinations; no general tool capability or station requirement | Extend resolved requirements and exact source selection |
| Armor sets and repair | Slots, category permissions, ordinary armor, quality, durability | Explicit family coverage and identity-preserving repair operation are missing | Make set pieces content; add small shared coverage and repair contracts |
| Sandbox development | Classless actions, skill attributes, crafting skill gain, optional progression | No general learned-recipe registry or permanent versus temporary grant provenance | Add one acquisition model without imposing class or level requirements |
| NPC authored behavior | Node runtime, world services, action invocation, attributes and inventory guards | Current node action surface is narrower than runtime targets and source selection | Extend behavior services with needed queries and structured outcomes |
| Explanation and source coverage | Console and CLI inspection, action descriptions, event observations, headless test arenas | No full value-contribution trace or world acquisition graph | Extend shared inspection and validators; reuse existing test fixtures |

Evidence and the practical limits of each row follow below.

## Item properties and bonuses

### Existing paths

The server's [effective attribute evaluator](../crates/rusterix/src/server/region.rs#L15694) reads a saved attribute, evaluates a derived formula using effective dependencies, applies formula bounds, and then applies active condition modifiers. The [condition modifier implementation](../crates/rusterix/src/server/region.rs#L15784) already combines additions, stack-scaled multipliers, and bounds without rewriting base attributes. [Typed condition modifiers](../crates/ruleset/src/lib.rs#L1945) and [typed derived stats](../crates/ruleset/src/lib.rs#L1992) provide reusable resolution and validation boundaries.

Equipment follows additional paths. [Equipment helpers](../crates/rusterix/src/server/region.rs#L22336) separately sum weapon, armor, or all equipped item attributes. [Combat variable resolution](../crates/rusterix/src/server/region.rs#L22380) exposes actor attributes, source-item values, and equipment sums as distinct inputs. [Structured damage calculation](../crates/rusterix/src/server/region.rs#L22516) reads actor and source bonuses and separately subtracts effective defender attributes and equipped armor values. The official [combat reference tables](../crates/ruleset/rulesets/eldiron/v1/combat.toml) show that division explicitly; they are conversion references rather than the production authoring source.

Item-specific attributes are already copied from definitions into templates in [item extraction](../crates/ruleset/src/lib.rs#L6369). The official reference includes `POWER` and resistance values on its [ritual equipment](../crates/ruleset/rulesets/eldiron/v1/equipment.toml#L684). Therefore, saying that item bonuses are entirely absent would be incorrect. Storing or summing an item attribute is nevertheless different from universally contributing it to the wearer's effective attributes.

For example, the inspected effective-attribute evaluator does not collect equipped `POWER`. Ordinary ruleset spell damage [passes source item ID zero](../crates/rusterix/src/server/region.rs#L19698), and the [structured damage evaluator](../crates/rusterix/src/server/region.rs#L22560) resolves no source item for zero. Those paths do not make an equipped focus's `POWER` a general wearer bonus. This is a source-level observation about that path, not a claim that every authored spell or combat formula behaves identically.

Client action-state calculation has its own [effective-value evaluator](../crates/rusterix/src/client/rules_ui.rs#L551) with derived formulas and condition modifiers. A server-only contribution change would leave another consumer to reconcile. Pure formula parsing can be reused, but the effective-value contract must also cover the input sources and calculation stages.

### Ownership decision before generalization

Use one effective-value contract for actor attributes. Keep an item's own rating distinct from the contribution projecting it to its wearer or to a particular action. Preserve semantic attribute roles; the draft's `physical_protection` and `fire_resistance` should not accidentally become competing engine constants alongside the existing configured armor role and attribute IDs.

Consider saved armor 5, condition bonus 2, and equipped armor 3. Current structured reduction can obtain 10 by combining the effective actor value 7 with equipment 3. If equipment is added to the effective actor value but the separate equipment sum remains, the result becomes 13. The current split is not itself double-counting; an incomplete migration would introduce it.

Recommended direction:

- Reuse saved attributes, semantic roles, derived formulas, and modifier arithmetic.
- Give every contribution a source instance, subject, activation context, stacking identity, and lifetime.
- Evaluate item-local ratings, actor attributes, and action-local values in explicit scopes.
- Choose exactly one projection of equipment into the relevant effective value; remove or adapt the corresponding raw equipment sum when that projection is introduced.
- Use the same evaluation contract for authoritative execution, action availability, terminal inspection, and explanations.

This is a shared value contract, not a requirement to store every fact in a new `properties` subsystem. A property registry can add metadata and validation to existing attribute identities where useful. Maintaining two independent stat systems would recreate the problem.

### Quality condition and activation differences

The current [weapon quality and condition curve](../crates/rusterix/src/server/region.rs#L22497) rounds and clamps each percentage to 1–100, maps it to `0.75 + percent / 400`, multiplies the two factors, and preserves at least one damage for a positive input. The data-model fixture's fixed output quality and the foundation's illustrative damage factors do not describe that current curve.

The inspected armor-sum helper does not itself test for condition greater than zero. If an item remains equipped, simply setting condition to zero does not make that helper omit its attributes. Existing [durability processing](../crates/rusterix/src/server/region.rs#L23982) supports timed drain and optional destruction, which is another behavior rather than a universal broken-item activation policy.

Before choosing new defaults, decide how broken items, diminishing effectiveness, formula bounds, rounding, and repair interact. Treat changes to those semantics as deliberate ruleset changes with scenario expectations, rather than incidental consequences of refactoring bonuses.

## Attachments events and persistent identity

[Items](../crates/rusterix/src/server/item.rs#L6) already contain serializable attributes, nested items, a runtime ID, and a creator UUID. [Stack compatibility](../crates/rusterix/src/server/item.rs#L137) compares stack identity, quality, and condition. It does not implement the draft's general attachment or material-property equality policy.

The inspected resolved definitions and item state do not provide a typed enchantment attachment catalogue, independently persisted attachment identities, or a general charge-and-trigger operation. Dynamic attributes can store ad hoc state, but do not establish activation, transfer, removal, replay, or stacking semantics.

There are existing [damage events](../crates/rusterix/src/server/region.rs#L20081), condition lifecycle events, and [structured event observations](../crates/rusterix/src/server/event_observation.rs#L1). The latter are explicitly bounded diagnostic observations. They are a useful vocabulary and inspection surface, not a durable transaction ledger or a guarantee that a zero-damage successful hit emits a reusable trigger event.

Recommended direction: reuse ordinary item state and existing event infrastructure, adding the missing attachment identity and semantic event contract. Define original hit identity, source item, target, success, charge commitment, secondary-effect ordering, and recursion rules. Extend stack equality when attached or material state becomes meaningful. Coordinate persistent identity with existing [behavior actor identity and incarnation](../crates/rusterix/src/server/nodes/mod.rs#L22) and the networking concept rather than assigning a third unrelated identity scheme.

The ember examples remain proposals. A damage notification by itself cannot prove that a charged on-hit effect behaves correctly on misses, zero damage, dead targets, repeated delivery, or save/load.

## World resources and map bindings

### Current resources are gameplay items

[Resource identification](../crates/rusterix/src/server/region.rs#L17612) reads item attributes such as `resource_id` and looks up `[resources]`. [Gather execution](../crates/rusterix/src/server/region.rs#L17669) targets a world item, validates availability and range, creates a ruleset-defined output, commits inventory insertion, and marks the item depleted. Respawning resources become invisible until their countdown ends.

The output currently comes from the resolved action's give-item effect, with a resource `produces` table as fallback. This is existing duplicate ownership that the new design needs to settle. The [respawn update](../crates/rusterix/src/server/region.rs#L24508) decrements a remaining timer while processing the item; it is not the proposed named-stock recovery and unloaded-world catch-up model.

This provides useful harvesting behavior but not twelve branches on one living tree, a separately available resin supply, shared river areas, or a stump lifecycle. Current harvesting depletes the node as a whole rather than subtracting from multiple independently defined supplies.

### Metadata and interaction reuse

The editor already reads and writes geometry-object, sector, and linedef text through [the Authoring dock](../creator/src/docks/authoring.rs#L495). [Geometry objects](../crates/rusterix/src/map/geometry_object.rs#L294) have UUID identity and a property container. [Tile metadata editing](../creator/src/actions/edit_tile_meta.rs#L47) exposes role, blocking, alias, gameplay tags, and rendering material metadata through the existing parameter UI and TOML conversion.

Those storage and UI mechanisms are useful, but accepting text or a gameplay tag is not evidence that `world_resource` is parsed or enforced. The inspected [tile asset representation](../crates/rusterix/src/map/tile.rs#L408) does not carry the proposed resource profile. A tile asset's identity is also not the identity of every painted placement.

Runtime targeting already includes [entity, item, prefab instance and part, and position targets](../crates/rusterix/src/server/region.rs#L18040). [Prefab interaction routing](../crates/rusterix/src/server/region.rs#L5150) can invoke a resolved rules action with prefab context. Reuse that target and placement groundwork; do not create a resource system limited to spawned inventory-item lookalikes or infer resources from visual meshes.

Recommended direction: extend the existing resource domain with a resource binding and persistent instance containing named stock, clock state, and lifecycle. Resolve existing map identities to that instance, including multi-cell features and explicit shared pools. The resource definition owns output conversions; interactions select quantities and operations. An authored action may still grant items for other purposes, but must not duplicate the resource's yield declaration.

## Quantities crafting tools and repair

### Existing inventory transaction pattern

[Recipe execution](../crates/rusterix/src/server/region.rs#L17186) resolves inputs and outputs, checks requirements, clones the actor, consumes the clone's inputs, adds its outputs, awards eligible skill gain, and only then replaces the authoritative actor. Output capacity therefore accounts for consumed input slots, and failed insertion does not partially commit that actor inventory change.

[Typed action effects](../crates/ruleset/src/lib.rs#L2351) and [prepared state modifications](../crates/rusterix/src/server/region.rs#L18352) supply another useful prepare/apply pattern. However, these are domain paths, not one general transaction across a map resource, multiple inventories, liquid contents, tool wear, and an existing target item's attachment state.

The resolver [rejects certain mixed effects](../crates/ruleset/src/lib.rs#L3055): state modification is limited in its combinations and cannot simply be added to a recipe to implement repair. Modification recipients are actors or entity targets, not arbitrary owned item instances. A [script-result action](../crates/rusterix/src/server/region.rs#L18774) queues an event after its own policy and costs; it does not await that script and roll back its costs if a later scripted transformation fails.

Recommended direction: retain specialized operations with one shared outcome and commitment contract. Add typed input selections and pending changes for the actual domains involved. Do not claim that an arbitrary list of existing effects is already atomic, or replace working crafting with a separate bone-specific executor.

### Liquids and requirements are real extensions

[Item containers and quantities](../crates/rusterix/src/server/item.rs#L77) support nested item storage and integer stacks. They do not establish liquid volume, compatible contents, vessel capacity, or recipe consumption from several vessels while preserving the vessels. A bag is not automatically a flask just because both have capacity.

[Recipe requirements](../crates/rusterix/src/server/region.rs#L17054) currently check skill and an optional known spell. The inspected executor does not enforce a deployed station capability or a general tool-capability requirement. [Recipe validation](../crates/ruleset/src/lib.rs#L5358) checks references and item quantities, rather than the proposed station, tool, and world acquisition graph.

An explicit owned source item and wear are already useful foundations, but the [source-effect resolver restrictions](../crates/ruleset/src/lib.rs#L3076) limit their composition. Generalizing exact tool selection to capabilities and built-in harvesting or crafting is substantive work, not merely changing the item ID in TOML.

The inspected [source selection and wear helpers](../crates/rusterix/src/server/region.rs#L18492) match the declared item and subtract wear with a clamp at zero. Those helpers do not require enough remaining condition to pay the full cost. The data-model proposal's sufficient-condition eligibility is therefore another explicit semantic change to review, rather than existing behavior to assume.

Recommended direction: start with count and volume quantity domains and explicit clock units, reusable tool and deployed-station requirements, and selection of actual input and tool instances. A general physical-units engine or an independent crafting framework per profession is unnecessary for the examples.

### Bone armor and repair scope

Ordinary armor pieces, slots, categories, and [equipment permissions](../crates/ruleset/src/lib.rs#L3962) already exist. Much of an initial five-piece bone set is content: item definitions, armor values, recipes, and appearances. Its existence does not require a new bone-armor runtime branch.

Explicit family coverage reports, source-complete materials, and common repair policy extend that content model. Repair additionally needs a validated mutation of an exact owned item while committing material consumption and preserving identity, quality, and attachments. Timed durability drain does not provide that operation. Shared armor-family defaults can compile to resolved values without introducing a broad inheritance system first.

## Learning classes and behavior hooks

### Existing learning and classless behavior

[Skill lookup](../crates/rusterix/src/server/region.rs#L17033) already defaults missing skill values to zero. [Crafting advancement](../crates/rusterix/src/server/region.rs#L17122) awards deterministic success-based progress with a cap and mastery margin. [Ability and spell checks](../crates/rusterix/src/server/region.rs#L17417) accept explicit actor lists and class unlocks. Some absent or unknown class paths are deliberately permissive; those fallbacks are not a permanent learned-knowledge record.

The inspected recipe path gates by skill and optional spell, not a general learned-recipe catalogue. There is no inspected source-aware distinction between permanent teacher acquisition and an active equipment grant that can later disappear. The data model's `knowledge`, `teaching`, and temporary grants therefore need a real contract.

Recommended direction: retain class progression and skill advancement, then give capability acquisition explicit provenance and lifetime. A general Human class can supply starting defaults without a special engine branch. Classes grant selected knowledge; they are not the only authority for learning it. Preserve valid classless behavior while replacing implicit fallbacks with deliberate availability policy where needed. Teacher costs, lesson availability, and profession access remain author-defined content.

### Behavior services should be extended

The current [node runtime and WorldServices interface](../crates/rusterix/src/server/nodes/mod.rs#L1) already separate behavior decisions from world operations. It supports action invocation, attribute access, inventory guards, dialogue, scheduling, and inventory offers. [Use Action nodes](../crates/rusterix/src/server/nodes/builtins.rs#L1080) and their [service implementation](../crates/rusterix/src/server/nodes/region.rs#L837) currently pass an optional entity subject; this is narrower than the runtime's item, prefab, position, and explicit source-item targeting.

Legacy script hooks also call [the shared action and recipe paths](../crates/rusterix/src/server/region_host.rs#L2420). A future hook design should cover the current node service boundary, rather than assume every author writes a legacy script. Item selection, resource queries, numeric stock or demand observations, and structured success/failure results can be exposed as actual gameplay needs require them.

Adaptive NPC economic participation remains optional authored gameplay in solo and multiplayer. The required foundation is shared authoritative operations and useful queries. A population-scaling formula, automatic workforce, universal price controller, or separate NPC-only production executor is not required by this review.

## Validation explanations and tests

[Rules documents and recovery](../crates/ruleset/src/document.rs#L45), [project runtime resolution](../crates/shared/src/project.rs#L626), and [runtime caches](../crates/rusterix/src/server/regionctx.rs#L80) should remain the backbone for new definitions. The production official source is compiled through [the bundled graph](../crates/ruleset/src/graph.rs#L27); illustrative flat TOML must be mapped into the existing authoring workflow before it becomes a public format.

The [validator](../crates/ruleset/src/lib.rs#L6146), [CLI inspection](../crates/ruleset/src/cli.rs#L1), and [action descriptions](../crates/rusterix/src/client/rules_ui.rs#L936) already provide meaningful inspection. Extend them with contribution traces, reference origins, resource state, operation previews, set coverage, and transitive acquisition paths. Existing bounded event observations can display outcomes, but cannot replace the calculation trace or saved grant provenance.

Inspected test definitions already cover conditions, restoration, sandbox actions, crafting failure, gathering, and ritual production. Examples include [condition restoration](../crates/rusterix/src/server/region_host.rs#L5088), [sandbox targets and source tools](../crates/rusterix/src/server/region_host.rs#L5717), [recipe failure without side effects](../crates/rusterix/src/server/region_host.rs#L6187), and [the ritual chain](../crates/rusterix/src/server/region_host.rs#L6528). Reuse those headless fixtures when implementation is eventually authorized. This review does not report that they currently pass.

New acceptance scenarios must cover the missing semantics, particularly no double-counted bonuses, independent resource instances, unloaded recovery, vessel-preserving consumption, exact-item repair, grant removal, and trigger replay. Solo and multiplayer need the same authoritative outcomes; network request deduplication and host clock persistence must coordinate with their existing architecture work.

## Simplifications to make in the proposed data model

| Draft idea | Reuse or simplify | Keep as a genuine new contract |
| --- | --- | --- |
| New property catalogue | Use metadata over existing attribute identities and semantic roles where possible | Scoped contributions, type and unit checks, calculation trace |
| Flat item catalogue and item kinds | Normalize current grouped item definitions; use composition rather than a broad new hardcoded kind switch | Stable validated references and explicit stack compatibility |
| Resource profiles and actions | Extend current resource/action resolution and existing placement targets | Named stock, instance binding, shared pools, lifecycle and clock policy |
| Generic operation vocabulary | Reuse action requirements, effects, inventory planning, and specialized handlers | Consistent commit boundaries across the selected domains |
| Tool and station catalogue | Use capabilities on ordinary items or world features | Requirements, access checks, deployment and exact source selection |
| Armor families | Compile modest shared defaults and explicit members | Set coverage, equipment contributions, and exact-item repair |
| Knowledge and teachers | Reuse class grants, skill growth, dialogue, and services | Permanent acquisitions, temporary grants, and provenance |
| Event and explanation system | Extend existing node services and structured observations | Causal hit identity, durable state, numeric traces, precise outcomes |
| NPC economic adaptation | Author behavior with shared queries and operations | No mandatory economic director or player-count controller |

This is a recommendation for the next design revision, not an instruction to rewrite the example documents into a final schema immediately. In particular, distributable package inheritance and asset-loader work should not become prerequisites for proving a local project-owned resource or contribution model.

## Smallest foundation decisions to settle next

1. **One value contract.** Decide which existing attribute identities are reused, how equipment projects item values, how action-local bonuses differ, and how all consumers obtain the same explanation. Resolve broken-item and quality behavior explicitly.
2. **One resource instance contract.** Decide how existing map, prefab, and item identities bind to named supplies, output ownership, recovery clocks, and shared areas. Preserve one meaning across all presentations.
3. **One operation outcome contract.** Extend the existing inventory transaction pattern to the chosen domain changes, then define what scripts and behavior nodes can observe. Separate immediate commitments from long-action policies.
4. **Persistent sources and acquisition.** Define item attachments and learned capability grants with source identity, lifetime, transfer, and save/load behavior. Coordinate with current actor and networking identities.
5. **Inspection and validation at each boundary.** Specify the contribution trace and acquisition report alongside the semantics. Use the four foundation examples and full bone set as acceptance cases.

The first recommended decision is developed in the contribution contract, with subsequent resource and operation contracts covering the other boundaries. The consolidated editor workflows now provide the next review surface: check which semantics need a supported policy choice, which are engine guarantees, and how an author creates a small or complex RPG through the same foundation. Outstanding gameplay choices and schema mappings still require review. Implementation remains outside this document's scope.
