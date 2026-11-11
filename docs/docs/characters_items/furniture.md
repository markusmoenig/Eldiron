---
title: "Furniture: Sitting and Lying Down"
sidebar_position: 14
---

UnendingNight contains an editable **MZ oak bed** inside the inn, plus usable
interior and courtyard benches. These are ordinary placed items with local
Authoring content. They do not add furniture templates to the global Items list.

## Try it

1. Open `test_projects/UnendingNight.eldiron` and play.
2. Approach a bench, select **Use**, and click it. The character sits.
3. Open the inn door, enter and approach the bed on the right side of the room.
   Select **Use** and click the bed. The character lies down under the open roof
   cutaway.
4. Use the same furniture again, or issue a movement command, to stand up. The
   character returns to the approach position before walking.
5. **Look** displays the placement's authored description above the furniture.

Releasing the mouse button or an intent shortcut does not leave the furniture.
The `Off` input stops movement; only an actual movement request or using the
same furniture again stands the character up.

Official Use range and permission rules still apply. An authored Item Intent
branch can own the interaction instead of the default furniture behavior. The
same runtime posture and occupancy system is used in 2D Iso and native 3D.

## Edit the furniture and its pose origins

Select the bed or bench in **Object** mode and enable **Authoring** to edit its
local narrative content. Edit its MZ Geometry recipe to change construction,
materials, collision footprint or interaction placement.

The bed recipe builds an oak frame with individual headboard planks, slats,
mattress, pillow, folded blue blanket and hanging blanket edges. Its wood, cloth
and linen material bindings remain editable recipe references. The bench recipe
contains its seat, legs and footprint.

A **Connector** operation on the assembled mesh defines each usable pose origin:

| Connector family | Use result |
| --- | --- |
| `rest.sit` | Sit at this origin, facing the connector's direction. |
| `rest.lie` | Lie at this origin, facing the connector's direction. |

Connector positions and directions follow recipe transforms, item rotation and
item scale. The connector is the **character pose origin**, not the top of the
mattress or seat. Preview the character's Sit/Lie clip when aligning it: the
provided bench uses local Y = 0.05 and bed uses local Y = 0.60. These positions
match the ruleset humanoid poses and the supplied furniture dimensions. Different
body proportions or furniture heights may need different connector offsets.

Multiple rest connectors create multiple independent places. Use chooses the
nearest unoccupied place. Each place admits one character; another user receives
an occupied message. Use on the same furniture stands its user up. Blocking
footprints remain attached to the actual furniture.

These are recipe connectors rather than a separate action for each asset.
When creating furniture, assign its recipe to a placed item's **MZ Geometry**
appearance and make the item visible. Keep blocking enabled when characters
should walk around its footprint.

## Character poses and future recovery rules

The official MZ humanoid includes editable **Sit** and **Lie** motion rows.
Lie is a dedicated rest pose, independent of death or unconsciousness. A rest
pose stays active until the character leaves the furniture. Other character
appearances need matching animations to display the posture correctly.

Bed use selects the settled Lie pose immediately, without playing or blending
through the standing-to-lying rotation. Standing up also switches directly back
to the standing pose. Sitting retains its animated transition. The Lie motion
remains editable in MZ; furniture uses its final pose through `avatar_pose_only`.

The runtime exposes ordinary attributes that nodes and future rules can inspect:

- `posture`: `standing`, `sitting` or `lying`.
- `resting`: whether the character is using a rest place.
- `furniture_id` and `furniture_slot`: the occupied runtime item and connector.

Furniture use currently changes posture and occupancy. It does not grant
recovery, healing or other stat bonuses. Those policies can later be added to the
ruleset using the shared state, without putting game rules into the MZ recipe.
