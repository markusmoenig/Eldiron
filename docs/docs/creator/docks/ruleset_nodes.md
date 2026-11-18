---
title: "Internal Ruleset Editor"
sidebar_position: 3
unlisted: true
---

This is a maintainer reference. The editor is hidden from standard Creator builds, including their project tree and menus. Game creators use the supplied RPG content, place world resources and encounters, and choose starting options; ruleset editing is not part of that workflow.

Maintainers can build Creator with `--features internal-rules-editor` to expose **Game / Rules** in the project tree. This explicit build feature is off by default. It preserves the existing editor for maintaining the integrated ruleset; there is no normal Advanced menu exposing it.

The internal editor provides a searchable catalogue and forms for project-wide gameplay definitions. Each project still owns its complete rules document, original and recovery checkpoints, whether or not the editor is visible. Hiding it does not change project files or local/multiplayer runtime rules.

## Find and edit a definition

1. In the project tree, expand **Game** and select **Rules**.
2. Browse the categories on the left, or enter a category or identifier in the search field and press **Return**. Clear the search and press **Return** to show the full catalogue.
3. Select a definition to open its form. Change values and press **Return** to commit text and numeric edits. Checkboxes and choices update when selected. Scroll the form to reach additional fields.
4. Save the project to keep the changes. Undo and Redo are available from the main toolbar.

Forms expose fields suited to the selected definition, including actions, items, classes, races, recipes, conditions, icons, and particle presets. Reference fields can offer choices from the current rules; the accompanying **Custom value** field lets you enter another identifier. Existing values retain their types. Numeric fields must contain a value of the appropriate type, such as `2` for an integer or `2.0` for a number.

The catalogue is shared with ruleset validation and other Creator tools; field descriptions and reference choices come from shared domain metadata. It also includes **resource_profiles**, which define world supplies and interactions, and **resource_pools**, which let several access points share one supply. Bind these definitions to map placements through [Authoring](../authoring#bind-world-resources).

An identifier is a stable reference, separate from the displayed name. All item groups share one item identifier namespace: `items.armor.bone_helm` and `items.tools.bone_helm` would conflict. The definition identifier, the project's item-template UUID and each spawned item's runtime ID serve different purposes; changing a name does not create a new identity.

## Create a bone armor family

1. Open **Game / Rules** and click **Armor Family** in the toolbar.
2. Choose a unique identifier prefix, such as `carved_bone`, and a display name. The supplied `bone` family already exists in current default rules. Choose the plate and binding ingredient identifiers and crafting skill. Defaults are `bone_plate`, `leather_strap` and `armorsmithing`.
3. Choose any classes that should receive the crafting commands and armor permission. Leave the selection empty to create the content without changing classes.
4. Keep **Define missing ingredients** enabled to create missing ingredient definitions, or disable it to require existing ones.
5. Click **Preview**, review the result, then click **Create Family**. Save the project.

The default family creates these five pieces:

| Piece | Slot | Base protection | Bone plates | Leather straps |
| --- | --- | --- | --- | --- |
| Bone Helm | head | 1 | 3 | 2 |
| Bone Cuirass | torso | 4 | 8 | 4 |
| Bone Greaves | legs | 3 | 6 | 3 |
| Bone Gauntlets | hands | 1 | 2 | 2 |
| Bone Boots | feet | 1 | 3 | 2 |
| Full set | | 10 | 22 | 13 |

This adds an armor category, family metadata, five item definitions, five recipes and five crafting actions. These are ordinary editable rules. Protection uses the attribute configured for the ruleset's armor role. Selected classes receive the commands in their main action bar and, where they already restrict armor categories, permission to use this family. Classes without an armor restriction stay unrestricted.

Creation validates the complete candidate before changing the rules or adding project item templates. Identifier collisions or an outdated preview stop creation. Undo removes the family and its newly created templates together; Redo restores the same template UUIDs.

Creating ingredient definitions does **not** supply inventory, world sources or production recipes. Maintainers still need to provide a world origin for every material in a new family. The guide consumes prepared plates and straps; the supplied default bone family separately includes its complete [world production chain](../../official_rules#world-to-bone-armor-production), with tool and station requirements, declared wear and targeted repair. Generated families project their protection through shared effective values and require positive condition. They do not automatically inherit the default family's acquisition, wear, repair or enchantments. The supplied Bone Cuirass of Strength demonstrates [passive bonuses](../../official_rules#passive-equipment-bonuses-and-enchantments); charged effects and advanced contribution policies remain separate work.

## Create, duplicate, and remove definitions

To add a class, race, or another definition, select an existing entry in the desired category and click **New**. This creates an empty `new_definition` beside it. Enter a unique name in **Identifier** and click **Rename**, then add the fields you need. With no selection, **New** creates an action. New definitions are empty; they do not yet contain a ready-made class or race template.

To start with an existing set of fields, select an entry and click **Duplicate**. The copy appears in the same category with a `_copy` suffix. Change its **Identifier**, click **Rename**, and edit the copied values. **Delete** removes the selected definition.

Renaming or deleting a definition does not automatically repair references elsewhere. Update those references and check validation before playing. For example, adding a class definition does not by itself make characters use it: choose that class for a character or configure an appropriate default.

## Fields and lists

Enter a name in **New field**, choose its **Type**, and click **Add Field**. Supported types include text, integer, number, boolean, list, and table. To remove a field, enter its name and click **Remove Field**. A dotted name such as `ui.icon` addresses a field in an existing nested table; create the parent table first if it does not exist.

List entries have individual **Remove**, **Move Up**, and **Move Down** controls. **Add Item** adds an entry; for an empty list, choose an item type first. Lists of records, such as recipe ingredients, expose each item's fields and an **Add Item Field** control. Nested tables and lists also expose their fields and list controls. For example, a class's level unlocks and starting loadout can be edited within its form. Scroll to the relevant entry before changing or reordering it.

## Icons and particles

Icon fields show a preview and an artwork grid containing bundled icons and project tiles. Select a thumbnail to change the icon reference. Under **fx / presets**, particle definitions show an animated preview and editable emitter properties. The preview refreshes after a committed edit.

To edit an item's actual MZ artwork, open its **Appearance** entry in the project
tree rather than replacing its icon reference in a rule form. The workspace
separates Icon On/Off, world models, worn models and materials. Character outfits
come from ruleset equipment chosen in character setup. See
[Appearance](../../characters_items/appearance). Particle effects attached to an
MZ model/rig use the shared emitter system; item behavior still owns state,
light switching and fuel rules.

## Recovery and validation

- **Checkpoint** records the current rules as a recovery snapshot.
- **Restore All** first checkpoints the current rules, then restores the project's original rules document.
- **Recover** swaps the current rules with the latest checkpoint; clicking it again can swap them back.

Save the project to persist the current rules and recovery snapshots. Validation errors are reported in the status bar. Invalid rules can remain saved so you can repair them. A running or paused game keeps its last valid rules when an edit fails validation; a new game cannot start until the errors are fixed. Compatible valid edits refresh running and paused games. Changes to an existing world resource's capacity, outputs or recovery policy require a state migration; changing its display name or initial-stock setting does not reset live stock. Migration tooling is not yet available.

For more about runtime behavior and inspecting the rules, see [Rules In Eldiron](../../rules_in_eldiron).
