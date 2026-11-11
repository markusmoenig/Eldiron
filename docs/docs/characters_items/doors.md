---
title: "Doors and MZ Item States"
sidebar_position: 12
---

A door uses the normal **Item** intent and state system. A placed door can have
its own authoring without requiring an extra entry in the global Items list. One editable MZ recipe
defines both its Closed and Open presentations. The item's **On/Off** state
selects the presentation in both **2D Iso and 3D**. Each placed item has its own
state; opening one copy never changes every door using that recipe.

## Try the inn door

Open `test_projects/UnendingNight.eldiron` and play. Approach the inn, select
**Open/Close** on the bottom toolbar, and click the door leaf. Click again with
that intent selected to close it. **Use** and **Look** remain separate buttons.
The door starts closed and unlocked. Its oak boards, iron fittings and ring
handle are actual MZ geometry, illuminated by Eldiron's sun and local lights.

The toolbar selects `intent.open_close`. The server resolves that request into
the explicit `open` or `close` intent from the target's current state **before**
checking the official intent's range and permission rules. Hovering the item
also selects the corresponding Open or Close artwork for the toolbar button.
`intent.open` is idempotent: opening an open door never closes it.

## Edit the door

1. Select the placed door with the **Entity** tool. Its item configuration is
   edited through **Entity Nodes**; its narrative content is edited in the
   **Authoring** dock.
2. The **Item State** node exposes **On / Open**, **Openable**, **Locked** and
   **Required Key**. For this recipe, Off means Closed and On means Open.
3. Open the item's **MZ Geometry** editor to edit its construction. The existing
   On/Off appearance preview uses the same recipe for both states.
4. The **Closed** row assembles the boards and fittings and attaches the
   footprint. The **Open** row references Closed and rotates it about the
   hinge at the recipe origin. The final **State Switch (Off / On)** chooses
   between those two rows.

The inn door's Closed pose is seated inside the doorway, at Z = 9.5. Its Open
pose swings outward by **170°** (Y rotation **−170°**) and includes **0.21** of
Z translation for jamb clearance, keeping the leaf folded along the exterior
wall. These are ordinary editable operations in the recipe's **Open → Transform**.
The collision footprint follows that same transform in iso and 3D. Moving the
whole placed item moves both poses; edit the pose transform to change only Open.

To edit what the player discovers, enable **Authoring**, select the placed door,
and edit its `description`, `[state.off]` and `[state.on]` descriptions. UnendingNight
includes examples on the inn door itself. Choose **Look** and click the door to
read them **above the door**. There is no automatic world hover tooltip or test
wording about On/Off. See [Look and World Text](world_text).

State Switch is available in the normal MZ operation catalog. Its two row
selectors are **Off / Closed** and **On / Open**. These describe the door
example: other items can use the same switch for unlit/lit, lowered/raised or
another pair of states. To inspect a branch's construction, select its row and
use the editor's selected-row preview.

Both outputs remain in the recipe. There are no separate Open and Closed mesh
assets to keep in sync. Material Texture operations reference ordinary editable
MZ materials. Geometry and footprint transforms travel together, so editing the
hinge rotation changes the displayed leaf and its collision shape together.

Stateful recipes are not automatically added as static Rect tiles. Place their
**Items** instead, so their identity, collision and behavior remain attached.

## Interior cutaway

The inn door uses the existing named **Inn** interior for its cutaway. When the
player enters that room, the door leaf disappears together with the near walls;
it reappears on leaving. Its closed/open state, collision and interaction remain
active. Cutaway is presentation rather than opening or removing the door.

On a placed item's **Entity Nodes**, add an **Attribute Override** with name
`iso_cutaway_area`, type **Text**, and the name of an existing interior area:

```toml
[attributes]
iso_cutaway_area = "Inn"
```

The area must be a sector marked `iso_interior = true`. Its stable Creator UUID
can also be used instead of its name. A player in another room does not hide
this door. Clear the field to disable the cutaway. No extra named area is needed
for each door. Native 3D Iso uses the same association; first-person views keep
the door visible.

## Keys and locks

The supplied **Inn Key** is a normal item beside the starting position. The door
references its stable item-template ID. To try the standard lock, enable
**Locked** on the door's Item State node before playing. Put the key in the
player's inventory using the normal Take command, or a startup Add Item node.
Take can be assigned to an action-bar slot or a player Input Binding.

The default policy is:

| Request | Result |
| --- | --- |
| Open an unlocked door | Select On / Open. |
| Open a locked door without its key | Keep it closed and show the locked message. |
| Open a locked door with its key | Unlock it and open it; retain the key. |
| Close an open door | Select Off / Closed; keep its unlocked status. |
| Close while a character occupies the closed leaf's footprint | Refuse to close over the character. |

Key references accept an item-template ID, ruleset ID/path or exact item name.
Keys inside carried containers also count. Prefer IDs for saved projects so
renaming an item does not change which lock it fits. **Locked** and **Required
Key** are gameplay data on the item; the MZ recipe contains presentation and
footprints, never an inventory check.

## Custom behavior nodes

An authored **Intent** event branch takes ownership of the item's intent
handling. The automatic door policy then steps aside. This is useful for
consumable keys, puzzles, timed gates, relocking and quest conditions. Handle
the intents you want that item to support, including Look if needed.

A keyed Open branch can use:

1. **Event (Intent) → Filter**, with field `event.intent`, Equals, `open`.
2. **Inventory Has**, selecting the key. It checks the player who triggered
   the intent; stable template IDs work here too.
3. **Has → Set Attribute**, `locked` = Boolean false, then **Set Attribute**,
   `active` = Boolean true.
4. **Missing → Message**, such as “You need the inn key.”

Handle `close` in another branch and set `active` to false. Writing `active`
through the existing Set Attribute or Toggle Attribute controls selects the
same MZ output as the default intent handler. No camera-specific node chain or
tile replacement is needed. Custom graphs own their own lock and occupancy
policy; direct attribute writes intentionally do not run the automatic policy.

In ordinary sprite-based 2D projects, existing `on_tile_id` / `off_tile_id`
bindings continue to follow `active`. This keeps Hideout's node workflow while
allowing MZ-backed items to supply native geometry and iso sprites.

## Collision and lighting

For MZ items with an authored Footprint, movement uses that footprint rather than
the legacy item collision circle. The same footprint is used in iso and 3D,
including the item's world rotation, elevation and scale. An open leaf remains
solid in its swung position while leaving the doorway clear.

Iso sprites retain their material/surface data and mesh shadows. 3D renders the
native mesh. Both respond to the current sun and local lights. The two states
are discrete poses; State Switch does not itself add a timed swing animation.
