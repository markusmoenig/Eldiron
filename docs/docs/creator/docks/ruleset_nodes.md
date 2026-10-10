---
title: "Ruleset Nodes"
sidebar_position: 3
---

Select **Game / Rules** in the Project tree to edit your game's ruleset. All
ruleset authoring uses nodes; you do not need to edit TOML. Each project owns its
complete ruleset and keeps an original copy for restoration.

## Find And Change A Definition

The left branch list groups definitions by path. Filter for a name such as
`training_sword`, `Warrior`, or `hit_burst`, then select its branch. Only that
branch appears on the canvas, keeping the full catalogue out of the way.

Definition, Table, and List nodes contain editable field rows. Change a value
inline and press Return to commit text. Field types distinguish text, integer,
number, boolean, table, and list values. A formula is a text value in the field
that expects it; changing a field's name or type also changes the rules data.

Each key has its own output terminal. Connect a Table or List field to its
matching child node. A scalar field uses its inline value unless connected to a
matching **Set Attribute** node. Connections assemble data; they do not execute
behavior. List indices start at zero and must remain consecutive.

Use the sidebar's **Node List** to drag in new nodes. Use Undo/Redo for edits and
**Tidy** to arrange the selected branch. Renaming a field preserves its wire;
deleting it disconnects its child, which must be removed or connected elsewhere.

## Add Or Remove Game Content

A **Definition** root gives a branch its unique path, for example
`/actions/repair`, `/races/Robot`, or `/classes/Engineer`. Add its typed fields
and connected tables or lists to describe the definition. Copying an existing
branch is useful when building a similar definition; give the copy a unique path.

**Disable** omits a branch without deleting its nodes. **Remove** deletes the
branch. Removing a race, class, profession, item, action, or other referenced
definition requires repairing its references too. Validation identifies invalid
definitions and references at the top of the canvas.

For a different kind of game, **Start Empty** checkpoints the current rules and
removes all definitions. Add the policies and content your game needs, including
attribute defaults and semantic roles. Races, classes, professions, and leveling
are optional. Generic definitions can hold custom data, but new data alone does
not implement an engine mechanic. See [Rules In Eldiron](../../rules_in_eldiron)
for the fields the runtime understands.

## Replace Icons

Icon fields display their artwork. Click one to open the tile picker and choose
a project tile. **Inherit** clears an optional override. Icon catalogue branches
also have a clickable preview; replacing that texture changes the artwork shared
by references to that icon. Explicit entity artwork still takes precedence over
ruleset defaults.

## Edit Particle Effects

Select a branch under `fx / presets`. Its **Particle FX Preset** root sets the
preset path, description, duration, and size multiplier, and shows an animated
emitter preview. Connect a single chain of particle modules:

| Module | Controls |
| --- | --- |
| Emission | Rate and spread |
| Motion | Speed range and turbulence |
| Lifetime | Particle lifetime range |
| Size | Particle radius range |
| Color | Four color-ramp stops and variation |
| Direction | Direction and gravity |
| Spawn Area | Point, Box, or Surface shape and spawn dimensions |
| Lifetime Curves | Four size and opacity points over particle lifetime |

Color fields show swatches and accept `#RRGGBB` or `#RRGGBBAA`. Omitted or disabled
modules use emitter defaults. These modules are shared with prefab particle
authoring. Actions, spells, and conditions reference presets by name; update
those references when renaming or removing a preset. Stage overrides may further
change color, density, duration, or size in play.

## Test Changes While Playing

Valid committed edits refresh running and paused Creator games at the next
region update, and refresh interactive **Help** against the accepted ruleset.
Current characters keep their health, inventory, position, and quest state.
Changed spawn defaults apply to new entities; restart play to test initial setup.

Invalid drafts stay editable and can be saved. An already-running game keeps its
last valid rules until you repair the draft. Starting a game from invalid rules
is blocked, with startup errors in the log. Check validation after removing or
renaming content, then test the affected action or spawn in your game.

## Restore Rules

Before a large change, use **Checkpoint** and save the project.

| Control | Result |
| --- | --- |
| Undo/Redo | Reverses or reapplies ordinary edits |
| Checkpoint | Saves the current rules as a recovery snapshot |
| Restore Branch | Restores the selected definition from the preserved original |
| Restore All | Restores the entire original, including deleted definitions; first checkpoints the discarded draft |
| Recover | Swaps the current rules with the latest checkpoint |
| Start Empty | Checkpoints the current rules and removes all definitions |

The original and checkpoints travel with the saved project. **Restore All**
restores that project's original even when a later Eldiron release changes the
bundled rules. Projects currently own full graphs: automatic upgrades, inherited
node deltas, and conflict resolution are not implemented. Changing the ruleset
selection in Game Settings does not replace the owned graph.
