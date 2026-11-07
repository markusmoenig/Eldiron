---
title: "Rigging and Animation"
sidebar_position: 4
---

For ordinary character setup, equip ruleset items through **Entity Nodes →
Equipment** and inspect **Appearance**. That is enough to dress a Human, Orc or
Skeleton. [Character and Item Appearance](appearance) explains the everyday
workflow; this page covers changing the model and rig itself.

## Open the model workspace

Open the character's **Appearance** entry and choose **Edit model**, or choose
**Model On / Model Off** for an item. The procedural geometry editor uses the
**main viewport**. Material textures continue to use the sidebar/minimap editor.
Use the preview's back control to return to Appearance or the map.

Start with the published body proportions or item dimensions. A group's controls
can adjust several ordinary construction rows together. Open that group to edit
its rows, then return to the parent group when finished. Geometry rows can build
contours, extrusions, lofts, revolved shapes and boxes; transforms, materials and
textures apply to the result. These are editable recipes, not fixed character
presets or opaque item-specific generators.

## Skeleton and body profiles

The viewport tool menu includes:

| Tool | Purpose |
| --- | --- |
| View model | Inspect the surface and silhouette. |
| Rig / joints | Select and edit the skeleton's bind joints. |
| Add joints | Extend the hierarchy with authored joints. |
| Shape profiles | Edit the profiles that form body parts. |
| Pose / animate | Work on motion keys for the selected skeleton. |
| Attachments | Inspect and edit published attachment frames and grips. |

Use Front, Side or Orbit views as appropriate. The geometry recipe's skeleton,
bind transforms and skin weights define how the mesh follows the joints. Body
profiles can use that rig to build a mesh around the skeleton. The supplied
humanoids are examples of this system, not the only possible skeletons.

A custom four-legged creature needs its own joint hierarchy, body profiles,
skinning and motion tracks. Adding joints alone does not automatically generate
a quadruped gait or adapt humanoid clothing. Check each garment and attachment
against the creature's actual part and socket names.

## Grips, sockets and worn items

An item's **Grip** defines the frame held by the character. An **Attach** operation
aligns that grip to the selected joint/socket. Edit a sword's grip or shield's
mounting frame rather than offsetting every character that uses it.

Equipment slots are ruleset data. The optional `equipment.avatar_anchors` map
connects declared slots to `main_hand` and `off_hand`; item appearance settings
can select the holding joint and two-handed handling. Worn accessories can attach
to other authored joints. Clothing recipes can consume `geometry:input/wearer`
and fit the wearer's current body before applying palette-bound materials.

Keep part IDs, joint names and attachment references consistent when replacing a
body or garment. The same bindings are evaluated for directional 2D sprites and
native 3D meshes. Use the item's **On character / Object** preview to inspect both
wearing and standalone placement.

## Motions and playback

Choose an available animation and use Play/Pause, phase scrubbing and preview
rotation to inspect it. Animation controls stay accessible in Rig mode. Editing
joint bind transforms changes the skeleton; editing a motion key changes a pose
in that animation. Check both the bind pose and the moving result.

Motion rows reference their skeleton and contain named joint tracks, normalized
key phases and visibility keys. Looping clips require matching endpoint poses.
Non-looping attacks can hold their final pose. Branch visibility can hide an
arrow after release and restore it for the next shot.

The supplied weapons select ready/walk/attack clips through `appearance_idle`,
`appearance_walk`, `appearance_attack` and `appearance_attack_time`. The bow's
own rig tracks combine with the body's draw/release animation; spear poses keep
both hands on the shaft. Item state and equipment remain gameplay data, while
these clips describe their visual presentation.

Visual release markers do not replace combat rules: the released bow arrow is
an animation visual, not a separately simulated server projectile. Validate
walking, attack recovery, clothing clearance and grips in actual 2D/3D play as
well as the model preview. Geometry, material, rig and motion edits participate
in project save and Undo/Redo.

## Bitmap avatars

Image-based avatars remain supported. The remainder of this page describes that
separate marker-color and frame-anchor workflow. Its hand overlays and sprite
perspectives do not describe how MZ's native 3D meshes are rigged.

A bitmap avatar can be selected through the character's `avatar` setting.
`size_2d` scales its 2D display; `size` controls its 3D billboard scale without
changing collision or gameplay size. Use **Edit avatar** in Appearance when the
selected source is a bitmap avatar.

### Bitmap tutorial video

This older tutorial demonstrates bitmap avatar authoring; current MZ model
editing uses the tools above.


Avatar setup and rigging walkthrough:

<div style={{ position: "relative", paddingBottom: "56.25%", height: 0, overflow: "hidden", marginBottom: "1rem" }}>
  <iframe
    src="https://www.youtube.com/embed/kmObqQil62k"
    title="Avatar Tutorial Video"
    style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "100%", border: 0 }}
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerPolicy="strict-origin-when-cross-origin"
    allowFullScreen
  />
</div>

---

### Typical Workflow

- Create or edit your character avatar in the Avatar Editor.
- Set base body-part colors in the character attributes.
- Add hand anchors (`Main Hand`, `Off Hand`) on the avatar frames you need.
- Configure item attributes so equipped armor/items can override those colors and add weapon overlays.
- Equip items in-game (or via scripts) and verify the visual result.

---

### Armor Item Overrides

Body-part colors are first read from the character attributes. Equipped armor items can then override those same body-part colors. This is how one armor set can recolor torso/legs/skin/hair without replacing all sprites.

Supported body parts:

- `light_skin` (`rgb(255, 0, 255)`)
- `dark_skin` (`rgb(200, 0, 200)`)
- `torso` (`rgb(0, 0, 255)`)
- `arms` (`rgb(0, 120, 255)`)
- `legs` (`rgb(0, 255, 0)`)
- `hair` (`rgb(255, 255, 0)`)
- `eyes` (`rgb(0, 255, 255)`)
- `hands` (`rgb(255, 128, 0)`)
- `feet` (`rgb(255, 80, 0)`)

For each body part, choose either:

- palette index: `<body_part>_index`
- exact color: `<body_part>_color`

```toml
torso_index = 3
legs_color = "#7c5533"
```

If both character and equipped item define a body part color, equipped item values win.

---

### Weapon Item Overlays

Weapons are drawn as overlays attached to avatar hand anchors.

Useful item attributes:

- `tile_id` (required): source tile for weapon sprite. Accepts UUID, tile alias, or palette index.
- `rig_scale` (optional): weapon scale, default `1.0`
- `rig_pivot` (optional): attach point on weapon sprite, default center
- `rig_layer` (optional): hand-depth ordering, `"front"` or `"back"` (default `"front"`)
- `rig_flip_back` (optional): if no `tile_id_back` is set, flip weapon horizontally in back view (default `true`)

```toml
tile_id = "YOUR_WEAPON_TILE"
# or:
# tile_id = "weapon_sword"
# tile_id = "12"
rig_scale = 0.8
rig_pivot = [0.5, 0.5]
rig_layer = "back"
rig_flip_back = true
```

`rig_pivot` meaning:

- `[0.0, 0.0]` top-left
- `[0.5, 0.5]` center
- `[1.0, 1.0]` bottom-right

String format is also accepted:

```toml
rig_pivot = "0.5, 0.5"
```

`rig_layer` controls draw order relative to the hand (while staying in front of the body):

- `"back"`: behind hand, in front of body
- `"front"`: in front of hand

You can also provide perspective-specific weapon tiles:

- `tile_id_front`
- `tile_id_back`
- `tile_id_left`
- `tile_id_right`

If missing, `tile_id` is used for all perspectives.

---

### Anchors In Avatar Frames

Weapon placement depends on avatar anchors:

- `Main Hand`
- `Off Hand`

Best practice:

- place anchors on each animation frame where weapon motion matters
- keep anchor movement smooth between frames
- verify all perspectives used by your game

If a specific frame anchor is missing, Eldiron falls back to available perspective data.

---

### Equip Slots For Weapons

Set the item `slot` to a name declared by the active ruleset's
`equipment.weapon_slots` or `equipment.armor_slots`.

Map slots that should render in an avatar hand:

```toml
[equipment]
weapon_slots = ["grip", "brace"]
armor_slots = ["shield_mount"]

[equipment.avatar_anchors]
grip = "main_hand"
brace = "off_hand"
shield_mount = "off_hand"
```

The keys can use any ruleset-owned slot names. The values name the two anchor
capabilities currently exposed by avatar frames: `main_hand` and `off_hand`.
The runtime does not infer aliases. Omit a slot from `avatar_anchors` when its
item should not be composited onto the avatar.

---

### Perspective Draw Behavior

By default:

- front/left/right: body first, then weapon overlay
- back: weapon first, then body

This makes back-view weapons feel properly behind the character.

---

### Update Behavior

Rig visuals update when equipment or rig-related character data changes.

If you do not see the expected result right away, re-check the equipped items and their attributes.

---

### Preview Tuning In Character Data

In the character **Rigging Preview** TOML, you can override weapon transform per slot without changing the item data.

```toml
animation = "Idle"
perspective = "Front"
play = true
speed = 1.0

[slots]
off_hand = "Shield"

[slot_overrides.off_hand]
rig_scale = 0.85
rig_pivot = [0.45, 0.58]
```

Notes:

- `slot_overrides` is preview-only (non-destructive).
- Keys under `slot_overrides` must match the slot key from `[slots]`.
- You can still keep final `rig_scale` / `rig_pivot` on the item itself.

---

### Quick Example (Armor + Weapon)

```toml
# Armor-like item
slot = "torso"
torso_index = 8
legs_index = 4
```

```toml
# Weapon-like item
slot = "main_hand"
tile_id = "YOUR_SWORD_TILE"
rig_scale = 0.9
rig_pivot = [0.45, 0.5]
```

After equip, the avatar should reflect the new armor colors and weapon overlay.
