---
title: "Look and World Text"
sidebar_position: 13
---

**Look** reveals authored descriptions deliberately. Select **Look** on the game
toolbar and click an item or character. In 3D, linked Props also support Look.
The official intent rules still decide whether that interaction is permitted.
World objects have no automatic name or description tooltip on hover.

## Reading a description

The description appears above the inspected object, projected through the active
**2D, 2D Iso or 3D camera**. It follows the target as the target or camera moves.
For an MZ item, the anchor uses the current state's mesh bounds, world rotation,
scale and elevation, so a door's text is positioned above its current leaf.
A Look result is private to the player who requested it. Descriptions of carried
items follow their carrier, with an on-screen position when the first-person
camera cannot project the player.

Text wraps inside a small panel and stays within the game viewport. The renderer
reserves space for the bottom toolbar and tries to separate concurrent passages.
Inspecting the same object replaces its previous passage; up to four passages
can be visible concurrently.

- **Short descriptions** disappear from the display after a reading interval based on
  their length (6–18 seconds), or can be dismissed by clicking the text.
- **Long descriptions** use pages of up to four lines. Click the text to advance;
  clicking the last page dismisses it. Paged text waits for interaction.
- **Escape** dismisses active Look text.
- Clicking the text consumes the click, so it never starts a walk or uses an
  object underneath it. Leaving the region clears the previous region's text.

Descriptions use the existing message font, color and locale resolver. The
current feature is interactive Look text. Existing speech and dialogue nodes
retain their behavior; dialogue response choices are not yet part of these Look
panels.

## Author content on the placement

Enable **Authoring** in Creator's tool strip. Select an item in **Object** mode, a character or item with the **Entity**
tool, or use the region tree. The Authoring dock edits that placement's TOML:

```toml
title = "Old Door"
description = "Splintered oak boards, reinforced by rusted iron."

[state.off]
description = "The door is shut. Scratches surround its worn handle."

[state.on]
description = "The door stands open. Darkness fills the room beyond."
```

A template can supply defaults, and local authoring overrides those defaults.
Empty local text inherits the template. Item states use `state.off` and
`state.on` for the shared inactive/active flag, or the item's explicit `state`
attribute when present. Characters use `mode.active`, `mode.dead` or their
current mode. Each level can also supply a general `description`.

Placed linked Props have local authoring too; missing text falls back to their
shared asset authoring. Selecting the source asset in the Props workflow still
edits that shared default. Plain static tiles do not acquire item identity or
intent behavior just by having an MZ appearance.

The MZ recipe holds appearance, state poses and footprints. Authoring holds
narrative content. Neither an extra global Item template nor another material or
mesh recipe is needed just to write a particular door's description.

See [Authoring](../creator/authoring) for the dock workflow and
[Doors and MZ Item States](doors) for the UnendingNight example.
