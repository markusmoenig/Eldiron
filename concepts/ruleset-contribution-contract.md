# Effective values and source contributions

Date: 2026-11-13

Status: Proposed behavior contract for review. This refines the earlier concept examples without introducing supported schema fields or authorizing implementation. Numbers are explicit review fixtures rather than final balance values.

Implementation update (2026-11-17): an initial equipment lifecycle slice now implements category-owned positive-condition activation, declared wear after matching positive health damage, and exact owned-item repair. Combat equipment sums and equipment-stat presentation share the activation check; repair preserves item identity, quality and opaque stored properties. The supplied bone policy keeps protection constant while condition is positive. This is an extension of the existing equipment projection paths, not the universal effective-value/contribution engine, attachment system or charged triggers proposed below. The [current maintenance guide](../docs/docs/official_rules.md#bone-armor-wear-and-repair) documents supported gameplay and [recipe implementation](../crates/rusterix/src/server/region_crafting.rs) supplies the operation boundary.

Implementation update (2026-11-18): the first shared passive evaluator now serves authoritative effective attributes, derived dependencies, condition arithmetic, action eligibility, text/HUD attributes and source explanations. Explicit category projections, typed passive enchantment definitions and persistent attachment identities are implemented. The supplied Bone Cuirass of Strength grants +2 STR, shares bone wear/repair and retains its identity through transfer/reconnect/save. Existing structured combat avoids counting a projected source twice. The remainder of this document stays a proposal for advanced stacking/replacements, item/action-local stages, talents, charged triggers, quantization and broader acquisition operations. Gameplay enchanting/disenchanting recipes are not supplied by this passive slice.

## Purpose and proposed decisions

Give attributes, equipment, talents, conditions, and enchantments one consistent contribution contract. Reuse existing attribute identities, semantic roles, formula evaluation, and condition arithmetic. Keep item ratings, effective actor values, and action-local values in explicit scopes so one source cannot be counted twice merely because several systems inspect it.

The [capability gap review](ruleset-foundation-gap-review.md) describes current source behavior. The [foundation concept](ruleset-foundation-concept.md) supplies the gameplay requirements, and the [data model](ruleset-data-model-concept.md) supplies the bone armor chain and illustrative authoring forms. This document specifies the proposed value semantics; it does not claim that the current engine already implements them.

The companion [shared operation contract](shared-operation-contract.md) specifies how contribution-changing mutations, resource costs, and separately committed triggered effects produce consistent outcomes.

The proposed decisions are:

- Saved base values remain separate from calculated effective values.
- Every contribution has an explicit subject, source identity, activation, operation, and lifetime.
- Actor attributes include their applicable equipment contributions. Combat and UI must consume that result without adding the same equipment again.
- An action receives an extra item-local bonus only through a separate declared action contribution.
- Derived formulas read effective dependencies, then receive contributions targeting their own result.
- Reading or explaining a value never equips items, applies effects, spends charges, heals resources, or advances time.
- Gameplay and presentation use the same contract in solo and multiplayer; the authoritative runtime decides which state changes commit.

Under the [RPG editor model](ruleset-editor-concept.md), consistent evaluation, source identity, and side-effect-free inspection are engine guarantees. Authors work with content values and supported policies for activation, stacking, quality, and related gameplay. They do not configure numeric precision or duplicate-source handling as ordinary game options. The worked fixtures remain policy proposals, and the deeper records support explanation and extension rather than the first authoring screen.

## Identity scope and ownership

### Existing attribute names retain their meaning

An attribute identifier is resolved within the effective ruleset. Engine-facing meanings continue to use semantic roles where available. A custom game can call its armor attribute `WARD`; the engine must not require `ARMOR` or a new `physical_protection` constant.

The draft property catalogue can describe type, units, bounds, and presentation for an existing attribute. It must not create a second independently calculated stat with the same gameplay meaning. For the examples below, `ARMOR`, `POWER`, `INT`, and `FIRE_RESIST` are content identifiers. The data-model example's protection and resistance names are corresponding illustrative identifiers to reconcile when adopting a schema.

### Three value scopes

| Scope | Owns | Example |
| --- | --- | --- |
| Item instance | The item's own values and effective ratings | A cuirass provides 4 armor; a knife has condition 56 |
| Actor | Effective attributes of a character or other eligible actor | Saved armor plus worn armor plus conditions |
| Action instance | Values belonging to one invocation and its effects | Physical weapon damage, a spell component, a tool-specific bonus |

An item's `ARMOR = 4` is an item-local rating. A family rule can project that rating into the equipped wearer's `ARMOR`. That projection is one contribution. It does not overwrite either the item rating or the actor's saved armor.

A focus's item-local potency can similarly project into wearer `POWER`, or modify an action performed with that focus. Those are different declarations. Merely finding the same numeric field through both a source-item reference and an equipment list does not authorize both uses.

### Contributions have one owner

| Fact | Owner |
| --- | --- |
| Saved base and declared default | The target attribute or item definition and its saved instance |
| Derived formula and its baseline bounds | The target value definition |
| Equipment projection | Item or equipment-family definition |
| Condition arithmetic and stacking | Condition definition and its active application |
| Talent advantage | Talent definition and the character's selected talent instance |
| Enchantment contributions and triggers | Enchantment definition and its attached instance |
| Action calculation stages and rounding boundaries | The resolved action or referenced calculation policy |
| Health, energy, condition, or charge changes | An authoritative state-changing operation |
| Human-readable explanation | A rendering of the same resolved inputs and calculation result |

Shared defaults may provide declarations, but resolution produces one canonical contribution. Inheriting a family rule and displaying it on an item does not register two copies.

## Required contribution information

The public authoring form may inline some of these fields. The resolved meaning must contain them unambiguously.

| Information | Required meaning |
| --- | --- |
| Definition identity | Which reusable contribution is being applied |
| Source instance | The actual item, attachment, talent selection, or condition application |
| Subject | Item, actor, or action instance receiving the contribution |
| Value identifier | A declared attribute or calculation value in that subject's scope |
| Operation | Addition, multiplication, a bound, or an explicitly enabled baseline replacement |
| Input | Constant or validated reference to another value, with any declared conversion |
| Activation | Explicit state and context requirements, such as equipped, wielded, unbroken, or source item used |
| Stacking | How repeat instances and distinct sources combine |
| Lifetime | What starts, refreshes, replaces, expires, or removes the contribution |
| Evaluation stage | The stage selected by the target's calculation contract |
| Provenance | Definition and instance references sufficient for inspection and removal |

The effective identity is the combination of source instance, attachment or application identity where relevant, contribution definition, subject, and stage. Re-reading a value or rebuilding a view uses the same identity. Two distinct rings bearing the same enchantment have distinct source identities, subject to their authored stacking policy.

Contributions may contain several operations, as existing condition modifiers do. Each operation is accounted once. Missing numeric values follow the target's declared default; invalid or undeclared references are errors rather than silently becoming zero. A string attribute or classification tag does not become a numeric modifier by passing through this contract.

## Evaluation contract

### Snapshot and dependencies

Evaluation uses an identified rules revision, committed world state, and explicit action context if applicable. Due expiry and time-based changes are settled by runtime operations before that snapshot is exposed. An action revalidates against the authoritative state at commitment; a stale UI preview is not authorization to use an old bonus.

Evaluation itself has no side effects. Caches are optional and must not change results. Their validity depends on the actual inputs: saved values, equipped and wielded state, item ratings, condition stacks, attachments, relevant charges, and rules revisions. Animation frames and view count are not inputs to gameplay values.

References form an acyclic dependency graph across scopes. For example, an item's rating may feed wearer armor, which may feed a derived defensive score. The item rating cannot in turn depend on that same defensive score. Source-value calculations and projection edges participate in cycle validation alongside derived formulas.

Initial activation predicates read committed state and explicit action context. A bonus cannot enable itself by testing the value it is currently increasing. More advanced effective-value predicates require explicit dependency semantics and cycle validation before they are supported.

### Baseline and modifier order

For one value and stage:

1. Resolve the saved base, or the declared definition default when the base is absent.
2. If a derived formula exists, evaluate it with effective dependencies. Its reserved `base` input refers directly to this value's saved base, not recursively to its effective result. Apply the formula's declared baseline bounds.
3. Collect applicable contributions targeting this value. Deduplicate exact source identities and apply the declared stacking selection policy.
4. If enabled, resolve a baseline replacement by explicit priority. Replacement changes the stage baseline, not the saved base or the formula's dependencies.
5. Add all applicable additions, including any stack scaling.
6. Apply the product of applicable multipliers.
7. Apply contribution bounds, then explicit hard domain bounds, then the target's declared quantization policy.
8. Return the typed result and its structured explanation.

Within the ordinary numeric stage:

```text
value = (baseline + sum(addition × stacks)) × product(multiplier ^ stacks)
```

No contribution chooses an arbitrary earlier or later point in a combat calculation. The target value's owner defines available stages. A rule wishing to add fire damage defines a fire effect or a contribution to the appropriate action component; it does not attach a stage name that bypasses normal damage policy.

Formula baseline bounds preserve the existing distinction between a formula's bounded result and later modifiers. A formula minimum of zero is not automatically a final floor against every later penalty. A value requiring a final floor must declare that separately. Explicit hard domain bounds protect values such as a bounded percentage; they are not inferred from an official attribute name.

### Stacking replacements and bounds

Distinct additive sources stack by default only where the target's policy allows that combination. Repeated observation of the same source never stacks. A nonstacking group must declare its selection policy, such as an explicit priority; file order, equip order, localized name, and runtime numeric IDs cannot choose the mechanical winner.

Condition stack count remains owned by the condition's stacking policy. An application with two stacks contributes twice its per-stack addition and the square of its multiplier. Refreshing duration does not apply its numeric modifier again to saved state. Replacement removes the old application's contribution and installs the new one as one state transition. This contract does not silently change whether one condition definition permits several applying sources.

Baseline replacements require explicit priority. Highest priority wins; equal winning values coalesce while retaining provenance. Equal-priority different replacements are invalid. Validation must consider fallback combinations as well as the current winner so removing or expiring one source cannot expose a hidden conflict. An invalid candidate equip or effect application is rejected without moving items or charging costs. A getter never resolves a conflict by arbitrary ordering.

Contribution minimums combine by taking the strongest minimum, and maximums by taking the strongest maximum. Apply minimum, then maximum. If they conflict, maximum wins and the trace reports the conflict, following the earlier condition proposal. Declared hard domain bounds apply afterward and take precedence over those modifier bounds. Static validation reports conflicts it can establish ahead of play.

For example, a contribution minimum of 12 and maximum of 9 produce 9 with a conflict explanation. If a property separately declares a hard legal domain of 0–100, an otherwise negative result finishes at zero. Formula bounds, modifier bounds, and hard domain bounds appear separately in inspection.

### Numbers units and rounding

Keep intermediate values fractional unless a named stage explicitly quantizes them. A number-valued attribute retains fractions. For an integer-valued final result, the proposed default is nearest integer with halfway values rounded away from zero. Convert final bounds to achievable integer bounds and clamp the rounded result so rounding cannot escape the legal interval. An interval with no legal integer is invalid.

The shared evaluator must specify numeric precision and use a canonical ordering of stable contribution identities for arithmetic reduction. This prevents declaration order and differing client arithmetic from changing a result. That ordering does not select stacking winners or replacement priority. Non-finite inputs and formula results are reported as invalid rather than silently converted to a usable bonus.

Costs and quantities may need different policies. For example, a fractional cost against an integer pool can round upward. Item counts and liquid volume cannot be combined without a declared conversion. The owning value or operation names the policy; a generic modifier does not round differently depending on which client asks.

Existing damage paths have their own rounding and quality behavior. This contract requires those stages to be named and shared before migration; it does not silently replace their curves with the illustrative factors used in earlier fixtures. Final combat rounding boundaries and default curves still need a separate balance and compatibility decision.

## Worked armor examples

### One projection into effective armor

The first fixture has saved actor `ARMOR = 5`, one equipped item projecting +3, and a guarded condition contributing +2. No multiplier or final cap applies.

| State or operation | Effective armor | Saved armor | Explanation |
| --- | --- | --- | --- |
| Item carried, condition absent | 5 | 5 | No active contribution |
| Equip the item | 8 | 5 | Base 5 + equipment 3 |
| Apply guarded | 10 | 5 | Base 5 + equipment 3 + condition 2 |
| Recalculate or load this state | 10 | 5 | Same sources, counted once |
| Unequip while guarded remains | 7 | 5 | Equipment contribution no longer applicable |
| Expire guarded while equipped | 8 | 5 | Condition contribution removed |

Physical damage reduction reads effective armor 10 once. It does not subtract an additional raw equipment 3. An explicitly authored formula can inspect equipment for another purpose, but an automatic legacy equipment sum is not part of the same projection.

### Complete bone set

In a separate fixture, the data model's five item ratings are helm 1, cuirass 4, greaves 3, gauntlets 1, and boots 1. Each family projection reads the item's effective local rating and contributes it once to wearer armor. With saved armor 5 and guarded +2, the actor has effective armor 17.

The reference bone family declares constant protection while condition is positive, with no quality scaling of armor rating. If the cuirass reaches condition zero and remains equipped, its +4 projection becomes inactive and effective armor becomes 13. Repairing that same cuirass above zero restores 17. If guarded then expires, armor becomes 15. No removal or repair operation subtracts or restores a number in the saved armor field.

Another family may declare a condition or quality curve for its item-local rating. That curve runs before the wearer projection and is not applied again by damage reduction. The curve must be defined by content, not inferred from a family name.

## Worked POWER and dependency example

This fixture uses a deliberately simple derived value:

```text
POWER baseline = saved POWER + floor(max(0, effective INT - 10) / 4)
```

Saved `POWER` is 2 and saved `INT` is 10. The actor wears an item contributing +4 `INT`, equips a focus explicitly projecting +1 wearer `POWER`, and has a condition adding +2 `POWER`.

| Calculation | Result |
| --- | --- |
| Effective INT | 10 + 4 = 14 |
| Derived POWER baseline | 2 + floor((14 - 10) / 4) = 3 |
| Effective POWER | 3 + focus 1 + condition 2 = 6 |

A spell whose declared formula is `rolled amount + effective actor POWER` adds 6. If its roll is 5, its pre-mitigation amount is 11. Its executor does not additionally read the focus's same +1 item field and produce 12. Using that item as the action's source does not duplicate the wearer projection.

An author may separately declare a +3 action-local potency contribution when casting through that exact focus. With a formula that explicitly includes the action-local potency channel, that invocation yields 14 before mitigation. Casting through another source keeps 11. Both contributions are legal because they target distinct declared values; inspection must make that intentional combination visible.

Removing the INT item recalculates the dependency: INT becomes 10, the POWER baseline becomes 2, and effective POWER becomes 5 with the focus and condition still active. Removing the focus then leaves 4. Nothing changes saved INT or saved POWER, and the +4 INT contribution is not also added directly to POWER.

This resolves the current focus ambiguity by requiring a declaration of wearer projection or action-local use. Merely storing `POWER = 1` on an item cannot implicitly mean both. Ordinary healing or other actions benefit only if their own formula references the relevant value.

## Conditions share the same arithmetic

This independent fixture starts with baseline 10, equipment +3, guarded +2, and a weakened condition with two stacks. Each weakened stack adds -1 and multiplies by 0.9.

```text
(10 + 3 + 2 - 1 × 2) × 0.9^2 = 10.53
```

The number-valued result is 10.53 regardless of definition order. A separate active maximum of 9 limits it to 9. Expiring weakened removes only its addition and multiplier, leaving the cap active and the result still 9. Removing that cap exposes 15 from baseline, equipment, and guarded. A target declaring final integer quantization would instead quantize its bounded result at the named final boundary.

The condition's timer, stack count, and source attribution remain condition state. Their changes invalidate affected effective values. Removing guarded does not require remembering an old armor number; the evaluator simply excludes its source from the next snapshot.

## Quality condition activation and wear

Activation predicates are explicit. The initial contract supports carried, equipped, wielded, source item used by this action, owned, positive condition, required charges, and declared combinations. Wielded state is an authoritative gameplay state and does not follow a visible hand animation.

Quality, condition, material traits, and charges are independent values. An item's physical damage curve can depend on quality and condition while its magic remains constant. A passive enchantment can work without charges while a trigger requires them. A family can disable its ordinary armor contribution when broken; an unusual enchantment can use another explicitly declared activation rule.

Wear is a state-changing operation or a consequence of an action outcome. Calculation and inspection cannot wear an item. The owning action contract determines when wear commits relative to a hit and its later triggers. For the ember sequencing proposed below, triggers inspect the committed post-hit state; wear included in the primary hit is already visible. Other sequencing requires an explicit action policy rather than a renderer callback.

Reducing an effective maximum can also require a separate resource transition. For example, unequipping a maximum-health item must follow the resource's declared current-value policy. The proposed ordinary policy clamps current health only when needed after a maximum decrease, and preserves current health on a maximum increase. That clamp commits with the equip, expiry, or breakage transition and is included in its outcome; merely querying maximum health cannot heal or injure an actor. Other resource policies remain possible when explicitly authored.

## Ember passive bonus and charged trigger

The sword fixture uses the foundation's ember attachment: +3 `FIRE_RESIST` while wielded and unbroken, plus a four-point fire effect on an eligible successful hit with that sword. The passive does not require charges; the trigger does. The armor version in the data model activates its passive when equipped instead of wielded.

Saved fire resistance 5 and a separate blessing +2 produce 10 while the sword is eligible. Zero charges leave resistance at 10. Unequipping or breaking the sword leaves 7 while the blessing remains. Removing the blessing while the sword is eligible leaves 8. Recharging changes charges without changing these magnitudes, and repair changes condition without replenishing charges.

The trigger creates a separate fire action component owned by the attachment. Its amount is 4 and its calculation policy applies the target's effective fire resistance once. It inherits neither actor `POWER`, physical weapon quality scaling, nor another source bonus unless its own definition explicitly requests those inputs. Magical damage does not bypass resistance merely because its source is an item attachment.

### Proposed hit and charge sequence

1. Resolve and commit the primary hit, including its declared primary wear and resource changes.
2. Publish an immutable original-hit context with action identity, hit identity, source item, actor, target, and success outcome. A landed hit is distinct from positive final damage.
3. Resolve the attachment's trigger against that context and the committed post-hit state. Require the exact source item, applicable ownership and activation, an unbroken source under this fixture's policy, a valid target, and an available charge.
4. Prepare the secondary fire effect and one-charge decrement together. Revalidate affected state and commit once, or skip with an explicit reason and no charge spent.
5. Deliver outcome notifications for the committed secondary effect. Its damage does not create another original weapon-hit event.

A miss or interrupted primary action emits no successful-hit trigger. A landed primary hit reduced to zero damage can trigger. If the primary hit kills or removes the target, the secondary effect is skipped without charge use. If primary wear breaks the sword, this proposed post-hit activation policy also skips the trigger without charge use; the successful primary hit is retained. This is an explicit sequencing choice for review.

A legally committed secondary effect reduced to zero by resistance still consumes its charge. Concurrent or delayed attempts cannot spend the same last charge twice. The runtime records the handled trigger identity from original hit, target, attachment, and trigger definition so repeated delivery cannot replay it. Multi-target actions must separately declare per-action or per-target frequency and charging; this single-target fixture implies neither policy for them.

With a target at health 20, primary physical damage resolving to 3, fire amount 4 reduced by fire resistance 1, and two starting charges, the target finishes at 14 and the attachment at one charge. An independent counterexample gives the target resistance 6: the fire component resolves to zero, the primary still deals 3, and one charge is still consumed.

## Removal transfer persistence and failure

Contributions are evaluated from currently applicable sources. Equipping registers the relationship and changes state; it does not add a permanent bonus into saved attributes. Unequipping, dropping, transferring, replacing, expiring, breaking, or disenchanting removes or changes the relevant applicability. A transfer removes the former owner's eligibility before the new owner can equip or wield the item.

Permanent learned knowledge and temporary grants use the same source identity principle but retain their different lifetimes. An item grant disappears with the item; knowledge learned independently remains. A change to a numeric modifier does not erase learned skills or acquisition history.

An equip or application operation validates slot and ownership rules, possible contribution conflicts, affected resource policies, and its complete costs before committing. Rejection leaves the previous source configuration intact. Removing or expiring a source must always leave a valid fallback evaluation; latent replacement conflicts must be eliminated through validation rather than discovered by a later UI read.

Save base values, item state, equipment and wield relationships, condition state, attachment identities and charges, and acquisition provenance. Effective totals are reconstructed. A cached total is never the sole saved representation. Loading cannot add bonuses again, replay hits, refill charges, or reset condition duration. Rules and state revisions identify which definitions were used; incompatible changes require an explicit reconciliation or migration decision.

## Explanation contract

Every effective-value result exposes structured data sufficient for a shared renderer to show:

- Subject identity, scope, value identifier, unit, and rules/state revision.
- Saved base or default and the derived formula with effective dependency results.
- Each candidate source, its applicability, lifetime, stack count, and active or suppressed status.
- Selected replacements, additions, multipliers, bounds, and quantization in evaluation order.
- Final value and any validation or conflicting-bound diagnostic.

A committed action adds its input snapshot, source and target identities, component calculations, state changes, and trigger outcomes. Human-facing text is a view over those facts. A tooltip can summarize them while Creator and terminal inspection expand them.

Example outputs:

```text
ARMOR 10 = saved 5 + worn armor 3 + Guarded 2
Physical reduction reads ARMOR 10 once.

POWER 6 = derived baseline 3 + equipped focus 1 + blessing 2
Baseline 3 uses saved POWER 2 and effective INT 14.
INT 14 = saved 10 + intellect item 4.

FIRE_RESIST 10 = saved 5 + ember attachment 3 + blessing 2
Ember charges 0: passive active; on-hit trigger unavailable.

Ember hit: fire 4 - effective target resistance 1 = 3
Target health 17 -> 14; attachment charges 2 -> 1.
```

Source references must identify the actual piece or attachment, including inactive candidates and their reasons. There is no separate hand-maintained UI formula that can show a bonus the runtime does not apply.

## Review acceptance scenarios

| Scenario | Expected outcome |
| --- | --- |
| Armor 5, equipment 3, condition 2 | Effective armor 10; one reduction input, never 13 |
| Full bone set, saved armor 5, guarded 2 | Armor 17; broken cuirass removes 4; valid repair restores 4 |
| Intellect item feeding derived POWER | Dependency is effective once; final POWER follows the worked 6, 5, and 4 results |
| Focus used as action source | Its wearer bonus is not added again through a raw source-item field |
| Separate declared action-local focus bonus | Only the matching invocation receives it, with its own trace entry |
| Condition additions and stacked multipliers | The number-valued fixture yields 10.53 in any declaration order |
| Refresh, expiry, and source replacement | No saved-base mutation and no residual bonus |
| Conflicting baseline replacements | Definition or candidate activation rejected; no arbitrary winner or invalid expiry fallback |
| Modifier bounds versus hard domain | Explicit precedence and diagnostics; no value outside declared hard domain |
| Integer quantization and costs | Declared rounding is consistent across clients and authoritative execution |
| Passive ember with zero charges | Resistance remains; charged trigger is unavailable |
| Valid zero-damage secondary effect | Charge spent once; no health reduction from that component |
| Primary hit breaks source or removes target | Secondary skipped with no charge use under the proposed ordering |
| Repeated hit delivery or save/load | No duplicate effect, charge use, source registration, or reward |
| Maximum increase or decrease | The resource's explicit current-value policy commits once; inspection has no side effect |
| Several views or solo versus multiplayer | Same authoritative calculation and state transition for the same inputs |

These are design expectations for later tests, not test results. Existing headless arenas, condition tests, equipment tests, action descriptions, and structured observation surfaces should be extended when implementation is authorized.

## Relationship to existing behavior and next decisions

| Area | Reuse | Deliberate extension or change |
| --- | --- | --- |
| Attribute identity | Existing attributes, semantic roles, and formula identifiers | Metadata for declared scopes, units, and final domains |
| Formula evaluation | Existing derived dependencies and condition arithmetic | Cross-scope projection graph and shared trace |
| Equipment | Existing items, slot policy, permissions, and item-local ratings | One wearer contribution path replacing equivalent raw equipment sums |
| Item magic | Existing effect application and event infrastructure | Attachment identities, trigger sequencing, charges, and replay handling |
| Runtime and UI | Existing rules resolution, caches, inspectors, and descriptions | One effective-value contract instead of parallel source semantics |
| Persistence | Existing entity and item save data plus condition state | Explicit attachment and grant lifetimes coordinated with stable world identity |

The main ownership recommendation is ready to review: effective actor values include their applicable equipment projections, and action-local contributions require separate declarations. Adopting that direction would require migrating existing combat and presentation consumers together; it is not a request to change them during the GPU rewrite.

Before implementation, settle the default item quality and condition curves, combat rounding boundaries, maximum-resource policies, replacement-conflict validation, and the proposed post-hit trigger sequencing. These are explicit gameplay semantics. They should be resolved through the worked cases before final schema names or code are selected.
