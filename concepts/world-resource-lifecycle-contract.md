# World resources and lifecycle

Date: 2026-11-14

Status: Behavior contract with an initial implementation as of 2026-11-17. The exact supported schema is defined by [the resource model](../crates/ruleset/src/world_resources.rs), with [bundled reference examples](../crates/ruleset/rulesets/eldiron/v1/world_resources.toml). Proposed records and balance values elsewhere in this document remain illustrative.

The current pilot binds map sectors, edges, and geometry objects through their Authoring TOML, and creates single-use remains from an explicit character `remains_profile` attribute. It includes named finite/unlimited supplies, shared pools across regions, simulation-clock recovery, expiry, optional exploitation suspension, tool wear and advancement, vessel filling, public lifecycle views and optional tile appearances. Resource operations use the existing authoritative request and persistence journal. Initial-stock and display-name edits preserve live stock; incompatible policy edits require explicit migration. Creator provides profile/pool choices and an undoable **Bind Resource** operation.

The supplied bone-armor chain now connects hand-gathered bootstrap resources and death-linked hides/bones to tools, vessels, deployed stations, processing and five equipped pieces. A binding may select an existing interaction for normal Use without changing its owner policy. Reusable prefab/paint-asset inheritance, mesh and collision replacements, growth/planting operations, timed work/reservations and migration tooling remain planned. The [runtime scenarios](../crates/runtime/tests/world_resources.rs) and [encoded server scenarios](../servers/dedicated/tests/world_resources.rs) identify the currently tested behavior.

## Purpose and proposed decisions

For the supported authoring workflow and copyable placement TOML, see [Bind world resources](../docs/docs/creator/authoring.md#bind-world-resources). The remaining examples below describe the broader contract and are not all supported configuration.

Define resource-bearing world features with the same meaning in solo and online multiplayer, and across 2D, isometric, 3D, and terminal presentation. A tree supplies materials because it is a configured world resource. Its sprite, mesh, and descriptions represent that resource; they do not own its stock or create rewards.

This contract refines the [integrated foundation](ruleset-foundation-concept.md) and the [data model and bone armor chain](ruleset-data-model-concept.md). The [gap review](ruleset-foundation-gap-review.md) distinguishes existing resource-item gathering from the persistent bindings, named supplies, and lifecycle semantics proposed here. The [contribution contract](ruleset-contribution-contract.md) continues to own effective values and item bonuses.

The proposed decisions are:

- One logical resource instance owns its lifecycle and named supplies, regardless of how many visual parts represent it.
- Several access points can explicitly share a pool. Each pool has one authoritative stock and recovery history.
- Ruleset definitions own valid outputs and behavior. Map TOML selects bindings and permitted local values. Saved state owns live stock, phase, and lifecycle.
- Every output references a registered item and an allowed source relationship. Resource extraction follows accounted world supply.
- Recovery follows the declared world clock, with equivalent outcomes while loaded or unloaded.
- Resource and inventory changes commit together. Competing actors cannot both collect the same last supply.
- Single-use remains cannot refill through reopening, rendering, loading, or repeating a death notification.
- NPC use follows the same operations. Economic participation and adaptation remain author-defined gameplay.

The [RPG editor model](ruleset-editor-concept.md) separates these guarantees from gameplay choices. Authors select content and supported finite, unlimited, renewal, and lifecycle policies, then configure relevant quantities and durations. Unique stock ownership, meaningful units, consistent unloaded settlement, and duplicate prevention remain enforced. The exact tree and well numbers and optional exploitation rule are review fixtures, not mandatory defaults for every RPG.

## Definition binding instance and access point

| Concept | Owns | Example |
| --- | --- | --- |
| Resource definition | Supplies, allowed outputs, interaction parameters, recovery, lifecycle, and permitted overrides | Mature tree or skeleton remains |
| Authored binding | Selection of a definition and allowed local configuration for a logical placement | This tree starts with twelve branches |
| Persistent instance | Current lifecycle, supply state, timing, and relevant history for one individual | Tree 17 currently has four branches |
| Shared pool | Stock and recovery explicitly shared by several access points | Two well openings drawing from one finite reservoir |
| Access point | Where and how an actor can interact with an instance or pool | Riverbank contact, a well opening, or a tree's reachable trunk |
| Presentation part | Appearance or selectable geometry linked to the logical identity | Canopy cells, trunk mesh, water faces, or text entry |

A resource instance may own its supplies directly or reference an explicit shared pool for a supply. Both models have one owner per stock value. A second opening to the same well cannot install a second water counter or recovery schedule. Local access conditions can differ: one opening may be blocked while another remains usable.

The association between gameplay and map content must be explicit. A Water tile role or a `tree` tag can help authoring and classification, but does not automatically instantiate unlimited water or harvestable wood. A resource profile and binding establish that meaning.

### Identity rules

A persistent resource identity belongs to the world and logical placement or dynamic creation event. It is distinct from the definition ID, artwork asset ID, current runtime numeric ID, and render cache identity.

One tree spanning several cells has one logical instance, even if its canopy and trunk are selected separately. Two trees using the same image have separate instances. A river can expose many access surfaces that all resolve to its one configured source. Grouping adjacent cells into one resource is an authored relationship, not an inference from matching pixels or adjacency alone.

Rebuilding geometry, switching cameras, entering a region, or rebaking sprites preserves the logical resource identity. Painting or duplicating a new tree normally creates a new placement and initial instance. Merely replacing artwork on an existing binding preserves its identity and live state. Deleting and replanting is a declared lifecycle or creation operation; it cannot happen accidentally because a preview generated new object IDs.

Dynamic remains use the defeated creature's persistent incarnation and death-event identity to establish one corpse resource. Several skeletons sharing an authored template still have distinct remains. A genuine later creature incarnation may create new remains through its own death; it does not refill the old corpse.

Authoring copies initialize from authored defaults, not from an implicit clone of live stock. Explicit world-copy or migration tools must state how they preserve or remap identities. Copying during ordinary gameplay is not an extraction operation and must not duplicate materials.

## Ownership and local configuration

| Fact | Owner | Local configuration or live state |
| --- | --- | --- |
| Output item identity and quantity unit | Registered ruleset item | Permitted source selection; resulting item quantities |
| Supply meaning and output conversion | Resource definition | Allowed quantities and current stock |
| Tool, skill, range, cost, and quantity policy | Resolved interaction and referenced action | Actual actor, source tool, request, and target |
| Default capacity, recovery, and lifecycle | Resource definition | Explicitly exposed placement overrides |
| Shared stock and schedule | The referenced pool | Access points reference it without copying it |
| Initial stock and state | Resolved authored binding | Used once when creating a new instance |
| Remaining stock and lifecycle progress | Persistent runtime instance or pool | Mutated by committed operations and world time |
| Appearance and collision consequences | Presentation binding and explicit world-transition policy | Selected from committed semantic state |

Binding resolution applies resource defaults, any explicit reusable asset or prefab binding, and then allowed placement overrides. A shared pool's stock and recovery policy come from the pool owner; access points cannot independently override that same shared policy.

Map TOML may configure supply, yield, recovery, or initial state only where exposed by the selected definition. Output overrides must reference both a registered item and a permitted output of that supply. A tree's branch supply cannot produce an unrelated registered sword simply because its item ID exists.

The earlier tree TOML remains an illustrative authoring view. A convenient `yield_per_use` field must normalize to one interaction quantity and output conversion. If the conversion is ambiguous or non-integral, validation rejects that shorthand and requires an explicit quantity declaration. The resource definition remains the canonical owner of its output conversion; the action does not repeat a second yield table.

Editing an initial value does not refill an existing instance. Changing a default, capacity, output, or recovery policy requires an explicit live-state reconciliation policy where existing instances are affected. An unresolved or renamed definition is reported for migration; it is not replaced with a fresh instance.

## Supplies quantities and output accounting

A resource can contain several named supplies. Each declares stock meaning, unit, capacity mode, output conversion, availability, and optional recovery. The name distinguishes supplies such as branches, bark, resin, and a felling opportunity on one tree.

| Stock meaning | Quantity consumed | Output conversion |
| --- | --- | --- |
| Item quantity | Two branch stock units | One registered branch item per stock unit, yielding two |
| Volume | Three water measures | One registered water measure per stock unit, transferred into a vessel |
| Harvest opportunity | One skeleton extraction opportunity | Six registered raw bones per opportunity |

Remaining stock is never negative. Count and volume use their declared representations; one count cannot substitute for a volume measure. Harvest opportunities are not silently treated as individual output items. For the examples, all quantities are integers.

Capacity is explicitly finite or unlimited. Unlimited is a validated mode, not a negative integer or infinity stored as ordinary remaining stock. An unlimited river supplies finite requested volumes under vessel capacity and interaction rules. It does not place infinite water into a player's inventory or remove access requirements.

For finite stock, extraction debits the declared amount and creates or transfers exactly the declared output bundle. If several outputs belong to one opportunity, the whole bundle must fit unless the interaction explicitly defines partial results. Supporting outputs cannot be silently lost to fit the main output.

Each interaction declares its quantity policy:

- **Whole quantity:** collect the specified batch or make no action changes. The branch and skeleton fixtures use this policy.
- **Clamped positive quantity:** transfer the minimum of the request, available stock, and compatible destination capacity. Water filling uses this policy.

A zero transfer is a no-op failure and grants no output, training, wear, or success costs. A positive partial transfer reports its actual quantity. Ruleset policy determines any training or costs for that actual operation; there is no implicit reward per mouse click.

Several supplies do not imply a universal physical mass simulation. Their relationship is declared by lifecycle transitions and conversions. Felling may disable branch and resin supplies and grant logs. If potential branch stock is forfeited, that is an explicit consequence of felling, not a hidden extra reward or a deletion of already collected inventory items.

## Time recovery and lifecycle

### Clock ownership

Ecological recovery in these fixtures uses one monotonic world clock with an identified epoch. A world day is a duration in that clock, not a real calendar date or time-of-day value that wraps. The host defines how world time advances in real-time, turn-based, or other gameplay modes.

World time can advance while a region is unloaded, allowing its resources to recover. A paused world clock advances no resource recovery. Stopping a solo game or server does not imply wall-clock recovery. Offline catch-up is a separate authored host policy; when enabled, it explicitly advances or maps into the world clock before resource settlement. Player count and render frequency do not directly advance a resource's clock.

The same resource policy applies online and offline. A persistent server may keep its world clock running with no connected players. A solo host may pause it on exit. Those are explicit host or game-time choices, not separate resource implementations.

### Proposed replenishment phase

For a recoverable finite supply, the first successful depletion from full stock starts the recovery phase. If a new instance starts below capacity, initialization starts its phase unless an explicit saved or authored phase is supplied. Further extraction does not restart the phase.

Each complete interval adds the declared recovery amount, capped at capacity. Reaching capacity ends that phase and discards unused recovery. A full resource cannot bank replenishment for the next harvest. A later depletion from full begins a new phase at that operation's world time.

With remaining stock `S`, capacity `C`, recovery amount `R`, and `k` completed eligible intervals, simple recovery gives `min(C, S + k × R)`. The schedule must also preserve the next deadline when still below capacity, and end it when capacity is reached. That formula applies only across an interval with no intervening extraction or lifecycle change.

Omitted recovery means no replenishment. Disabling access does not automatically disable a pool's recovery. A separate damage or lifecycle rule can suspend it. When a suspension ends, these fixtures begin a fresh interval; skipped intervals during suspension are not saved up.

### Lifecycle is separate from stock

A bare living tree can have zero branch stock and later recover. A stump is a different lifecycle state whose living-tree interactions are unavailable. A dry well can have zero water while its reservoir remains recoverable. Spent skeleton remains are a single-use source whose opportunity never replenishes.

Transitions declare which supplies are retained, reset, discarded, or disabled; which recovery schedules end or start; and any changes to access, collision, or presentation. Growing or regrowing a feature cannot accidentally initialize every supply again on each intermediate visual update.

The basic tree fixture has no automatic regrowth after felling. Planting, stump removal, and regrowth can be authored with explicit requirements, durations, and supply initialization. Replenishment of branches is not a substitute for that lifecycle.

### Unloaded settlement and ordering

Settlement must produce the same authoritative state whether the resource was observed continuously or unloaded until the next interaction. Simple recovery can use arithmetic catch-up. More elaborate growth rules need an equivalent chronological or aggregate calculation. A performance limit must not silently drop elapsed recovery or create extra stock.

At a given world time, settle due lifecycle transitions, recovery, and expiry consistently before accepting an interaction against that state. Apply scheduled events in their defined chronological order; lifecycle changes that cancel a recovery schedule remove that pending recovery. Corpse expiry at the same time as a harvest request takes effect first in these fixtures.

Then validate and serialize competing interaction commitments against the current resource and destination state. Tie-breaking or fairness among simultaneous player requests is a separate gameplay or host scheduling policy. The result must still debit each unit once. Several views or access points cannot each settle the same recovery interval independently.

An action rejected after time settlement does not undo legitimate recovery or expiry. It makes no action-specific stock, inventory, tool, cooldown, or training changes.

## Interaction and contention contract

An interaction identifies the actor, logical resource and supply, selected access point, requested quantity, exact tool or vessel where relevant, and applicable rules and state revisions. The access point resolves to the authoritative stock owner before any output is prepared.

Validate lifecycle and supply availability, reach and any line-of-sight policy, ownership or access permissions, skill or knowledge requirements, usable tools, destination compatibility and capacity, and all costs. Reach uses declared world-space interaction rules. A successful visual pick alone does not establish reach or permission.

Prepare one state transition covering stock debit, all outputs, vessel contents, tool wear, eligible skill gain, cooldown, and lifecycle consequences. Commit them together. General operation planning belongs to the later shared operation contract; this resource contract requires that these changes cannot be split into unrelated successful steps.

For the timed fixture policy, starting an interaction makes no reservation and charges no start cost. Completion revalidates all inputs and commits atomically. Interruption commits nothing. If another actor collects the supply first, completion fails without costs. Reservations or charged attempts can be added as explicit operation policies rather than being assumed here.

Duplicate delivery of the same request returns its recorded outcome without repeating changes, even if the resource has recovered since the first commitment. Reusing that request identity with different parameters is rejected. A genuinely new request is evaluated against current stock. Save/load and request-handling state must preserve this distinction within the host's defined replay window. When outcome records are retired, requests from an expired request generation are rejected rather than executed again as new requests.

There is one authority for a shared stock and its revision even when access points lie in different regions. Region routing and network transport are implementation concerns; clients and neighboring region views cannot each commit their own copy of the same pool.

## Example one a living tree

### Reference definition and placement

The ruleset defines `wood_branch`, `tree_bark`, `tree_resin`, and `wood_log`, the permitted tree outputs, and interactions for collection and felling. The placement binds to a mature-tree profile. Actual item definitions and lifecycle fields remain proposed content rather than an assertion that these exact IDs are currently registered.

The branch supply has capacity and initial stock 12, yields 2 branches per collection, and recovers 4 per world day. A usable pruning tool loses 1 condition per committed collection. An eligible success grants 1 woodworking point under the fixture's advancement rule. The basic living-tree appearance is healthy at stock 8–12, sparse at 1–7, and bare at 0.

The same tree can also contain finite bark and resin supplies, as in the bone-chain example. They do not replenish merely because branches do. Felling consumes one felling opportunity, yields 6 logs, requires a suitable axe, and changes the lifecycle to stump. It sets remaining living-tree supplies to zero, disables their interactions, and cancels their recovery. No automated regrowth is enabled in this fixture.

### Expected transitions

Rows are independent unless explicitly presented as a time sequence. Sufficient inventory capacity and valid requirements are assumed except where the row tests their failure.

| Scenario | Before | Expected outcome |
| --- | --- | --- |
| Collect once | Branch stock 12; carried branches 0; tool condition 100; woodworking 10 | Stock 10; carried branches 2; tool condition 99; woodworking 11; healthy appearance |
| Exhaust | Stock 2; otherwise eligible | Stock 0; branches +2; tool wear and training once; bare appearance |
| Incomplete batch | Stock 1; normal batch 2 | Rejected; no action changes |
| Inventory full | Stock 12; cannot fit 2 branches | Rejected; no stock debit, wear, training, or cooldown |
| Two final collectors | Stock 2; A and B both eligible | One receives 2; stock becomes 0; the other fails without costs |
| Several canopy tiles | All parts reference the same tree | Collection through any valid part debits that one supply |
| Identical neighboring tree | A different placement uses the same artwork | Its stock is independent |
| Fell | Living tree; branches 10; other supplies available; axe condition 100 | Logs +6; axe condition 99; stump; living-tree supplies 0; recovery cancelled |
| Fell or collect again | Stump | Rejected; no extra logs or branches |

For the recovery sequence, empty the branch supply at world day 0 with its phase beginning at that time. With no later harvesting, day 1 has stock 4 and sparse appearance, day 2 stock 8 and healthy appearance, and day 3 stock 12. Loading directly at day 3 gives the same outcome. Saving at day 0.5 preserves the half-day remaining until the first recovery.

If that full tree remains untouched until day 10 and is then harvested to stock 10, the next recovery is day 11. It cannot use the unused days 3–10 to refill immediately. Felling it at any time cancels its branch deadline; settling the former deadline later cannot restore the stump's stock.

### Optional sustained exploitation policy

A separate fixture enables damage after the third transition to empty in the rolling interval `(now - 3 world days, now]`. Emptying the same tree on days 0, 1, and 2 triggers damage on day 2. Collection and branch recovery are suspended for 3 world days, with stock 0. Damage ends on day 5 and a fresh recovery interval starts; the first 4 branches arrive on day 6.

The transition history and damage deadline are persistent state. Repeated attempts against an already empty tree do not count as new depletion transitions. This policy is optional authored ecology, not a mandatory penalty applied to every tree or game. Stock-based appearances and damaged-lifecycle appearance are separate; damaged or stump presentation takes precedence over the ordinary branch threshold.

Required explanation example: "Collected 2 Wood Branches from tree 17. Branch supply 12 → 10; pruning tool condition 100 → 99; woodworking 10 → 11. Next recovery in 1 world day."

## Example two a river with several access points

The river binds to an unlimited freshwater source. Several water faces, tiles, or riverbank access points refer to that source. Water quality and permitted outputs come from the definition and local binding, not the artwork's color or the number of visible water cells.

The actor owns a four-measure vessel. Filling transfers the minimum of requested volume and compatible free vessel capacity; available source stock does not impose a finite limit in unlimited mode. Water is a registered liquid item held as contents, and the vessel preserves its own identity and properties.

| Scenario | Before | Expected outcome |
| --- | --- | --- |
| Fill | Empty four-measure bowl; request 4 | Bowl contains 4 freshwater measures; source remains unlimited |
| Top up | Bowl contains 3 freshwater measures; request 4 | Transfer 1; contents become 4 |
| Full vessel | Bowl contains 4; request more | Zero transfer rejected; no training or wear |
| Incompatible liquid | Bowl contains 2 saltwater measures | Freshwater filling rejected; existing contents unchanged |
| Inaccessible bank | Correct source; actor out of reach or blocked under the action's access policy | Rejected despite a valid water selection |
| Two actors | Each owns an empty four-measure vessel and has access | Each can receive 4 through its own valid request; no shared finite stock is consumed |
| Duplicate request | First fill committed, then actor poured the water out | Replaying that request does not refill; a new request can |

An unlimited source still requires quantity validation and configured per-interaction limits. It has no depleted flag, refill countdown, or remaining-volume counter. A drying river requires a declared lifecycle or supply-mode transition; it is not inferred from fewer players, different rendering detail, or a hidden stock reset.

Multiple access points do not multiply a finite source into an unlimited one. If a game wants a finite river reservoir, it binds those points to an explicitly finite pool using the well-style contract below.

Required explanation example: "Transferred 1 Fresh Water measure from river 2. Requested 4; vessel free capacity limited transfer to 1. Contents 3 → 4. Source mode: unlimited."

## Example three a finite recoverable well

The well draws from a named finite pool with capacity 12 measures, initial stock 3, and recovery of 3 every half world day. Initialization below capacity begins a phase at day 0, so the first deadline is day 0.5. A fill request uses clamped positive quantity and a compatible owned vessel.

Two openings may refer to the same pool. Their position and local access conditions are separate, but remaining water and recovery phase belong to the pool. Sealing one opening prevents use there without stopping reservoir recovery. A drought or poisoned-source lifecycle would need an explicit pool policy.

| Scenario | Before | Expected outcome |
| --- | --- | --- |
| Limited fill at day 0 | Pool 3; empty vessel capacity 4; request 4 | Transfer 3; pool 0; vessel 3; next deadline still day 0.5 |
| Fill empty well | Pool 0 before next deadline | Zero transfer rejected; no costs or training |
| Shared final supply | Pool 3; two openings; two eligible requests for 4 | First committed request receives 3; the other fails with no transfer |
| Full destination | Pool 3; vessel already full | No action debit; recovery schedule unchanged |
| Blocked opening | Pool available; selected opening sealed | Access failure; another valid opening can still be used |
| Region reload | Pool 0; deadline day 0.5 | Reload preserves pool 0 and deadline; it does not reapply initial stock 3 |

Starting from the first row's empty pool and making no further withdrawals, day 0.5 has 3 measures, day 1 has 6, day 1.5 has 9, and day 2 reaches capacity 12 and ends recovery. Any access point observing the pool at those times sees the same quantities. Loading the pool at day 2 computes 12 rather than four independent opening-local refills.

A request arriving exactly at day 0.5 first settles recovery to 3 and can then transfer up to 3. A rejected request does not reverse that scheduled replenishment. Withdrawals while the pool is below capacity do not restart its half-day phase.

Recovery represents the declared world source replenishing the reservoir. Transport to a vendor or producer must withdraw actual volume from the pool. Adding an NPC delivery schedule cannot create a second refill path or credit more water than the same pool can provide.

Required explanation example: "Transferred 3 Fresh Water measures from reservoir 4 through well opening B. Requested 4; available stock limited transfer to 3. Pool 3 → 0; vessel 0 → 3. Next recovery: 3 measures at world day 0.5."

## Example four skeleton remains

A skeleton's committed death creates one remains instance for that creature incarnation and death event. The reference profile has one harvest opportunity yielding six registered `raw_bone` items. It has no recovery and uses `corpse_extraction_only` delivery. This supplies the bone-armor chain without unrelated random material drops.

The fixture allows collection by hand, requires reach and inventory capacity for all six bones, and has no tool wear or training reward. A different corpse-processing interaction can declare tools or skills, but those costs and requirements are explicit. Potential bones are part of this remains supply until extracted; they are not also pre-awarded into ordinary loot.

| Scenario | Before | Expected outcome |
| --- | --- | --- |
| First extraction | Opportunity 1; bones carried 0 | Opportunity 0; bones carried 6; remains become spent |
| Reopen or inspect | Opportunity 0 | No new bones and no replenishment |
| Insufficient output space | Opportunity 1; cannot fit six bones | Rejected; opportunity remains 1; no bones awarded |
| Two collectors | One opportunity; both can fit output | One receives 6; the other fails; total extracted remains 6 |
| Repeat death notification | Remains already created for that death | Same remains identity and state; no second source |
| Save and load | Spent remains, or an unspent saved opportunity | Preserve 0 or 1 respectively; never reinitialize from the profile |
| New skeleton incarnation | A genuinely new creature later dies | New remains with its own opportunity; old remains stay spent |

If a game chooses direct death loot instead, transferring the six bones into its loot destination and consuming the opportunity are one operation. Later corpse processing sees no remaining opportunity. If that transfer cannot commit, it leaves the opportunity unconsumed and reports failure; it does not automatically fall back to another reward mode. Delivery mode is a ruleset choice, and switching it for a live corpse requires reconciliation of already awarded material.

For an expiry fixture, remains expire at world day 2 after a death on day 0. Unspent opportunity is forfeited at expiry and can no longer be harvested. At exactly day 2, expiry is settled before a collection request. Already collected bones remain in their owners' inventories. Ordinary items stored in a corpse container follow a separately declared corpse-disposal policy; this fixture contains no other loot.

Spent or expired remains may become hidden or be removed according to presentation and disposal policy. Disposal does not erase the death/source identity needed to reject duplicate creation or extraction within the host's replay contract. A respawning encounter creates a new creature through its authored spawning rules, not by resetting a disposed corpse.

Required explanation example: "Collected 6 Raw Bones from remains 81. Harvest opportunity 1 → 0. Source: skeleton incarnation 19, death event 27. No further bone extraction available."

## Presentation access and world consequences

Presentation consumes committed lifecycle and stock state to select appearances, descriptions, and interaction hints. It can show a bare tree, a dry well, or spent remains without changing supply. A terminal can describe the same states and deadlines. No harvest completion depends on waiting for a particular animation frame.

Collision and navigation consequences are explicit world changes where needed. Felling can replace a solid trunk with the declared stump collision shape. A shared pool can remain intact while one access point becomes blocked. A visual disappearance alone must not silently remove collision or make another access point usable.

The authoritative runtime validates the access point's relationship to the resource and its current position. Clients submit an interaction request rather than an award amount inferred from pixels or local stock. Several simultaneous views read the same world state and do not independently advance simulation.

## Saved state editing and recovery

The persistent record needs enough information to restore identity and derive the same future outcomes:

- Definition and resolved-binding revision, world identity, logical placement or creation origin, and any pool reference.
- Lifecycle state and deadlines, including suspended or pending transitions.
- Each named supply's remaining stock or unlimited mode, capacity policy, recovery phase, and next deadline where applicable.
- Relevant history such as depletion timestamps for sustained exploitation.
- Dynamic creation and extraction identities, handled outcomes, or coordinated host replay records needed to prevent duplicates.
- Access-point state when it can change independently of its resource or pool.

Artwork caches and derived appearance labels need not become authoritative save state. Initial stock, remaining stock, and capacity remain separate values. A save at a full supply stores an inactive recovery phase; it does not retain a bank of missed intervals.

Changing a live capacity below remaining stock, swapping an output, renaming a supply, splitting or joining pools, or changing clocks requires an explicit reconciliation plan. It may preserve overflow, convert state, or discard potential supply under a declared policy with a report. A reader must not silently clamp, reset, or reinterpret quantities just to make the new definition load.

Authoring previews and ordinary fresh Play sessions can initialize temporary resources from authored defaults. Persistent test sessions resume their saved state. An explicit reset command can restore initial state in an identified test world; stopping or restarting a view is not that command.

## Validation explanation and acceptance

Validate registered and permitted outputs, stock units and conversions, initial stock versus finite capacity, positive recovery intervals and amounts, compatible quantity policies, valid lifecycle transitions, unique instance ownership, and consistent pool references. An unlimited source cannot also declare an ordinary finite remaining-stock value. A single-use remains supply cannot acquire recovery through an unrelated default.

World acquisition review traces collected materials to these source identities, including tool, vessel, skill, knowledge, and access dependencies. Vendor and NPC production uses the same accounting. Whether NPCs adapt to shortages or player participation is authored gameplay, not an extra stock-generation rule.

Interaction explanations include source, access point, supply, request, actual quantity, limiting requirement, output conversion, stock before and after, costs, training, lifecycle changes, and next eligible recovery. Pure inspection can show pending time settlement, labeled separately from the committed snapshot, without writing authoritative state or committing action rewards. Failed actions distinguish exhausted supply, invalid lifecycle, insufficient tool condition, missing capacity, incompatible contents, reach, permission, and stale source identity.

| Acceptance case | Required result |
| --- | --- |
| One tree represented by many parts | One stock, phase, and lifecycle across all presentations |
| Several trees sharing artwork | Independent identities and depletion |
| Tree recovery observed versus unloaded | Identical stocks at days 1, 2, and 3; no bank after full |
| Felling with recovery pending | Stump remains unavailable; old deadline cannot restore branches |
| Sustained exploitation | Damage on day 2, suspension through day 5, first refill day 6 |
| Unlimited river | Finite capacity-limited transfers without a fabricated finite stock |
| Finite well and shared openings | One pool depleted and recovered once; partial fills conserve volume |
| Due recovery and an interaction at the same time | Settle the scheduled state before request validation |
| One skeleton death and several delivery attempts | One source; at most six bones under the fixture |
| Remains expiry | No extraction at or after expiry; collected inventory unaffected |
| Simultaneous extraction or request replay | No duplicate outputs, wear, training, or supply debit |
| Region, renderer, or view restart | No reset of persistent supply or dynamic source creation |
| Live definition change | Explicit reconciliation with inspectable consequences |
| Solo, NPC, and online player use | Same authoritative source and operation semantics |

These are expectations for future scenario tests, not test results. Document checks can establish consistency of the examples but cannot prove runtime, UI, persistence, or networking support.

## Shared operation boundary

The companion [shared operation contract](shared-operation-contract.md) defines how resource stock, inventory inputs and outputs, tool wear, vessel contents, skill gain, repair targets, and enchantment attachments are prepared and committed together. This document establishes the resource invariants that operation planning must preserve.

Before choosing final field names, review resource grouping and access-point authoring, live-definition reconciliation, world-clock and offline policies, optional growth and exploitation rules, and timed-action reservation choices. No CPU-to-GPU implementation work or mandatory economic controller follows from this concept.
