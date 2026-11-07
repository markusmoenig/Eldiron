# New-project procedural materials

Each `.material` is a complete, standalone MaterialZ PBR recipe. Edit it here or
open it in MaterialZ; there is no dependency on another repository. Each companion `.palette.json`
authors Eldiron color choices as project palette indices, including ramp stops
and material defaults. RGB constants in `.material` are only a portable cache
for standalone MaterialZ. On import, Eldiron resolves the saved indices through
the project palette; changing that palette updates all uses. Custom palettes
with missing slots use the nearest existing color rather than inserting RGB.

Creator discovers files in this folder, sorts them by filename, and adds editable
Tilepicker entries to new blank projects and projects created from starters.
`result from material` selects the material to import; its `called` label names
the Tilepicker entry. Use numeric filename prefixes to set their order. Existing
saved projects are not populated or overwritten. Once created, each project owns
its recipe copy, so editing these files affects subsequently created projects.

The usual GPU worker generates edited tile textures. Validated default bakes in
`default-bakes.eldiron` bootstrap new projects without rebuilding the stock ruleset
library and six starters. Source fingerprints, palette, resolution and recipe
keys must match; changed definitions fall back to a fresh bake. Each file's default base color
provides a temporary swatch when no matching bake exists. Sources use raw coordinates
so source detail can vary across world-coordinate pages. Patterns such as the
brick grid intentionally repeat their layout.

After changing the stock recipes or palette bindings, regenerate the embedded
archive with the ignored maintainer test (this is CPU baking, without a GPU):

```sh
ELDIRON_PREPARE_MZ_CACHE="$PWD/creator/embedded/materials/default-bakes.eldiron" \
cargo test -p eldiron-creator prepare_embedded_material_bakes --lib --offline -- --ignored
```

The cache is derived data. The editable `.material` and `.palette.json` files
remain the source of truth; an authored project override is never replaced by it.

- Grass: stretched grass grain with two editable endpoint color rows and a four-tone ramp.
- Rock: angular stone facets with dark crevices and four stone tones.
- Hills: a consistent stone-gray ramp for slopes and pale crests; it does not deform geometry.
- Street: restrained, four-tone asphalt, ready to mix using road shapes.
- Walls: staggered clay bricks with pixel-aligned mortar and subtle continuous weathering.
- Water: opaque blue/teal ripples with low roughness, ready for 2D and 3D surfaces.

Rock, Walls and Water are blocking in 2D. Their roles are Mountain, Man Made
and Water respectively. Grass (Nature), Hills (Mountain) and Street (Road) are
walkable. These tile properties survive baking, copying and project saves.
Ruleset item-state sprites stay available to items and are excluded from the
Tilepicker; they do not occupy starter-material slots.

The minimap's **Continuous / Repeat / Seamless** icons edit the selected
material's saved tiling mode and update every map use after its GPU bake. Output
and selected-source viewing controls are separate. Period X/Y and edge-blend
width are available in material properties. Continuous mode varies across the
world; Repeat wraps the authored period; Seamless also blends its edges.

Keep materials in PBR mode with Color, Roughness, Metallic, Opacity and Emission.
No Height, Normal or BSDF output is required by the current Eldiron runtime.

Pixelization is a global **project** render setting, not part of these recipes.
Game configuration defaults to:

```toml
[materials]
pixels_per_tile = 16
```

In **Game → Settings → Procedural Materials → Pixels per tile**, slide or type
a resolution from 1 to 64. The default stores actual 16×16 textures; 32 stores
32×32 textures. Use 0 for smooth evaluation with 64×64 storage. Changing this setting
rebakes all procedural tiles and previews on the shared surface grid. Color,
opacity, roughness, metallic and emission use the same grid; recipe copies stay
standalone and retain their full procedural detail.

Beside this control, **Characters & Map Items** has independent appearance settings:

```toml
[appearance]
sprite_pixels_2d = 0 # 0: source detail; 16/32/64: sprite sampling grid
pixel_size_3d = 1   # 1: native; 2..16: screen-pixel blocks
```

Both controls update the map live and support undo. They affect characters and
items placed in the world, including bitmap sources. Inventory icons retain their
own resolution. In 3D the meshes render on a coarse grid with their normal lighting,
shadows and animation, then expand with nearest sampling and depth testing against
the full-resolution world. This changes the silhouette as well as the shading;
floors, walls and other world surfaces retain their own material resolution.
Pixelized 3D entities sample authored texels directly, without mip averaging. Their
final shaded colors snap to the project palette, with post effects evaluated on the
coarse grid and bloom excluded from their pixels. This keeps crisp pixel blocks
while retaining scene lighting, fog, shadows and world occlusion.
These controls apply to the default Raster2D / Raster3D renderers.

Water uses 16 Loop Flow frames. Eldiron advances one frame per server animation
tick, exactly like normal tiles: the default 250 ms tick gives a four-second
loop. Changing the server tick changes both kinds of tiles together. The minimap
uses the editor's tile tick too. Material Props exposes Loop Frames; the seconds
value in portable MaterialZ recipes applies to standalone MaterialZ playback.
Compositions inherit animated sources and retain independent spatial/time frames.
Walls use Continuous Grid cell identities and three clay tones, with subtle
weathering. The same recipes/core features are back-integrated into original MZ.

Repeat and Seamless bake one complete period per animation frame, independent
of map size or placement count. Continuous mode keeps world pages. Identical
color-and-PBR frames share physical GPU atlas storage, including composition copies.
