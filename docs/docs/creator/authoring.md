---
title: "Authoring"
sidebar_position: 3
---

<div style={{position: 'relative', paddingBottom: '56.25%', height: 0, overflow: 'hidden', marginBottom: '1rem'}}>
  <iframe
    src="https://www.youtube.com/embed/YLHWBTcqfps"
    title="Eldiron Authoring"
    style={{position: 'absolute', top: 0, left: 0, width: '100%', height: '100%'}}
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerPolicy="strict-origin-when-cross-origin"
    allowFullScreen>
  </iframe>
</div>

Authoring in **Eldiron Creator** adds narrative and descriptive metadata to the world. It also binds map placements to ruleset-defined resources such as trees and wells. Descriptions serve text, 2D and 3D play; resource supplies use the same authoritative rules in solo and online play.

## Authoring Dock

The tool strip contains an **Authoring** toggle.

When it is enabled, contexts that would normally show the **Tiles** dock show the **Authoring** dock instead.

This makes Authoring a persistent mode:

- geometry and selection tools still work normally
- tile-backed contexts switch from **Tiles** to **Authoring**
- clicking Authoring opens its dock from any tool context; the sidebar **Console** remains independently available

## What You Can Edit

The Authoring dock edits TOML metadata for:

- selected sectors
- selected linedefs
- selected 3D Geometry Objects
- selected linked Prefab instances
- placed characters and items, selected on the map or in the region tree
- selected character templates
- selected item templates

Important:

- sectors, linedefs, and Geometry Objects are authored from the current region selection
- linked Prefab instances edit their own authoring; blank descriptions inherit the shared asset description
- placed characters and items keep their own authoring, without requiring an extra global template
- character and item templates supply reusable defaults; local descriptions override those defaults
- character and item configuration belongs in **Entity Nodes**, and gameplay logic in **Behavior Nodes**

So the split is:

- `Authoring`: descriptive and presentation text, plus resource bindings on sectors, linedefs and Geometry Objects
- `Entity Nodes` and `Behavior Nodes`: configuration, input mappings, rules-based actions, and gameplay logic

## Minimal Format

The current starter template is:

```toml
title = ""
description = """
"""
```

This is shown automatically for empty selections so the expected format is always visible.

Use:

- `title` for the player-facing name of the place, connection, entity, or item
- `description` for the longer descriptive text

`title` is optional for `look`. If only `description` is present, that still works.

Examples:

```toml
title = "Your ship"
description = """
The familiar deck of your faithful ship creaks softly beneath your feet. It has carried you through many adventures, and still feels more like home than any harbor.
"""
```

```toml
title = "Crossroads"
description = """
A small crossroads of worn earth and scattered stones, marking the meeting point between harbor, home, and garden.
"""
```

## How It Is Used

Right now the authoring metadata is already used by:

- text-style terminal room titles and descriptions
- text-style exit and room presentation
- authored sector description messages in regular gameplay
- `look` for characters and items in 2D, 3D, and text play when no behavior response supplies a description

## Place a Boneworking lesson

Place the supplied **Boneworking Manual** where a character can obtain it.
Studying the owned book with **Study Boneworking Manual** requires Boneworking
10 and permanently teaches the batch bone-plate recipe. Passing the book to
another character does not remove the first character's knowledge. Books supply
knowledge; the recipe still needs world-sourced materials, tools and a station.

For a teacher, configure a character's attributes through **Entity Nodes** with
`profession = "Boneworker"`. An explicit `teaching = ["advanced_boneworking"]`
attribute can offer that lesson independently of profession. These are entity
settings, rather than descriptive Authoring metadata.

The learner can use **Learn Advanced Boneworking** on that nearby NPC. To teach
as part of a conversation, connect the chosen dialogue branch to **Teach** and
select `learn_boneworking`. The event's subject is the learner; the graph's NPC
is the teacher. Use **Done** and **Failed** for appropriate dialogue responses.
The shared action checks the offered lesson, range, learner skill and costs
again when invoked. Hiding a dialogue option alone does not enforce a lesson.

New Rangers already know this lesson and have Boneworking 10. Other characters
can learn through either source regardless of class or profession. Existing
basic bone recipes require no lesson.

## Place Bear inscription

Place the supplied **Patterns of the Bear** manual where it can be obtained,
or configure a character with `profession = "Inscriber"`. A Teach node can
choose `learn_bear_inscription`; the manual uses **Study Patterns of the Bear**.
Both teach the same permanent knowledge at Boneworking 10. No ruleset editor or
content activation setting is required.

Provide accessible skeleton remains, trees for resin, river or well access,
and the established sources for a knife, bowl, workbench and ordinary bone
cuirass. The learned **Carve Bear Sigil Inlay** action creates the inlay from
bone plate, resin and water. In this example, **Inscribe the Bear** targets one
owned bone cuirass, consuming the inlay, another resin and water. Both use an owned carving
tool and a nearby deployed bone workbench. Repair a broken target first.

The result is **Bone Cuirass of the Bear**, with +2 STR while equipped and usable.
Its identity, quality and condition are retained. The pre-enchanted supplied
variant can also be placed directly. Item artwork remains editable native MZ;
the inlay shows a carved bear paw and the manual uses parchment and leather.
Other compatible equipment can take the same inscription. Distinct compatible
inscriptions can coexist on one piece. See
[Fox inscription and compatible enchantments](../official_rules#fox-inscription-and-compatible-enchantments).

## Expose the crafting journal

The default game toolbar includes **Crafting & Knowledge**. Custom screen buttons
and character input bindings can use `ui.journal`; existing Actions and Spellbook
panels also provide a **Journal** button. No ruleset editor or recipe activation
setting is required.

The journal discovers open and currently known recipes from the project's
supplied rules. Place the world resources, teachers, manuals, tools and stations
that make those recipes usable in your game. Players can search by recipe,
ingredient or craft, inspect current requirements, choose an exact owned piece
for repair or inscription, and pin recipes to their own action bar. Adding
supplied recipes does not require copying every recipe into every class's
starting shortcuts.

The Knowledge tab explains permanent acquisitions and active equipment grants.
The same screen works with solo and online character snapshots; previews are
read-only and every craft is validated by the host when performed.

### See crafting and teaching in Wayfarer’s Rest

Open [Wayfarer’s Rest](wayfarers_rest) and start at **09:30**. In Mara’s
tannery, the **Tannery Water Cask** supplies water. The placed **Tanning
Frame** and leather workbench satisfy station requirements. During her
working hours, Mara tans hide, twists cord and makes leather boots and hoods.
Her **Talk** branch offers the finished inventory for sale. The demo gives her
a finite starter batch.

Press `J` to open the journal and inspect the requirements for a leather
recipe. Mara’s **Use** branch offers **Advanced Leatherworking** through a
**Teach** node using `learn_advanced_leatherworking`. The shared action checks
the learner’s Leatherworking 10 requirement and saves the lesson to that
character. See the [leather armor rules](../official_rules#leather-armor-learning-and-repair)
for the full production, learning and repair chain.

## Bind world resources

A tree image does not automatically provide wood. A placement's `[resource]` binding names a profile from the project's supplied ruleset. The profile owns the supplies, registered output items, interaction requirements and recovery policy; the placement can override only the settings that profile exposes. Choose an existing profile in Authoring; opening a ruleset editor is not required.

1. Select a sector, linedef or 3D Geometry Object and open **Authoring**.
2. Choose a resource profile in the profile selector.
3. Choose **Independent** for a separate source, or choose a compatible shared pool.
4. Click **Bind Resource**. Review the resulting TOML and save the project.

The binding preserves other authoring values and supports Undo/Redo. If `[resource]` already exists, edit it directly; Bind Resource will not overwrite it. Resource binding controls currently apply to sectors, linedefs and Geometry Objects. Reusable Prefab and paint-asset resource inheritance remains planned.

### Bundled profiles

| Profile | Default interaction | Supply and recovery |
| --- | --- | --- |
| `mature_tree` | `collect_branches` | 12 branches; recovers 4 per 86,400,000 simulation milliseconds. Also holds 4 bark, 2 resin and one felling opportunity yielding 6 logs. Bark, resin and felling do not recover. |
| `river` | `fill_water` | Unlimited fresh water; fills up to 4 volume units per interaction. |
| `finite_well` | `fill_water` | Capacity 12, initially 3 water units; recovers 3 per 43,200,000 simulation milliseconds. Fills up to 4 units per interaction. |
| `skeleton_remains` | `collect_bones` | One extraction yielding 6 raw bones; no recovery. Becomes spent when emptied and expires after 172,800,000 simulation milliseconds. |
| `loose_flint` | `collect_flint` | 4 flint stones, collected one at a time by hand; finite with no recovery. |
| `fallen_timber` | `collect_branches` | 20 branches, collected one at a time by hand; finite with no recovery. |
| `wild_flax` | `collect_fiber` | 16 fiber, collected in whole batches of 2 by hand; finite with no recovery. |
| `deer_remains` | `collect_hide` | One raw hide; requires a skinning tool and costs 1 condition. No recovery; expires after 172,800,000 simulation milliseconds. |

All bundled interactions require the actor to be within 2 world-space units. **Use** on a bound map placement invokes the profile's default interaction, or the named `interaction` selected in that access point's binding. Named requests can also select an interaction directly. There is not yet a dedicated menu exposing every interaction.

Branch collection takes a whole batch of 2 and requires an owned tool with the `prune` capability, such as `pruning_knife`. Successful collection costs 1 tool condition and grants woodworking practice subject to skill caps. Bark and resin collection also require `prune` and cost 1 condition. Felling requires `fell_tree`, supplied by `woodcutting_axe`, costs 1 condition, changes the source to `stump` and discards its remaining supplies. Automatic regrowth is not implemented.

Water fills an existing owned vessel, such as `wooden_bowl` with capacity 4. The amount is limited by available water and free vessel capacity. Full vessels and incompatible liquids cannot be filled. The vessel keeps its identity and other attributes; water does not become a loose inventory stack. Tools and vessels must be in top-level inventory or equipment. Their definitions do not automatically give them to characters; configure starting items or place obtainable tools in the world.

### Independent tree

Add this to the selected placement's Authoring TOML:

```toml
title = "Old orchard tree"
description = "A broad tree with low, reachable branches."

[resource]
profile = "mature_tree"

[resource.supplies.branches]
initial = 8
capacity = 12
recovery_amount = 4
recovery_interval_ms = 86400000
```

The tree profile exposes those four branch settings. It does not permit a placement to substitute outputs or invent an item. Output identifiers must exist in the ruleset. For finite supplies, omitted `initial` means full capacity; unlimited supplies omit capacity and initial stock entirely.

To make normal **Use** collect bark at a tree access point, use:

```toml
[resource]
profile = "mature_tree"
interaction = "collect_bark"
```

The interaction must already exist in the profile. Different access parts can
select different interactions while sharing the same explicit logical `instance`
UUID; selecting an interaction does not change their shared supply policy.

### River and shared well

Bind river access points to the bundled `river` pool:

```toml
[resource]
profile = "river"
pool = "river"
```

Bind each access point for the village well to the same bundled pool:

```toml
[resource]
profile = "finite_well"
pool = "village_well"
```

A pool shares stock across access points, including points in different regions of the same world. Pool access bindings cannot override supplies or specify `instance`; the supplied pool definition owns those settings. For a well with its own independent stock and exposed local settings, omit `pool`.

Independent source identity comes from the map and placement UUIDs, not the tile artwork. Different trees using one image have different stock. An optional, explicit non-nil `instance` UUID groups several parts into one logical source; all parts must resolve to the same policy. Conflicting owners are rejected. Copying a placement without an explicit instance creates an independent source; copying an explicit instance intentionally shares it.

### Skeleton remains

Opt a character into remains extraction by setting the character's mechanical `remains_profile` attribute to `skeleton_remains`. The initialization record is equivalent to:

```toml
[attributes]
remains_profile = "skeleton_remains"
```

Set this through character configuration, separately from descriptive Authoring TOML. Death creates one logical remains source for that incarnation, reusing the loot corpse when present. Carried loot stays ordinary loot; the 6 potential bones belong to the resource supply and are not granted again as a random reward. Duplicate death notifications do not create another source. Expiry prevents further extraction without removing bones already collected.

For a deer, use [Add Animal](../characters_items/entity_nodes#configuration-nodes)
and choose the species. Its ruleset entry supplies the shared artwork and
`deer_remains` binding. Use `remains_profile` directly when authoring a
custom character that needs an explicit remains profile.

### State, time and appearance

Stock, phase, deadlines and source identity survive world save/reload.
Recovery and remains expiry use running simulation milliseconds, independent
of player count, rendering and the in-game clock. Moving the time slider
changes **Time Range** schedules independently. For example, the bundled
well’s 43,200,000 ms interval requires 12 hours of accumulated simulation
time. Pausing or closing the world does not accumulate that time.

When a full supply is depleted, its recovery interval starts. Further extraction does not restart that interval. Recovery stops at capacity and does not bank extra stock. Optional exploitation policies can suspend recovery; the bundled tree does not enable one by default.

Actors contend for one authoritative supply in both local and hosted games. Extraction commits outputs, stock, tool wear and skill gain together. A failed action grants no partial output and charges no wear or practice; elapsed recovery or expiry can still settle. Existing source state is not replenished by inspection, reconnecting or editing the initial-stock value. Capacity, output and recovery-policy changes require migration; migration tooling remains planned.

Optional `[resource.visual_tiles]` entries map appearances such as `healthy`, `sparse`, `bare` or `stump` to actual project tile UUIDs. Include a `healthy` fallback whenever supplying a visual map. Clients also receive semantic phase and supply information. These mappings change authored tile appearances; mesh and collision replacement, planting and growth are not yet implemented.

The supplied [bone armor production chain](../official_rules#world-to-bone-armor-production)
connects these sources to tools, stations, liquid-consuming processing recipes
and all five armor pieces. Place at least one flint patch, one fallen timber
source, one flax patch, two bark-bearing trees, river access, and enough
hide/bone sources for the desired production. Four deer and four skeleton
remains cover the worked full-set example. The default Actions catalogue
provides crafting, [bone armor repair](../official_rules#bone-armor-wear-and-repair)
and [inscriptions](../official_rules#fox-inscription-and-compatible-enchantments).
NPC participation and economic balancing are authored gameplay policies.

## Author a placed door or character

1. Enable the **Authoring** toggle in the tool strip.
2. Use **Object** mode for a placed item, or the **Entity** tool for a character or item on the map,
   or select its entry beneath the current region in the project tree.
3. Edit the displayed TOML in the **Authoring** dock. The dock title identifies
   the selected placement. You can switch tools while keeping Authoring enabled.
4. Play, choose **Look**, and click the object. Its description appears above it.

The authoring is stored with that placement, saved in the project and copied when
that placement is duplicated. It does not add a door to the global Items list or
change the MZ recipe. Undo/Redo applies separately to each authored target.
Blank local descriptions inherit template defaults. A placement can override one
state and leave other states inherited. The template remains useful for shared
content, but a new template is not required just to describe a particular door.

```toml
title = "Inn Door"
description = "Weathered oak planks bound with iron bands."

[state.off]
description = "The oak door is shut. Its iron handle is worn smooth."

[state.on]
description = "The door stands open, revealing the quiet inn beyond."
```

World objects do **not** reveal descriptions or names on hover. Inventory,
equipment and toolbar UI retain their tooltips. See [Look and World Text](../characters_items/world_text)
for reading, dismissing and paging descriptions in 2D, iso and 3D.

## Character And Item Authoring

Character and item templates can define descriptive fallbacks used by `look`.

Characters support mode-based overrides:

```toml
title = "Guard"
description = """
A weary guard watches the road.
"""

[mode.active]
description = """
A weary guard watches the road.
"""

[mode.dead]
description = """
The guard lies motionless on the ground.
"""
```

Items support state-based overrides:

```toml
title = "Torch"
description = """
A simple wall torch.
"""

[state.off]
description = """
An unlit torch is fixed to the wall.
"""
on_use = "You light the torch."

[state.on]
description = """
A lit torch flickers warmly against the stone wall.
"""
on_use = "You extinguish the torch."
```

Resolution order is:

1. matching `mode.<value>` for characters
2. matching `state.<value>` for items
3. fallback to the top-level `description`

So `mode.*` and `state.*` are optional overrides, not required fields.

For items, `state.*` can also carry simple use text like `on_use`.
That authored `on_use` message is used as a fallback in text, 2D, and 3D play when no explicit item `on_use` behavior overrides it.

For sectors, you can also add:

```toml
show_in_2d = false
show_in_3d = false
```

to suppress automatic sector description messages per view mode.

## Global Authoring Settings

Global authoring and text-presentation behavior lives under **Game / Authoring**.

That configuration controls things like:

- startup welcome text
- startup room/description behavior
- exit presentation style
- terminal colors
- auto-attack behavior for text-style clients
- sector description message policy

See [Authoring Configuration](../configuration/authoring) for the full reference.

## Optional AI Text Input

The terminal client can use the same authored rooms, exits and intents with optional
AI interpretation. See [AI for Text Play](../clients/text_ai.md) for account setup,
provider options and cancellation. No extra authoring mode is needed.
