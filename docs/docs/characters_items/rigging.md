---
title: "Character Rigging"
sidebar_position: 4
---

Rigging in Eldiron lets a character avatar adapt visually to equipped gear:

- armor can recolor body parts
- weapons can attach to hand anchors
- front/back/left/right views are handled automatically

Think of the avatar as the base body, and equipped items as visual modifiers on top.

To enable avatar-based rendering on a character, set an avatar in the character attributes, for example:

```toml
avatar = "human"
```

---

## Typical Workflow

- Create or edit your character avatar in the Avatar Editor.
- Set base body-part colors in the character attributes.
- Add hand anchors (`Main Hand`, `Off Hand`) on the avatar frames you need.
- Configure item attributes so equipped armor/items can override those colors and add weapon overlays.
- Equip items in-game (or via scripts) and verify the visual result.

---

## Armor Item Overrides

Body-part colors are first read from the character attributes. Equipped armor items can then override those same body-part colors. This is how one armor set can recolor torso/legs/skin/hair without replacing all sprites.

Supported body parts:

- `light_skin`
- `dark_skin`
- `torso`
- `legs`
- `hair`
- `eyes`
- `hands`
- `feet`

For each body part, choose either:

- palette index: `<body_part>_index`
- exact color: `<body_part>_color`

```toml
torso_index = 3
legs_color = "#7c5533"
```

If both character and equipped item define a body part color, equipped item values win.

---

## Weapon Item Overlays

Weapons are drawn as overlays attached to avatar hand anchors.

Useful item attributes:

- `tile_id` (required): source tile for weapon sprite
- `rig_scale` (optional): weapon scale, default `1.0`
- `rig_pivot` (optional): attach point on weapon sprite, default center

```toml
tile_id = "YOUR_WEAPON_TILE_UUID"
rig_scale = 0.8
rig_pivot = [0.5, 0.5]
```

`rig_pivot` meaning:

- `[0.0, 0.0]` top-left
- `[0.5, 0.5]` center
- `[1.0, 1.0]` bottom-right

String format is also accepted:

```toml
rig_pivot = "0.5, 0.5"
```

You can also provide perspective-specific weapon tiles:

- `tile_id_front`
- `tile_id_back`
- `tile_id_left`
- `tile_id_right`

If missing, `tile_id` is used for all perspectives.

---

## Anchors In Avatar Frames

Weapon placement depends on avatar anchors:

- `Main Hand`
- `Off Hand`

Best practice:

- place anchors on each animation frame where weapon motion matters
- keep anchor movement smooth between frames
- verify all perspectives used by your game

If a specific frame anchor is missing, Eldiron falls back to available perspective data.

---

## Equip Slots For Weapons

Set the item `slot` to the names defined in your Game Settings.

Default weapon slots are:

- `main_hand`
- `off_hand`

For avatar hand lookup compatibility, Eldiron also recognizes common aliases.

Main hand aliases:

- `main_hand`
- `mainhand`
- `weapon`
- `weapon_main`
- `hand_main`

Off hand aliases:

- `off_hand`
- `offhand`
- `weapon_off`
- `hand_off`
- `shield`

Use your preferred naming style; these aliases are resolved by the runtime.

---

## Perspective Draw Behavior

By default:

- front/left/right: body first, then weapon overlay
- back: weapon first, then body

This makes back-view weapons feel properly behind the character.

---

## Update Behavior

Rig visuals update when equipment or rig-related character data changes.

If you do not see the expected result right away, re-check the equipped items and their attributes.

---

## Quick Example (Armor + Weapon)

```toml
# Armor-like item
slot = "torso"
torso_color = "#8b5a2b"
legs_index = 4

# Weapon-like item
slot = "weapon_main"
tile_id = "YOUR_SWORD_TILE_UUID"
rig_scale = 0.9
rig_pivot = [0.45, 0.5]
```

After equip, the avatar should reflect the new armor colors and weapon overlay.
