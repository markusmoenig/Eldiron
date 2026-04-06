---
title: "Builder Tool"
sidebar_position: 6
---

The **Builder Tool** (keyboard shortcut **`B`**) lets you place reusable **builder graph assets** such as tables, wall torches, wall lanterns, campfires, fences, and other assemblies onto map geometry.

Builder graphs are structural assets. In the current release they are authored as text-based **`.buildergraph`** scripts with a live preview. They define:

- geometry
- named **material slots** such as `TOP` or `LEGS`
- named **item slots** such as a tabletop surface or attachment point
- a required host target:
  - `Sector`
  - `Linedef`
  - `Vertex`

## Builder Script Editor

Opening a builder asset shows the **Builder script editor** instead of the older node canvas.

The editor contains:

- a text editor for the `.buildergraph` script
- a live 3D preview on the right
- syntax highlighting for Builder keywords and identifiers

Builder scripts describe primitives such as `box` and `cylinder`, their attachment to the current host, and any named slots they expose.

Typical host usage looks like:

- `host = sector;` for floor or platform props
- `host = vertex;` for point-mounted props such as wall torches or campfires
- `host = linedef;` for span-based props such as rails or long wall pieces

The preview is intended for fast structural iteration:

- change dimensions or offsets
- confirm growth direction
- verify named material and item slots

## Picker Workflow

When the Builder tool is active, the lower picker area shows the **Builder Picker** instead of the **Tile Picker**.

From there you can:

- browse project builder assets
- create new assets with **New**
- select a builder asset
- **Apply Build** to the selected hosts
- **Clear** the builder graph from the selected hosts

Single-click selects a builder asset. Double-click, **Return**, or maximize opens the Builder script editor.

## Host Targets

Each builder graph declares its output target. That target decides what the tool applies to:

- **Sector** builders place assemblies on sectors, for example tables or platforms
- **Linedef** builders place assemblies along edges, for example fences, rails, or balconies
- **Vertex** builders place assemblies on points, for example wall torches, lanterns, campfires, or posts

Selecting a builder asset switches the map-edit host mode to the matching target automatically.

## HUD Slots

Builder hosts use the same upper-right HUD area as other map tools, but the icons represent **builder slots** instead of direct tile assignment.

There are two slot types:

- **Material slots**: assign visual tile sources to named parts such as `TOP` or `LEGS`
- **Item slots**: attach other builder assets to named anchors or surfaces such as a tabletop

For example, a table builder can expose:

- `TOP` and `LEGS` as material slots
- `TOP` as an item surface slot for child props placed on the tabletop

## Applying Materials

Use the **Tile Picker** with a Builder host selected to assign tiles to the currently selected builder **material slot**.

This keeps the builder graph reusable:

- the graph defines the slot names
- the placed instance decides which tile fills each slot

The same table graph can therefore be reused with different materials without duplicating the graph itself.

## Attaching Child Builders

Builder **item slots** can host other builder assets.

This is used for workflows such as:

- placing an object on a tabletop
- attaching content to a shelf
- mounting a child prop onto a stand

Point attachments use **item anchors**. Surface attachments use **item surfaces** such as a tabletop or shelf top.

## Presets

The first release includes a few ready-to-use examples:

- **Table**: a simple sector-hosted furniture prop
- **Wall Torch**: a vertex-hosted wall prop with material slots for base, torch body, and flame
- **Wall Lantern**: a second wall-mounted vertex prop used to validate wall attachment
- **Campfire**: a floor-mounted vertex prop used to validate point placement on the ground

These presets are meant as starting points for your own Builder assets.

## Example: Wall Torch

The following `Wall Torch` example is a good starting point because it shows the full basic Builder workflow:

- a `Vertex` host
- multiple primitives
- named material slots
- an item slot at the flame tip

Source:

```txt
name = "Wall Torch";
host = vertex;

preview {
    width = 1.0;
    depth = 0.4;
    height = 2.0;
}

let plate = box {
    attach = host.middle + host.out * 0.03 + host.up * 0.00;
    size = vec3(0.18, 0.28, 0.05);
    material = BASE;
};

let arm = box {
    attach = host.middle + host.out * 0.10 + host.up * 0.00;
    size = vec3(0.08, 0.06, 0.18);
    material = BASE;
};

let holder = box {
    attach = host.middle + host.out * 0.18 + host.up * 0.00;
    size = vec3(0.10, 0.10, 0.06);
    material = BASE;
};

let torch = cylinder {
    attach = host.middle + host.out * 0.18 + host.up * 0.02;
    axis = host.up;
    length = 0.42;
    radius = 0.025;
    material = TORCH;
};

let tip = cylinder {
    parent = torch.top;
    attach = vec3(0.0, -0.04, -0.075);
    axis = host.up;
    length = 0.18;
    radius = 0.03;
    material = FLAME;
};

slot item flame_top = tip.top;
slot material base_mat = plate.center;
slot material torch_mat = torch.center;
slot material flame_mat = tip.center;

output = [plate, arm, holder, torch, tip];
```

### How It Works

- `host = vertex;`
  - The torch is mounted from a single point on a wall surface.
  - This is a better fit than `linedef` for small props such as torches or lanterns.

- `preview { ... }`
  - Defines the preview host dimensions used by the Builder editor and CLI preview renderer.
  - It helps verify scale and orientation while authoring.

- `plate`, `arm`, `holder`
  - These three `box` primitives make up the wall bracket.
  - Their `attach` expressions place them relative to the host frame:
    - `host.up` moves vertically
    - `host.out` moves away from the wall face

- `torch`
  - A `cylinder` aligned to `host.up`, so it stands vertically.
  - `material = TORCH;` exposes the torch body as a separate material slot.

- `tip`
  - A smaller cylinder attached to `torch.top`.
  - This uses the Builder hierarchy model: the tip inherits the torch orientation.
  - The local `attach = vec3(...)` offsets place the flame volume correctly relative to the torch tip.

- `slot item flame_top = tip.top;`
  - Exposes an item attachment point at the top of the flame.
  - Child props or future effects can attach there.

- `slot material ...`
  - Exposes named material slots to the HUD when the Builder host is selected in Eldiron.

- `output = [...]`
  - Lists the parts that should actually be emitted into the final Builder assembly.

### In Eldiron

To test the example:

1. Create or select a wall in 3D.
2. Create a fresh `Vertex` on that wall face.
3. Switch to the `Builder` tool.
4. Create `Wall Torch` from the Builder picker.
5. Apply it to the selected vertex.
6. Assign tiles to:
   - `BASE`
   - `TORCH`
   - `FLAME`


## Tips

- Use Builder for reusable placed structures, not for painting terrain or assigning floor tiles.
- Keep the script generic and expose named slots instead of hardcoding materials.
- Start with simple `Sector` or `Vertex` assets before moving on to more complex `Linedef`-based props.
- Use `Vertex` hosts for point-mounted props such as torches and campfires.
- Use `Linedef` hosts for props that actually need span semantics.

## Related Pages

- [Overview](/docs/creator/tools/overview)
- [Tile Picker](/docs/creator/docks/tile_picker_editor)
- [Visual Script Editor](/docs/creator/docks/visual_script_editor)
- [Working With Tiles](/docs/building_maps/working_with_tiles)
