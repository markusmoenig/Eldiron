---
title: "Character and Item Appearance"
sidebar_position: 3.5
---

The official ruleset supplies editable MZ bodies, clothing materials, item icons
and item models. Character setup decides what a character wears; appearance
editing changes the artwork used by those definitions. You do not need to build
a rig or copy material bindings to dress an ordinary ruleset character.

## Dress a character

1. Open the character's **Entity Nodes**.
2. Choose its ruleset race and class on **Entity**.
3. Add an **Equipment** node connected to Entity when you want a custom outfit.
4. Drag item headers from the project tree onto Equipment. Names are displayed,
   while stable project item references are stored. A drop replaces the item in
   the same slot; conflicting two-handed equipment and class restrictions are
   reported.
5. Open the character's **Appearance** entry to inspect the result in **2D** or
   **3D**, choose an animation, play/pause it, scrub its phase or turn the preview.

Without an Equipment node, the character inherits its class outfit. An explicitly
empty Equipment list equips nothing. Starting equipment belongs in character
setup; runtime equip/unequip changes belong in behavior or game interactions.

The default Human, Orc and Skeleton are distinct MZ body recipes. The same rig,
clothing, materials and attached items produce directional sprites in 2D and
native posed meshes in 3D. Procedural 3D characters are not camera-facing sprite
billboards. Authored bitmap avatars remain available as a separate art choice.

## Edit an item's appearance

Select **Appearance** beneath the item in the project tree. The workspace offers:

| Control | What it edits or previews |
| --- | --- |
| Icon On / Icon Off | Independent active/default and inactive 2D icon recipes. |
| Model On / Model Off | Independent active/default and inactive world model recipes. |
| Worn model | A fitted or attached model used while equipped, when the item defines one. |
| Materials | The item's referenced procedural surface materials, when present. |
| On character / Object | Equipped preview versus the item by itself. |
| On / Off | The state currently shown in the appearance preview. |

These controls replace the former separate On/Off tree rows. On is the ordinary
default. An item with no distinct Off artwork falls back to its available default;
opening the Off editor lets you author that state separately.

Inventory icons and world models are separate artwork. A sword's icon does not
have to be a camera render of its model. In the map, the procedural item path uses
its world model in 3D. Ordinary 2D ground items use their state icon; items with
model effects can use a model-rendered sprite to keep effect anchors aligned. Worn clothing
uses fitted geometry and material bindings; held equipment uses its grip and the
character's socket.

Official item icons are **32×32** MZ recipes. Their colors, outlines and shading
ramps use project palette indices. The global world-material resolution and
character/map-item pixelization controls do not resize inventory icons.

## On/Off behavior

Previewing Off does not toggle the live game's item. At runtime the item's
boolean **`active`** attribute selects its state. Behavior changes that value
and handles any additional gameplay response, such as a light or fuel use.

The procedural bindings are:

| Attribute | Source |
| --- | --- |
| `appearance_icon` / `appearance_icon_off` | On / Off icon recipe. |
| `appearance_geometry` / `appearance_geometry_off` | On / Off model. |
| `appearance_worn_geometry` | Model fitted or attached to a wearer. |
| `appearance_materials` | Explicit mesh-part-to-material bindings for worn surfaces. |

Use the workspace controls for ordinary editing. These keys are useful when
inspecting item TOML or authoring custom ruleset definitions. Material bindings
use `mesh-part=material-source`, for example a `ruleset:material/...` source;
geometry can reference `ruleset:geometry/...` or a project asset UUID.

The torch demonstrates independently editable icons and models. Its lit icon is
animated, and its 3D recipe can attach particle effects to the model/rig. The item
behavior still owns light switching and fuel consumption. Bitmap-only items can
continue to use `on_tile_id` and `off_tile_id`; see [Attributes](attributes#active).

## Change artwork, not every wearer

An item references its MZ assets. Editing a referenced asset changes every place
that uses it; duplicating and rebinding an asset creates an independent variant.
Clothing and equipment keep their ruleset identity and gameplay properties while
you edit their look. Project edits are saved and survive ruleset refreshes.

All authored recipe colors resolve through the project's single palette. New
projects start with **Endesga 64**. [Palette switching](../creator/tools/palette)
can adapt that artwork to a smaller or larger color set without reauthoring each
body, garment or icon. Direct bitmap overrides keep their own image colors.

## Model, rig and animation editing

Use **Edit model** on a character's Appearance workspace to open its MZ geometry.
Geometry/rig editing uses the **main editor viewport**, with construction rows in
the sidebar. Material texture previews use the **minimap**.

Start with published body or item dimension controls. Open a group to inspect its
ordinary geometry operations when those controls are not enough. The viewport
provides **Rig / joints**, **Add joints**, **Shape profiles**, **Pose / animate**
and **Attachments** tools. Animation controls remain available while inspecting
the rig. See [Rigging](rigging) for authoring skeletons, grips and motions.

The material editor can also use **Image / Tile Source** to combine bitmap art
with procedural shapes, or **Model Render** to use model output in artwork. See
[Procedural Materials](../building_maps/procedural_materials).

## Pixel style

Under **Game → Settings → Characters & Map Items**, **2D sprite pixels** sets the
sampling grid for characters and items on the map, while **3D pixel size** sets
crisp screen-pixel blocks for their meshes and shading. World textures and
inventory artwork retain their own settings. See
[Tile and Appearance Settings](../configuration/game#tile-and-appearance-settings).

The preview and runtime share appearance assets, but inspect changes in a real
2D/3D scene as well: camera scale, lighting and nearby scenery affect readability.
Rendering performance and export/platform coverage remain active development;
a preview working correctly is not a guarantee of a particular game FPS.
