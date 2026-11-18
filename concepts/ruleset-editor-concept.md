# RPG ruleset editor and author workflows

Date: 2026-11-14

Status: Product decision updated on 2026-11-17. The generic ruleset editor is internal maintenance tooling, hidden from standard Creator builds. The earlier public Choose/Customize/Extend workflow proposal retained below is superseded and is not an implementation backlog.

## Current product decision

Eldiron supplies one complete default RPG ruleset for small adventures and open sandboxes. Ordinary creators browse the supplied character/item content, build their world, place resources and encounters, and choose player starting options. They should be able to finish a game without opening or understanding a ruleset editor.

There is no per-family activation workflow. Bone armor enters a game through placed items or obtainable ingredients and crafting knowledge. Leaving it out of the world means leaving out acquisition paths, not deleting its definitions. Player-selectable classes remain a starting-options concern, separate from the class definitions used by other characters.

Remove **Game / Rules** and Rules menu commands from standard Creator navigation. Do not introduce an Advanced menu for the same editor. Retain its code and tests, the shared catalogue/domain metadata, and the guided family planner for ruleset maintainers. Explicit builds with `internal-rules-editor` restore the project-tree entry and Rules dock; standard host features never enable it implicitly. Legacy/native navigation commands cannot open the editor in a standard build.

The project's complete rules document, original, checkpoints, stable definition and item identities, template synchronization, world bindings and local/multiplayer resolution remain intact. UI visibility is not a rules migration, content deletion or permission policy.

Current supporting code: [catalogue](../crates/ruleset/src/catalogue.rs), [domain metadata](../crates/ruleset/src/domain_metadata.rs), [family planner](../crates/ruleset/src/equipment_family.rs), [internal family editor](../creator/src/docks/rules_editor/family.rs). See the [maintainer build instructions](../creator/README.md#internal-ruleset-maintenance), [internal editor reference](../docs/docs/creator/docks/ruleset_nodes.md) and normal [resource authoring workflow](../docs/docs/creator/authoring.md#bind-world-resources).

The first complete world-to-bone-armor production and maintenance chain is now supplied, including supporting material/tool sources, declared wear and targeted repair. Further content should follow that tested world acquisition model. The following historical design does not require building additional public configuration screens first.

## Superseded editor proposal and historical review

Everything below records the previous editor proposal and its rationale. It is retained for reference, not as the current normal Creator workflow. Internal maintenance can reuse useful parts when needed.

## Impact on the current Creator implementation

Historical source review date: 2026-11-15. The following observations describe the implementation before the catalogue and family work above; they are retained as the rationale for that work. The application was not launched and tests were not run during that review. Subsequent implementation and verification are summarized above, so the table's Creator-local catalogue and metadata locations are no longer current ownership boundaries.

The active **Game / Rules** route opens the dedicated Rules dock in [project context routing](../creator/src/undo/project_helper.rs#L1544), registered as [RulesEditor](../creator/src/dockmanager.rs#L111). Existing rules-node code remains in the repository, but it is not the normal surface selected by this route. That proposal built on the then-visible catalogue and inspector; the route is now restricted to internal builds.

| Current component | Evidence and reusable behavior | Impact of the proposal |
| --- | --- | --- |
| Document and recovery state | [RulesState and RulesDocument](../crates/ruleset/src/document.rs#L45) preserve the current document, original, checkpoints, and typed values | Retain this shared authoring source; guided choices must not create a second configuration store with competing mechanics |
| Catalogue and search | [catalogue](../creator/src/docks/rules_editor.rs#L480) builds entries from the document's table structure | Add RPG domain and usage views; retain the complete catalogue for deeper editing |
| Property inspector | [form](../creator/src/docks/rules_editor.rs#L544), nested forms, typed scalar parsing, lists, icon choices, and particle previews | Reuse the editing capabilities while adding focused domain forms and progressive detail |
| Domain information | [field ordering and help](../creator/src/docks/rules_editor.rs#L210) and [reference choices](../creator/src/docks/rules_editor.rs#L315) provide an initial layer of RPG knowledge | Replace growing path and key heuristics with a shared description of definition kinds, references, units, supported fields, and editor presentation |
| New duplicate rename and delete | [event handling](../creator/src/docks/rules_editor.rs#L1638) edits the selected table; New starts empty and rename moves a key | Add complete domain templates and dependency-aware change planning; availability changes require separate commands |
| Undo and recovery | [undo and redo](../creator/src/docks/rules_editor.rs#L1136) restore RulesState snapshots; larger button operations capture a before state | Apply a guided family or selection change as one authoring edit and undo step |
| Live-rule publication | Edits call [sync_live_rules](../creator/src/docks/nodes/mod.rs); [prepare_live_rules](../crates/shared/src/runtime_assets.rs) prepares accepted runtime rules (link updated after module extraction) | Keep last-valid behavior; publish a coherent completed guided edit rather than each intermediate definition separately |
| Project item synchronization | [sync_ruleset_items](../crates/shared/src/project.rs#L651) creates or refreshes templates from the resolved item catalogue | Define selection and authoring-browser visibility separately from template existence and persistent instances |
| Help and inspection | [HelpDock](../creator/src/docks/help.rs#L12) refreshes from project rules, and the [help engine](../crates/ruleset/src/help.rs#L10) supplies frontend-independent responses | Reuse linked explanations and queries for review; source coverage, contribution traces, and change impact need additional services |

### Concrete assumptions to change

The catalogue special-cases `items` as grouped definitions and particle presets as another nested group. Item reference choices also flatten the current item groups. Adopting the data-model example's flat item tables unchanged would make those loops treat item fields as entries or reference candidates. A normalized catalogue adapter should identify definitions independently of storage paths; guided authoring does not itself require changing the current grouped representation.

Reference choices are partly inferred from field names such as `skill`, `item`, and `class`. Attribute choices read uppercase-looking keys from `attributes.defaults`. Custom values remain editable, but new resource references, scoped contributions, lower-case custom attributes, and attachment definitions will need deliberate metadata and reference resolution. Storing their nested fields in a generic form is not enough to supply valid choices or explain their runtime meaning.

The current inspector exposes Identifier, Add Field, Remove Field, and low-level nested controls at the front of normal editing. Retain those capabilities in deeper editing while making domain values and valid RPG choices the ordinary surface. New currently creates an empty definition, so a complete armor-family template is new authoring behavior rather than a relabeling of that button.

Rename and delete operate on the selected definition without a reference-impact plan. Existing validation can report the broken result, but the proposed guided workflow must inspect dependents before applying the change. Hiding a class from player creation must not reuse Delete. The current RulesEditor has no dedicated selection or use-specific availability model; that needs defined rules data and corresponding consumers, not only new checkboxes.

The edit path publishes valid rules, while item-template synchronization also runs through project lifecycle code. The proposed design must define when generated or refreshed templates, appearance references, authoring filters, and availability consumers update after a guided change. Removing an item from a player crafting selection cannot be implemented by deleting its definition or allowing synchronization to recreate it with different meaning.

### Scale and rendering implications

Form construction collects bundled and project icon images, and particle previews use image buffers. These presentation paths intersect the ongoing GPU migration and should adopt its shared preview and resource interfaces. The editor redesign should preserve semantic preview inputs rather than introduce another independent CPU display path.

Individual edits clone full RulesState snapshots and edit the serialized document. Catalogue construction walks the document, and forms can rebuild after several edits. These are concrete scaling points to measure with thousands of definitions and realistic undo history. They are not measured bottlenecks in this review, and do not justify replacing the document model without evidence.

The existing rules-editor tests cover field types, nested list edits, save/reload, dropdown choices, icon undo, previews, and toolbar layout. Retain and extend those cases for guided additions, availability versus deletion, dependency changes, and large catalogues. The existing CPU render test will also need the rendering rewrite's validation path. None of those tests was executed for this review.

### Recommended evolution

Keep the document, recovery, validation, and useful inspector behavior. Introduce a shared catalogue and domain-description layer, then build overview and Choose/Customize views that write through coherent authoring commands. Keep deeper definition editing available through the same commands and document.

Add templates, availability semantics, dependency and source queries, change previews, and review results as explicit authoring services. Connect their results to item synchronization, Help, and live-rule publication. This is a substantial evolution of navigation and editing workflow, with significant reusable infrastructure; a second independent ruleset editor and storage model are unnecessary.

## Engine guarantees gameplay policies and content

This classification governs how the technical concepts become authoring features.

| Category | Responsibility | Examples | Editor treatment |
| --- | --- | --- | --- |
| Engine guarantees | Keep supported gameplay coherent and explainable | Stable instance identity; authoritative operations; no duplicate commitment; declared units; consistent effective values; saved state integrity | Always enforced; visible through explanations and diagnostics |
| Selectable gameplay policies | Define supported variations in RPG behavior | Learning, progression, equipment permissions, recovery, wear, repair, paid attempts | Curated defaults, focused settings, and deeper policy editing where supported |
| Content | Populate the RPG world through those capabilities | Classes, talents, professions, materials, creatures, armor pieces, recipes, sources, lessons | Searchable choices, domain forms, templates, and dependency links |

The ruleset defines what an operation costs and how a tree recovers. The runtime preserves the resulting accounting. An author cannot turn off shared-pool consistency or allow a retry to award another item as a shortcut to easier gameplay. Easier gameplay can instead change legitimate yield, costs, requirements, or recovery policy.

Likewise, content-specific numbers in the technical examples are review fixtures. Twelve branches, a ten-point creation budget, or a +3 enchantment are not mandatory engine constants. A declared quantity unit, source relationship, or calculation scope must remain meaningful when an author changes those numbers.

Every field offered as gameplay configuration needs implemented semantics and an explanation. An arbitrary text field cannot add a new capability merely because its name resembles a supported rule. Genuinely new RPG capabilities may still require engine development.

## Proposed Game Rules workspace

### Overview and RPG domains

On opening **Game / Rules**, show a readable overview of the game's selected content, development model, local changes, and review status. A new project offers a small number of curated starting configurations with descriptions of what they select. Existing projects open their actual configuration rather than being forced through setup again.

| Domain | Author questions | Primary controls |
| --- | --- | --- |
| Overview | What kind of RPG have I assembled, and what needs attention? | Selection summary, customization summary, review links |
| Characters and development | Who can the player create, and how do characters develop? | Character paths, attributes and talents, learning, progression, starting grants |
| Combat and actions | What can characters do, and how does conflict work? | Available actions, requirements, costs, conditions, supported calculation policies |
| Items and equipment | What can be obtained and worn? | Equipment families, pieces, permissions, durability, properties, enchantments |
| Materials crafting and professions | Where do goods come from and how are they made? | Sources, recipes, skills, services, lessons, production chains |
| World resources | What can be gathered, and how does the world recover? | Resource profiles, outputs, recovery and lifecycle policies, placement usage |
| Review and testing | Are definitions valid, sources available, and outcomes as intended? | Diagnostics, dependency inspection, isolated scenarios, explanations |

Names and layout are provisional. Search and direct links remain available across domains. A selected definition shows its relevant form, effective values, references, and preview. Related definitions are reachable without requiring the author to navigate the entire catalogue manually.

Large catalogues need search and filters by material, family, skill, source, usage, and customization. Authors should not have to visit hundreds of profession entries just to choose a useful set of trades for their game.

### Choose Customize and Extend

These are depths of work within the same workspace, not separate documents or incompatible editor modes.

- **Choose:** select existing RPG content and supported policies with useful defaults. Show meaningful summaries and required dependencies.
- **Customize:** edit the selected definition through a domain form. Offer valid references, clear units, effective-value previews, and consequences.
- **Extend:** create or duplicate content using an appropriate template, then expose deeper fields and supported behavior composition as needed.

Advanced inspection remains accessible from context. Ordinary selection does not require contribution IDs, transaction plans, internal event payloads, or raw TOML. Extension can use the same public definition format as bundled content, with identical validation and no privileged official-only behavior.

Character path is an author-facing label for the class-backed choices used here, including the unrestricted Human class. Creation availability references those same definitions. Race identity is configured separately.

## Selection availability and world usage

One global checkbox per definition would hide important distinctions.

| State | Meaning | Example |
| --- | --- | --- |
| Present in the project | The definition exists and may be referenced | Ranger is in the catalogue |
| Offered for a particular use | A specific creation, learning, encounter, or crafting route exposes it | Ranger is available in player creation; bone armor is taught by a lesson |
| Used in authored content | Map placements, NPCs, loadouts, or other definitions refer to it | A Ranger NPC guards a road; a tree placement uses a resource profile |
| Existing in saved play | Persistent instances or learned capabilities already use it | A player owns bone boots or knows a recipe |

Choosing only Human for player creation changes that availability list. It does not delete Ranger, change existing NPC classes, erase learned techniques, or disable progression needed elsewhere. Offering a recipe does not make every character know it; acquisition still follows its learning sources and requirements.

Dependency review distinguishes definitions from availability. If bone armor requires a repair operation, selecting its family can propose the required definitions. Making it learnable also needs an authored learning route. Making its ingredients obtainable needs world sources. The editor must not infer all three from one content selection.

A guided selection change shows added references, necessary supporting definitions, and conflicting choices before applying one coherent edit. It reuses definitions already present instead of cloning another copy. Removing availability is reversible; removing a referenced definition requires a separate impact review and explicit reference repair.

Selection never silently populates a map, awards starting goods, edits unrelated NPC behavior, or fills merchant stock. Authors make those changes through their normal world and character workflows.

## Workflow one a basic adventure

The author wants a compact adventure with recognizable classes, modest crafting, and straightforward resource renewal.

1. Start with the integrated defaults and a curated adventure configuration. The overview describes the chosen character paths, progression, equipment, and crafting scope.
2. Offer Warrior, Ranger, and Cleric in character creation. Review their starting attributes, loadouts, and unlocks through readable summaries. Keep other definitions available to NPC authoring where used.
3. Choose the desired equipment families and a small collection of trades and recipes. Show required materials, supporting recipes, tools, and lessons together. The rest of the catalogue remains searchable for later use.
4. Adjust familiar values in focused forms, such as attack cooldown, starting possessions, XP thresholds, or resource yield. Inspect the effective result without editing modifier stages.
5. Select a simple resource policy, such as fixed-interval renewal. Author the actual trees, water sources, creatures, and lessons on the map. The rules editor links to those placements and reports missing acquisition paths.
6. Run isolated scenarios for a starting character, a representative encounter, and one crafting chain. Review the actual selected rules and explanations, then return to map or story authoring.

The outcome is a small playable configuration of the common ruleset. The author has not had to define a numeric type registry, write operation payloads, or configure advanced ecology. Reagents still come from real sources; a small crafting scope does not authorize unexplained vendor materials.

## Workflow two an open sandbox

The author wants open character development, a broad production catalogue, and deeper gathering and recovery.

1. Offer the general Human path in player creation. Choose supported attribute-allocation and talent policies, with strength-oriented and intellect-oriented previews. Human race and Human path remain distinct definitions despite similar labels.
2. Enable the desired practice, teaching, discovery, and other learning routes. Review caps and advancement pace as policies. Keep class or level systems where other authored characters use them; hide them from player creation or player development where appropriate.
3. Select professions and production families. Show the existing materials and skills they share rather than creating a separate material or skill system for every trade. No mandatory global profession-count limit is introduced.
4. Choose resource behavior and customize relevant profiles: finite supply, renewable supply, depletion, optional sustained exploitation, and explicit growth or regrowth where supported. Units and clock meaning remain visible in the form.
5. Bind actual world features through the existing map authoring route. Selected tile or object TOML remains supported, with valid references, field guidance, resolved defaults, and a clear indication of placement versus reusable-asset scope.
6. Review tools, stations, learning opportunities, and source reachability across the intended world. Run representative harvesting, crafting, repair, and enchanting scenarios, including failure and recovery cases.

NPC economic behavior is authored through behavior tools using shared queries and operations. The rules editor supplies reusable content and relevant policy, but does not require an automatic workforce or a player-count economic controller. Solo and online hosting choices belong to their host configuration; both retain the same gameplay accounting guarantees.

The outcome is a deeper use of the same foundation. The author can expose structured character paths later without converting the project to another ruleset engine.

## Workflow three creating an armor family

The author wants to add bone armor using the reference chain, or create a new material family based on that pattern.

1. Choose the equipment-family template. If Bone already exists, edit it or create an explicitly identified variant instead of introducing an accidental duplicate. Choose an existing registered material or open the material template to define a new one.
2. Select the family slots and shared defaults: permissions, protection behavior, durability, repair policy, and applicable properties. The reference Bone family covers head, torso, legs, hands, and feet. Its ordinary equipped protection is presented as a wearer bonus with a readable explanation.
3. Generate or propose the complete declared piece list and recipes as one reviewable draft. Edit per-piece ratings and ingredient quantities in a comparison view. The reference totals of 22 plates, 13 straps, and protection 10 can be shown from the resolved definitions rather than separately maintained text.
4. Follow the dependency view from raw bones to prepared plates, and from hide, bark, and water to leather straps. Select the actual creature sources, processing recipes, tools, deployed stations, and learning routes. A new material needs its own real world origin.
5. Open the existing Appearance workspace for icons, world models, and worn appearances. Required presentation coverage follows the declared slots and supported views; placeholder or fallback use remains visible.
6. Add optional supported item properties or enchantments through forms that explicitly distinguish an item rating, a wearer bonus, and a source-action bonus. A focus's wearer POWER bonus must not be duplicated as an automatic source bonus.
7. Review the proposed definitions, references, appearances, and world acquisition gaps, then apply the guided addition as one undoable authoring edit. Drafts can be saved while incomplete. The editor does not present unfinished content as ready.
8. Test obtaining materials, making a piece, equipping it, breaking or repairing it, and saving and restoring it in an isolated scenario. Inspect the same calculation and operation outcomes used by runtime consumers.

Templates supply reusable structure and supported defaults. They cannot invent the world source, choose final balance, or make an unsupported field executable. The author should reach deeper definition editing only when the normal template and domain controls cannot express the intended RPG content.

## Dependencies explanations and diagnostics

### Different reviews answer different questions

| Review | Question | Result treatment |
| --- | --- | --- |
| Definition and capability validity | Are types, references, supported semantics, and combinations valid? | Structural errors block activation of invalid rules and new play, as applicable |
| Availability and acquisition | Can the exposed content be created, learned, obtained, and maintained in this game? | Missing paths mark the relevant content incomplete, with links to repair them |
| Equipment and presentation coverage | Are declared sets, slots, recipes, and appearances covered? | Report missing members and fallbacks in the affected family |
| Scenario execution | Does this operation produce the expected state changes? | Show committed outcomes and failures from the identified isolated scenario |
| Balance and world supply | Are resource rates, consumption, progression, and material demand appropriate? | Reports for author judgment rather than universal automatic balancing |

An unused recipe in the full catalogue does not force every small map to contain its ingredients. An exposed chain whose only reagent source is an unexplained vendor fails acquisition completeness. That is distinct from a syntactically broken item reference. Authors may playtest a structurally valid but unfinished world, while its review clearly identifies missing content paths.

Scenario fixtures may provide declared starting materials for a focused mechanics test. Those materials belong to the isolated fixture; they are not evidence that the authored world supplies them. Test state must remain separate from ordinary project content and persistent play saves unless an explicit authoring operation is requested.

Show actual outcomes with scope and revisions. A source check is not a runtime test, a passed single recipe scenario is not an economy simulation, and a valid schema is not proof that every recipe is obtainable. UI labels should express those distinctions plainly without requiring authors to understand the internal machinery.

### Explanations stay close to decisions

An armor form can say "Adds 4 armor while equipped and unbroken" and link to its source calculation. A tree can say "12 branches, collect 2 at a time, recover 4 per world day" and distinguish that definition from the selected placement's live stock. A repair preview identifies the existing item, material cost, restored condition, and preserved enchantment charges.

The deeper inspector can expand contribution and operation records, but normal forms summarize the meaningful consequences. All displays use resolved data and shared explanations. Recalculating a preview never changes the world.

## Safe edits recovery and persistence

All authoring depths share validation, undo, checkpoints, and recovery over the same rules document. Guided additions involving several definitions should be one coherent authoring operation with a change summary. This is separate from a runtime gameplay operation that consumes stock or ingredients.

Display-name changes preserve stable identifiers. Identifier changes require a reference-aware change plan, including unresolved script or saved-state references where automatic repair cannot be established. Deleting a class used by NPCs is different from removing it from player creation. Deleting a source profile used by map placements requires resolving those usages.

Distinguish the project's current definition from its original snapshot and any known default. Project-owned rules remain complete documents today. Comparing with originals does not imply live upstream inheritance, automatic package updates, or minimal override generation already exist.

Running play should retain the last valid rules when a draft is invalid, consistent with the current authoring contract. Structural validity alone does not authorize resetting live resources or migrating existing attachments. Edits affecting persistent state need the reconciliation required by the resource, contribution, and operation concepts.

Curated starting configurations propose changes rather than erase custom definitions or overwrite map, character, palette, and appearance work. Returning to an earlier configuration is not a world reset. Review saved instances separately before destructive or incompatible changes.

## Consolidated concept responsibilities

| Document | Responsibility after consolidation |
| --- | --- |
| [Integrated foundation](ruleset-foundation-concept.md) | RPG product scope, integrated default world, sourcing, and shared ownership principles |
| [Data model and bone chain](ruleset-data-model-concept.md) | Illustrative definitions, references, complete production content, and material accounting |
| [Capability gap review](ruleset-foundation-gap-review.md) | Dated evidence about reuse and missing runtime/editor contracts |
| [Contribution contract](ruleset-contribution-contract.md) | Item, actor, and action values; source activation, calculation, removal, and explanations |
| [World resources](world-resource-lifecycle-contract.md) | Resource binding, identity, stock, recovery, lifecycle, and access |
| [Shared operations](shared-operation-contract.md) | Preparation, validation, commitment, failure, retries, and structured outcomes |
| This editor concept | Author-facing choices, guided extension, domain navigation, and review workflows |

Technical contracts enforce their guarantees regardless of authoring depth. Their example policies remain reviewable rather than fixed universal defaults. This concept does not settle item damage curves, trigger ordering alternatives, offline world-time behavior, or reservation economics merely by describing where they might be edited.

## Review acceptance and remaining choices

- A basic-adventure author can choose paths and content and adjust common values without editing internal transaction or contribution records.
- A sandbox author can expose open learning and deeper resources using the same definitions and editor.
- Hiding Ranger from creation preserves Ranger NPCs and saved state.
- Choosing bone armor reveals shared dependencies without duplicating existing material definitions or creating free world supplies.
- Guided extension covers every declared armor piece, learning route, repair chain, and required appearance.
- Advanced changes use the same runtime semantics and validation as ordinary forms.
- Engine consistency, source accounting, and persistence are not optional gameplay toggles.
- Adaptive NPC economy remains authored gameplay, and unrelated genres remain outside product scope.

Next review should choose the first overview fields, curated starting configurations, availability controls, and equipment-family template fields. Walk the three workflows against representative projects, resolve remaining semantic policy decisions, and only then settle the public schema and implementation sequence. The ongoing GPU rewrite remains outside these documentation changes.
