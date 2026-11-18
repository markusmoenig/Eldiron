# Shared gameplay operations

Date: 2026-11-14

Status: Proposed general behavior contract, with an initial resource-operation implementation as of 2026-11-17. Operation records, states and quantities below remain illustrative rather than a supported universal schema or API.

Implementation update (2026-11-17): the [resource handler](../crates/rusterix/src/server/region_resources.rs) stages extraction outputs, tool wear, skill gain and vessel changes before committing them with source stock under the shared world lock. Local and online play use that handler; player requests also use the existing sequencing and durable receipt journal. [Runtime](../crates/runtime/tests/world_resources.rs) and [dedicated-host tests](../servers/dedicated/tests/world_resources.rs) cover the resource slice. The [crafting planner](../crates/rusterix/src/server/region_crafting.rs) now also stages solid/liquid inputs, action resource costs, tool wear, skill gain and outputs, with deployed station checks. The complete supplied bone-armor chain is tested in local and dedicated runtimes and through the multiplayer codec. The same planner now implements exact owned-item repair, preserving identity, quality and opaque future properties while committing inputs, tool wear and condition together. The bone maintenance policy also provides post-damage wear and shared broken-equipment activation. Local/dedicated persistence and encoded-host tests cover repair retries and foreign-target rejection. This does not implement the proposed general operation records, timed reservations, enchanting or a journal for arbitrary NPC/script effects. See the [current resource guide](../docs/docs/creator/authoring.md#bind-world-resources) and [request contract](../docs/network-gameplay-requests.md).

## Purpose and proposed decisions

Give harvesting, transfers, crafting, repair, enchanting, and triggered effects a common preparation, validation, commitment, and outcome contract. Keep their domain rules in resource definitions, recipes, item policies, and effects. The shared contract specifies how their changes become one consistent gameplay outcome.

This develops the operation boundary proposed by the [foundation concept](ruleset-foundation-concept.md), [data model](ruleset-data-model-concept.md), [contribution contract](ruleset-contribution-contract.md), and [world resource contract](world-resource-lifecycle-contract.md). The [gap review](ruleset-foundation-gap-review.md) identifies existing inventory planning and action paths that can be reused when implementation is eventually authorized.

The proposed decisions are:

- Preview and preparation do not consume materials or apply gameplay effects.
- The authoritative runtime selects exact participants and validates the complete operation before commitment.
- Immediate operations expose all their mandatory changes together or none of them.
- Resource debit, input consumption, output placement, target mutation, tool wear, skill gain, and cooldown belong to that same boundary when declared.
- Domain policies distinguish whole batches, positive partial transfers, and valid effects whose numerical result is zero.
- Timed operations declare start, interruption, reservation, and completion policies. They do not hold one transaction open for their whole duration.
- Follow-up effects have explicit separate commitments and cannot retroactively undo a committed primary action.
- Retries, repeated events, save/load, and several views must not duplicate a commitment.
- Solo, player multiplayer, and authored NPC behavior use the same operations and source accounting.

The [current authoring decision](ruleset-editor-concept.md) keeps ruleset maintenance internal while ordinary creators build worlds from the supplied content. Costs, durations, quantities and failure policies belong to that content; local resource settings may be exposed through world bindings. Exact commitment, source accounting, identity preservation and replay protection remain engine guarantees. Internal editing cannot turn a half-applied operation into a supported shortcut; a paid start step is instead an explicitly defined operation with its own coherent outcome.

## Ownership and operation identity

### Definitions supply meaning

| Owner | Supplies |
| --- | --- |
| Resolved action | Targeting, shared requirements, independent action costs, timing, cooldown, and execution policy |
| Resource definition and binding | Supply ownership, output conversion, extraction quantity, and lifecycle consequences |
| Recipe | Ingredients, outputs, tool and station requirements, quality calculation, and advancement opportunity |
| Repair operation | Exact target eligibility, repair inputs, condition change, and preserved state |
| Enchanting or recharge operation | Target eligibility, sourced inputs, attachment creation or charge restoration |
| Contribution and effect definitions | Applicable values, trigger eligibility, component calculations, and source attribution |
| Authoritative runtime | Current participants, actual input selection, state validation, random outcomes, commitment, and outcome identity |
| Behavior or player input | Which permitted operation is requested and any allowed explicit selections |

The generic craft action does not repeat a recipe's ingredients. An independent action fee may add another declared cost with its own owner and explanation. If two legitimate costs consume the same item, the plan sums their demands; it cannot use one unit to satisfy both. Duplicating the same fact is a resolution issue, not an extra automatic cost.

All outputs reference registered definitions and authorized transformations or world sources. A client, script, or NPC cannot manufacture output just by submitting an item ID and desired quantity. The operation derives its output from the selected definition and validates its inputs or source debit.

### Request operation and plan are distinct

A request names an authorized intent and permitted parameters. The host resolves the actor and its current incarnation from the controlling session or trusted local/behavior context. It does not trust a client to claim another actor's inventory or supply authority.

One accepted logical request receives one operation identity. A plan proposes a commitment for that operation against identified rules and state. Preparation may repeat after a state conflict, but that does not create another logical operation, another independent random roll, or another entitlement to output.

The identity context includes world, actor and incarnation, request generation and identity, operation, relevant definition revision, and any parent operation or trigger identity. Exact item, attachment, resource, pool, access-point, and station identities remain available throughout. A definition ID or display name cannot substitute for an individual instance.

A newly created output receives one persistent identity. Replaying its outcome returns that same output reference. Repair and recharge mutate an existing identity. Enchanting creates a new attachment identity on the existing item; retrying does not create another attachment.

## Preview and authoritative preparation

### Preview has no authority

A preview can show eligibility, likely selected inputs, capacity after consumption, costs, outputs, duration, and failure policies. It carries the rules and state revision it inspected. It cannot reserve materials, wear tools, spend charges, grant skill, or change current health merely by calculating values.

The actual request is validated again. A player may lose reach, a station may be removed, another actor may collect the final branches, or a live definition may change after preview. An obsolete preview is not authorization to use old state. The final outcome reports what was actually selected and committed.

### Resolve participants and the complete change

Preparation establishes:

- The actor, exact mutable target if any, resource and access point, and relevant rules dependencies.
- Explicit item, tool, vessel, input-stack, or attachment selections.
- Automatically selected inputs and participants under the definition's deterministic selection policy.
- Input quantities, units, transfers, outputs, target mutations, wear costs, advancement changes, and cooldown.
- Consequences of item breakage, equipment contributions, lifecycle transitions, and maximum-resource policies.
- The affected state and conditions that must still hold at commitment.

Explicit selections remain exact. If the selected cuirass disappears, repair cannot silently choose another cuirass. An invalid explicitly selected knife cannot be replaced with an enchanted knife. Automatic selection follows permitted matching, access, and preservation rules with a deterministic tie-break. The data-model fixture prefers ordinary eligible tools before enchanted tools, then stable instance identity.

An input may be spread over several compatible stacks. Selection records actual quantities from each. Categories, differing quality, attachments, material properties, and nested inventory access follow explicit matching policies. An ordinary material requirement does not silently authorize destroying an attachment or discarding properties that its transformation must preserve.

One participant can have several roles only where the operation defines their compatibility. A reusable tool cannot also be consumed as an ingredient or deleted to free an output slot by accident. A repair target is not an ingredient. Destructive transformations such as salvage need a declared target-consumption rule.

### Validate prospective capacity and effective state

Validate ownership and access, actor incarnation, knowledge and skill, lifecycle and supply, quantity and unit compatibility, sufficient tool condition for the full cost, deployed station requirements, target eligibility, and all other costs.

Capacity uses the complete prospective state after consumption and transfers. A full inventory can craft when consuming ingredients frees enough room. A partially occupied stack does not free a slot. Incompatible outputs cannot be merged to make a plan fit. Liquid inputs debit contents and preserve the vessel unless the definition explicitly consumes it.

Equipment, condition, or attachment changes also validate prospective contributions and required current-resource adjustments. Breaking a maximum-health item may require a declared current-health clamp. That consequence belongs to the commitment; a later tooltip calculation cannot cause it.

Recovery, expiry, and disposal due at the authoritative world time are settled before operation validation under their own contracts. Rejection does not undo those independent time changes.

## Immediate commitment and outcomes

### One logical commitment

The runtime revalidates participants and dependent state, then applies the plan as one visible gameplay transition. Readers cannot observe ingredients gone while their output is absent, or bones awarded while the remains opportunity is still available.

Mandatory changes and the committed outcome identity belong to the same logical boundary. A successful craft cannot lose its declared skill gain or cooldown because those updates ran in unrelated callbacks. Domain handlers can prepare specialized changes while preserving this guarantee.

If another operation changes a participant, the runtime can reprepare within the original intent and selection constraints or terminate with a state-conflict outcome. It cannot apply stale deltas, widen permissions, switch an explicit target, or retry until an unrelated inventory becomes a valid substitute. Replanning retains the operation identity and any fixed random outcome.

One authority owns commitment of the affected state. If participants span ownership boundaries, the host routes or coordinates a supported complete operation, or rejects it. Partial success across independently committed resource and inventory owners is not acceptable. This is a gameplay invariant rather than a choice of database or network protocol.

### Outcome distinctions

| Outcome | Gameplay meaning | Cost and effect policy |
| --- | --- | --- |
| Rejected request | Malformed, unauthorized, invalid identity, or unsupported operation | No operation-specific gameplay changes |
| Failed validation | Requirements, capacity, stock, target, or state do not permit commitment | No operation-specific costs, outputs, wear, training, or cooldown |
| Pending | Declared timed operation or coordinated attempt has not reached a terminal outcome | Only explicitly committed start steps or reservations may exist |
| Cancelled or expired attempt | Pending operation ended before completion | Follow its start-cost and reservation policy |
| Committed application | Declared operation applied, including valid partial quantity or zero numerical effect | Declared costs and mandatory consequences apply once |
| Committed gameplay failure | Authored paid attempt failed its gameplay resolution, such as a chance-based craft | Explicit failure costs and consequences apply |

Validation failure and paid gameplay failure differ. The deterministic production fixtures below have no chance-based failure, partial crafting, or failure cost. A future skill roll must specify paid success and failure plans rather than obtain material loss from an implementation error.

A positive water transfer is an application even if smaller than requested. A zero transfer is a no-op failure. An ember effect legally reduced to zero damage is a committed application and consumes its charge. Domain policy determines meaningful application; a universal requirement that every number must change would be incorrect.

Cancellation after commitment cannot undo an operation. The response reports the committed outcome. A refund, reverse transfer, or compensation is another explicit operation with its own eligibility and accounting.

## Timed operations reservations and start costs

### Default timed fixture

The reviewed default has no reservation and no start cost. Starting records pending work and required explicit selections. It does not subtract ingredients or promise future stock or output capacity. Completion settles due time changes, revalidates the actor and explicit participants, selects any automatic commodity inputs, and prepares one commitment.

Automatic selection at completion can use newly available equivalent materials within the original matching rules. It cannot change an explicit selection, consume a protected valuable item, or choose another mutable target. An explicitly selected station must still be valid; automatic station selection follows its declared completion-time policy.

Interruption produces no completion costs, outputs, wear, training, or cooldown. Another actor may collect the supply during the work; completion then fails normally. A relevant semantic rules change cancels this pending attempt as stale without completion costs. Cosmetic changes do not change operation semantics.

The operation names its duration clock and pause behavior. Duration does not follow render frames or an unexamined wall clock. Region rendering or view closure alone is not interruption. The fixture cancels on explicit interruption or loss of the acting incarnation. Disconnect and restoration policies must be declared; the simple no-cost default cancels pending attempts on actor disconnect or restoration rather than completing them implicitly.

Persistently resumable work is optional. It retains the same operation identity, clock progress, explicit selections, committed start steps, and reservations, then revalidates before completion. It neither creates a new request nor reapplies a start cost on load.

### Optional reservations

A reservation records a claim on identified state. It is not completed extraction or another inventory copy. Available finite stock for other operations is stock minus applicable outstanding reservations. Completion converts the claim into the actual debit and outputs once. Releasing a claim restores availability, not extra physical stock.

A reservation declares scope, ownership, lifetime, clock, cancellation, and expiry. Ingredients or funds may move into identified escrow. Item and attachment targets remain exact. Reserving ingredients does not guarantee future station, actor, target, or output-capacity eligibility; those dependencies still require validation.

Reservations do not suspend unrelated lifecycle rules unless explicitly supported by the resource. If remains expire or a target is destroyed, releasing its claim cannot recreate missing stock. Stock, claims, and physical materials remain separately accounted.

The complete reservation policy must be supported before an operation requests it. The basic harvesting and production examples do not depend on reservations.

### Optional start and failure costs

An operation can commit an explicit start step, such as a lesson payment or setup material. That step has its own identity and outcome. Later cancellation does not pretend it never occurred.

Refundable materials or currency should remain in identified escrow until disposition is decided. Irreversibly consumed fuel or effort need not be refundable. A refund transfers or releases accounted value once; it cannot become an unexplained material source.

Preview and outcome distinguish start, success, paid-failure, and cancellation costs and possible refunds. The generic framework does not silently choose these gameplay economics for every profession.

## Random outcomes scripts and notifications

For randomized operations, the authority resolves randomness under the operation identity. Validation precedes the paid attempt, and the result is fixed for its commitment or recorded attempt. Conflict retries and repeated requests cannot reroll it. Revealing a result before allowing free cancellation requires an explicit gameplay rule; it is not ordinary preparation.

Scripts and behavior nodes request operations and receive structured outcomes. A preparation requirement hook obeys the same read-only and deterministic dependency contract as other checks. A world-mutating script cannot masquerade as a pure predicate.

A script-defined mandatory change must use supported planned mutations or its own explicit operation. Queuing a completion event does not make all future script execution part of the previous transaction. A notification failure cannot justify refunding ingredients, repeating outputs, or rolling back a successful craft.

Commitment records required follow-up obligations with identities that can be delivered safely. UI, audio, animation, logs, and diagnostic history describe outcomes. They do not grant rewards or determine success. Bounded observations alone are not the persistence or duplicate-handling contract.

## Worked example one harvesting branches

Use the living tree's whole-batch yield 2, capacity 12, recovery 4 per world day, one condition point of pruning-tool wear, and one eligible woodworking point per success.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Branch supply | 12, full, recovery inactive | 10; phase starts with first deadline one world day later |
| Actor inventory | 0 branches; capacity for 2 | 2 branches |
| Selected tool | Condition 100 | Condition 99 |
| Woodworking | 10 and eligible | 11 |
| Action cooldown | Ready | Declared cooldown starts once |

The plan resolves one tree through its access point and one exact usable tool. It validates output before debiting stock. A full inventory, insufficient tool condition, invalid lifecycle, or missing whole batch fails without changes to any row, apart from independent elapsed-time settlement.

With two collectors and stock 2, the first commitment receives two branches. The second sees stock 0 and fails without wear, training, or cooldown. Replaying the winner's request after recovery returns the earlier outcome without harvesting again.

Timed collection without reservation can begin while stock 2 exists. If another actor collects it first, completion fails without costs. An animation reaching its end cannot override that result.

## Worked example two filling a vessel

Use a finite well at stock 3 and an owned four-measure bowl containing 1 compatible freshwater measure. The request is 4. The permitted transfer is `min(request 4, stock 3, free capacity 3) = 3`.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Shared well pool | 3 water measures | 0 |
| Exact bowl contents | 1 freshwater measure | 4 freshwater measures |
| Bowl identity and other properties | Existing owned bowl | Preserved |
| Recovery phase | Next deadline day 0.5 | Same deadline |

Pool debit and content increase commit once. No loose water stack or replacement bowl is created. Filling declares no tool wear or advancement in this fixture, so the generic operation does not infer them from transferred volume.

At an unlimited river the same request transfers 3 without a finite source debit. A full bowl transfers zero and fails as a no-op. Incompatible saltwater contents, inaccessible access, or loss of bowl ownership fails without a transfer.

Several openings resolve to the same well pool. Competing plans must revalidate its actual stock. Repeated delivery after pouring the bowl out cannot repeat the old fill; another fill needs a new request.

Recipe liquid inputs use the same exact-content accounting. Preparing one bone plate from one bone and one water measure changes bowl contents 4 → 3, creates one plate, and applies declared knife wear and eligible training together. The bowl remains. If that plate cannot fit after input consumption, all those action changes are rejected.

## Worked example three crafting a bone cuirass

Consume eight bone plates and four straps, produce one cuirass, require an assembly tool and accessible deployed bench, charge one tool-condition point, and grant one eligible boneworking point. Output quality is 100 under the data model's explicit fixed policy.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Plate stack A | 5 plates | Consumed |
| Plate stack B | 3 compatible plates | Consumed |
| Strap stack C | 4 straps | Consumed |
| Assembly tool | Condition 56 | Condition 55 |
| Boneworking | 20 and eligible | 21 |
| Output | No new cuirass | One cuirass, quality 100, condition 100, unique output identity |
| Deployed bench | Valid and accessible | Unchanged reusable station |

An initially full inventory can succeed because consuming those three stacks frees room for the output. If instead ingredient stacks retain surplus in every occupied slot, the nonstackable cuirass may not fit. That prospective state fails without consuming its eight plates or four straps.

The cuirass is a planned output, not a reward granted by a success-message script. Materials, wear, and skill gain cannot repeat when a crafting notification is delivered twice.

For timed work naming a specific bench, losing that bench before completion fails without completion costs. Missing knowledge, insufficient inputs, or an invalid tool follows the same failure contract. Making the complete set is five recipe operations. A future batch request must explicitly choose whole-batch atomicity or per-piece commitments and report partial progress honestly.

## Worked example four repairing an existing piece

Target one exact owned bone cuirass at condition 80. Consume one prepared plate and one strap, use the repair station, charge one tool-condition point, and restore up to 20 condition with maximum 100. Create no armor and replenish no enchantment charges.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Cuirass | Existing identity; condition 80; quality 100 | Same identity; condition 100; quality 100 |
| Materials | 1 prepared plate and 1 strap | Both consumed |
| Tool | Condition 56 | Condition 55 |
| Existing ember attachment | Same attachment; charges 1 | Preserved; charges 1 |
| Equipped location and other properties | Current valid state | Preserved unless explicitly changed by the repair definition |

The plate is already prepared here. In the complete-set chain, preparing it from surplus bone is a preceding operation consuming one water measure and one knife-condition point. Starting at knife condition 56, preparation leaves 55 and repair leaves 54. These are two commitments; failed repair does not undo a prepared plate.

A piece at condition 95 reaches 100 for the full declared cost. A piece already at 100 is ineligible and consumes nothing. A broken piece can reach 20 under this policy, reactivating its normal equipped contributions. Recalculation does not add a permanent bonus to saved actor attributes.

If the target is transferred, removed, or becomes ineligible, repair fails without selecting another cuirass. Repair training needs its own declaration; it is not inherited from the original crafting recipe.

## Worked example five attaching an enchantment

The actor knows ember inscription, owns one sourced inlay, and selects an unbroken owned bone cuirass with no ember attachment. A valid inscription station is accessible. Consume the inlay and attach ember with two initial charges to that exact cuirass.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Ember inlay | Quantity 1 | Consumed |
| Cuirass | Existing identity, quality, condition, equipment state | Preserved |
| Ember attachment | Absent | One new persistent attachment, charges 2 |
| Other attachments | Existing valid attachments | Preserved subject to declared compatibility |

Input consumption and attachment creation commit together. If the cuirass is worn and its passive applies, fire resistance becomes 10 from saved base 5, ember 3, and blessing 2. Saved base remains 5. An equip callback does not increment it.

A second ember application is rejected under the fixture's same-attachment policy. It consumes no inlay and grants no second passive. Missing station access, unknown inscription, wrong target kind, broken target, attachment conflict, and lost ownership also fail before consumption.

Making an inlay is a separate recipe consuming one plate and one resin with its declared wear and training. The enchanting operation declares no extra wear or training in the data model; it must not inherit the reagent recipe's costs accidentally. Every inlay still traces to world-sourced bone, water, and resin.

## Worked example six recharging

Select an exact ember attachment at one charge out of two, with an owned sourced inlay, knowledge, and station access. Consume the inlay and restore up to two charges, clamped to the attachment maximum.

| Participant | Before | Planned committed state |
| --- | --- | --- |
| Attachment | Charges 1, maximum 2 | Charges 2; identity preserved |
| Inlay | Quantity 1 | Consumed |
| Target item | Existing condition and quality | Unchanged |
| Other attachments | Existing state | Unchanged |

Only one charge is actually restored because capacity is one, but the cost remains one whole inlay. There is no implicit refund for unused restoration potential. An attachment already at two is ineligible and consumes nothing.

The data-model recharge definition does not require an unbroken item. It therefore permits recharging a broken owned item, but does not repair it or activate magic requiring positive condition. Repair and recharge are different operations.

Of two attempts on the same attachment, the first may fill it. The second revalidates and fails at maximum without consuming another inlay. A delayed request cannot install new ember when the selected attachment has been removed.

## Worked example seven ember after a primary hit

Use an authorized ember-bearing attack source under the contribution contract. The bone chain does not assume its knife can serve as a combat source; that remains an explicit action policy. Here the source is eligible and remains unbroken through the primary commitment.

Target health starts at 20, primary physical damage resolves to 3, and ember starts at two charges. The primary first commits health 20 → 17 and its own declared costs and wear. Its outcome records the original successful-hit identity and exact source.

A child operation then prepares fire amount 4 against the same valid target with effective fire resistance 1. Health 17 → 14 and charges 2 → 1 commit together. Fire does not inherit actor POWER or physical weapon scaling unless its effect explicitly declares those inputs.

| Situation after primary commitment | Secondary outcome | Effect on primary |
| --- | --- | --- |
| Valid source, target, and charge | Fire and charge debit commit together | Remains committed |
| Fire resistance 6 against amount 4 | Zero fire damage; one charge consumed | Remains committed |
| Primary killed or removed target | Skipped; no charge consumed | No rollback |
| Primary wear broke source | Skipped under post-hit activation policy | Primary and wear retained |
| Ownership or attachment invalid | Skipped with reason; no charge consumed | Remains committed |
| Another eligible hit spent last charge | No effect or charge debit | Remains committed |
| Repeated original hit delivery | Existing child outcome returned or pending child resumed | No duplicate application |

Child identity derives from original hit, target, attachment, and trigger definition. It is not an unrestricted client command. A committed child does not emit another original weapon-hit trigger. Multi-target and repeated-hit actions need explicit frequency and charging rules beyond this single-target fixture.

After primary commitment, reporting the whole action as if nothing happened is incorrect. The outcome retains that result and identifies the child as committed, skipped, or pending. Mandatory consequences cannot move into optional children merely to weaken atomicity.

## Persistence retries and uncertain delivery

Persist enough identity and outcome information to distinguish a request that never committed from one whose response was lost. Repeating a pending request observes the same pending operation. Repeating a terminal request returns that outcome, including recorded failure; a fresh attempt needs a new request identity.

Identity is bound to parameters. Reusing it with another target, quantity, actor, or recipe is rejected. Retired outcome records require expired request generations to be rejected rather than treated as new work. Bounded retention cannot make ancient requests valid material-creation attempts.

Gameplay commitment and durable storage are related but distinct. A checkpoint must contain a consistent cut of changed state, outcomes, committed start steps, escrow or reservations, and required child obligations. Recovery cannot combine consumed inputs from one checkpoint with absent outputs or an old request ledger from another. The host may distinguish in-memory commitment from durable acknowledgement under its persistence model; no database is prescribed here.

Restoring pending work follows its cancel-or-resume policy. Restoring committed work does not rerun it. Reconstructing contributions, appearances, or views cannot register extra bonuses, spend charges, or grant training. Child obligations resume under their original identities and validate applicable current state before their own commitment.

These semantics coordinate with the [networking and persistence concept](networking-mmo-concept.md). Local play uses the same principles through its local host. NPC jobs and online player requests cannot bypass source accounting through separate material-generation paths.

## Structured outcome and explanation

A result exposes structured facts rather than only a boolean or success string:

- Operation, request, actor, target, source, parent, and child identities.
- Terminal status or pending phase, with stable failure, cancellation, expiry, or skip reason.
- Relevant rules revision, input snapshot, and committed state revision where applicable.
- Selected stacks, vessels, tools, stations, supplies, attachments, and actual quantities.
- Costs, outputs, preserved identities, mutations, advancement, cooldowns, and lifecycle consequences.
- Expected children and their independent statuses.
- Calculation and limitation traces from the contribution and resource contracts.

Example: "Crafted Bone Cuirass. Plates 5 + 3 and straps 4 consumed; knife condition 56 → 55; boneworking 20 → 21. Created item 42, quality 100, condition 100." Repair identifies the existing item and preserved attachment. Failure explains why nothing committed while distinguishing independent time settlement.

Creator, terminal, gameplay messages, behavior nodes, and network-facing views can summarize or expand the same outcome. A client may predict a preview; the authoritative result determines actual inventory and world changes.

## Acceptance and consolidation review

| Scenario | Expected invariant |
| --- | --- |
| Invalid or stale explicit source | No silent substitute or completion costs |
| Capacity after ingredient consumption | Prospective capacity determines success; no partial loss on failure |
| Several input stacks or vessels | Selected quantities validate and commit together |
| Tool also matching an ingredient | No accidental consumption or double use |
| Competing final supply or charge | One valid commitment for the shared amount |
| Positive partial fill versus zero transfer | Explicit quantity policy; no reward for a no-op |
| Missing knowledge or station | No outputs, input loss, wear, training, or cooldown |
| Repair and recharge | Target identity preserved; only declared properties change |
| Enchanting and retry | One reagent consumed and one attachment created |
| Timed interruption without start costs | No completion changes or fictional reservation guarantee |
| Reservation or escrow cancellation | Release accounted state once without creating stock |
| Relevant definition change during work | Declared stale policy; no mixed-schema commitment |
| Primary and later child | Primary retained; child independently validated and committed |
| Randomized paid attempt and retry | No free reroll or duplicate failure cost |
| Save/load or lost response | Consistent state/outcome cut; no committed work replay |
| Several views, solo, online, or NPC | Same authoritative operation semantics |

These are expectations for the full contract, not a claim that all scenarios have been executed. The resource subset now has the tests linked in the implementation update; the first complete bone production slice is now tested; the supplied bone repair scenarios are also tested; enchantment and timed-operation scenarios remain planned.

Review the remaining timing, reservation, chance-failure, persistence and balance choices while completing the supplied RPG content and its world acquisition paths. The former public guided-extension editor proposal is superseded; internal maintenance tools can support this work without adding configuration steps for ordinary creators. Implementation is now permitted; future work should extend the tested resource boundary without treating this whole proposed schema as implemented.
