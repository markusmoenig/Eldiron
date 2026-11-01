---
title: "Ruleset Editor"
sidebar_position: 3
---

The ruleset editor lets you change project-wide gameplay data in **Game / Rules**. It provides a searchable catalogue and forms; editing rules does not require connecting nodes or writing TOML. Each project owns its complete rules document, so your edits travel with the project.

## Find and edit a definition

1. In the project tree, expand **Game** and select **Rules**.
2. Browse the categories on the left, or enter a category or identifier in the search field and press **Return**. Clear the search and press **Return** to show the full catalogue.
3. Select a definition to open its form. Change values and press **Return** to commit text and numeric edits. Checkboxes and choices update when selected. Scroll the form to reach additional fields.
4. Save the project to keep the changes. Undo and Redo are available from the main toolbar.

Forms expose fields suited to the selected definition, including actions, items, classes, races, recipes, conditions, icons, and particle presets. Reference fields can offer choices from the current rules; the accompanying **Custom value** field lets you enter another identifier. Existing values retain their types. Numeric fields must contain a value of the appropriate type, such as `2` for an integer or `2.0` for a number.

## Create, duplicate, and remove definitions

To add a class, race, or another definition, select an existing entry in the desired category and click **New**. This creates an empty `new_definition` beside it. Enter a unique name in **Identifier** and click **Rename**, then add the fields you need. With no selection, **New** creates an action. New definitions are empty; they do not yet contain a ready-made class or race template.

To start with an existing set of fields, select an entry and click **Duplicate**. The copy appears in the same category with a `_copy` suffix. Change its **Identifier**, click **Rename**, and edit the copied values. **Delete** removes the selected definition.

Renaming or deleting a definition does not automatically repair references elsewhere. Update those references and check validation before playing. For example, adding a class definition does not by itself make characters use it: choose that class for a character or configure an appropriate default.

## Fields and lists

Enter a name in **New field**, choose its **Type**, and click **Add Field**. Supported types include text, integer, number, boolean, list, and table. To remove a field, enter its name and click **Remove Field**. A dotted name such as `ui.icon` addresses a field in an existing nested table; create the parent table first if it does not exist.

List entries have individual **Remove**, **Move Up**, and **Move Down** controls. **Add Item** adds an entry; for an empty list, choose an item type first. Lists of records, such as recipe ingredients, expose each item's fields and an **Add Item Field** control. Nested tables and lists also expose their fields and list controls. For example, a class's level unlocks and starting loadout can be edited within its form. Scroll to the relevant entry before changing or reordering it.

## Icons and particles

Icon fields show a preview and an artwork grid containing bundled icons and project tiles. Select a thumbnail to change the icon reference. Under **fx / presets**, particle definitions show an animated preview and editable emitter properties. The preview refreshes after a committed edit.

## Recovery and validation

- **Checkpoint** records the current rules as a recovery snapshot.
- **Restore All** first checkpoints the current rules, then restores the project's original rules document.
- **Recover** swaps the current rules with the latest checkpoint; clicking it again can swap them back.

Save the project to persist the current rules and recovery snapshots. Validation errors are reported in the status bar. Invalid rules can remain saved so you can repair them. A running or paused game keeps its last valid rules when an edit fails validation; a new game cannot start until the errors are fixed. Valid edits refresh running and paused games.

For more about runtime behavior and inspecting the rules, see [Rules In Eldiron](../../rules_in_eldiron).
