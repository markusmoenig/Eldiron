# Procedural Recipes Concept

## Purpose

Eldiron currently has several related procedural systems:

- TileGraph for procedural tiles
- Generated patterns in 3D Paint
- Procedural paint brushes
- Procedural stamps such as grass, trees, rubble, vines and candles
- Particle generation inside TileGraph

These systems should be replaced by one simple, user-editable source format. TileGraph has grown toward a Material Maker/Substance-style workflow with too many nodes and too much authoring complexity, and should not remain as a second procedural authoring path.

The replacement is a **Procedural Recipe**: Eldiron's shared, consumer-neutral procedural asset language. The same core evaluator should serve tiles, materials, brushes, stamps, paint, particles, Avatar appearance, equipment, UI controls, icons, props and generated geometry. A Recipe should feel closer to a small image editor than a programming language or node graph.

The Recipe system should be universal, but an individual Recipe does not need to be useful in every context. A cloth material may work on a wall, shirt, banner, helmet lining or button background. A wall-layout Recipe may only make sense to a geometry consumer. Reuse comes from shared primitives, parameters, materials, shapes and outputs rather than forcing every Recipe to support every consumer.

## Design Goals

- Simple enough to use without understanding node graphs or programming.
- One shareable asset can be reused anywhere its declared outputs satisfy a consumer's contract.
- Existing built-in patterns and stamps become editable recipes instead of hard-coded choices.
- The procedural core is independent of Tiles, Avatars, UI, maps and renderer-specific data models.
- Every applicable visual operation can work in tileable/wrapping space.
- Deterministic seeds make procedural content stable and reproducible.
- Recipes can generate coverage, color, material, masks, height, normals, particles, attachments and geometry together.
- Consumers provide typed context such as dimensions, coordinates, state, colors, masks and transforms.
- Existing Tile and Material recipes remain compatible while compiling into the shared core.
- The editor provides useful presets and immediate visual feedback.

## Non-Goals

- Recreating Substance Designer or exposing a large general-purpose node graph.
- Supporting arbitrary procedural code.
- Guaranteeing that every Recipe produces an equally useful Tile, Avatar, button, brush, stamp or geometry object.
- Replacing the placement tools: recipes describe generated content, while paint and stamp tools control where it is placed.
- Allowing Recipes to mutate gameplay state, query arbitrary engine systems or become a second scripting language.
- Making the core crate depend on Avatar, UI, map or `GeometryObject` implementation types.

## Core Architecture

Procedural Recipes should be split into a consumer-neutral kernel and small consumer adapters.

```text
Recipe Core
  parameters and inputs
  coordinate domains
  fields, patterns and noise
  SDF shapes and strokes
  color and material composition
  attachments and particles
  generic geometry output
            |
            +-- Tile Adapter
            +-- Brush / Stamp / Paint Adapter
            +-- Avatar / Wearable Adapter
            +-- UI Adapter
            +-- Geometry / Prop Adapter
```

The core parses and validates a Recipe into an immutable, versioned program. A consumer invokes that program with a typed context and requests named outputs. The adapter converts those outputs into its own runtime representation.

Conceptually:

```text
RecipeProgram
  schema version
  declared parameters
  resources
  fields
  shapes
  materials
  operations
  named outputs

RecipeInvocation
  dimensions
  coordinate domain
  seed and stable instance id
  time and state
  scalar, vector and color inputs
  optional masks, distances and transforms

RecipeResult
  named typed outputs
```

The evaluator must remain deterministic, bounded and side-effect free. There are no arbitrary loops, filesystem calls, gameplay mutations or consumer API calls. Consumers control render dimensions, operation budgets, particle limits and geometry limits.

### Typed Inputs

Recipes declare the parameters they accept instead of reading implicit Tile-, Avatar- or UI-specific globals.

Useful input types include:

- Scalar, integer and boolean.
- 2D/3D vectors and transforms.
- Color and palette-family requests.
- Stable ids and seeds.
- Time and normalized animation phase.
- State enums such as normal, hovered, pressed or disabled.
- Scalar fields such as coverage, height, distance-to-edge or wear masks.
- Named anchors or consumer-provided attachment transforms.

```text
Input primary: Color = #805A3C
Input secondary: Color = #C49A62
Input wear: Scalar = 0.15
Input state: State = normal
Input size: F2 = F2(1.0, 1.0)
```

Required inputs must be supplied by the adapter. Optional inputs use declared defaults. Unknown inputs are reported so misspelled parameters do not silently do nothing.

### Typed Outputs And Consumer Contracts

Recipes publish named outputs with explicit types. Each adapter declares the output types it understands.

```text
Tile Adapter       coverage, color, height, material, particles, geometry
Avatar Adapter     coverage, color, material, normal, attachments, particles
UI Adapter         coverage, color, border, icon, shadow
Geometry Adapter   geometry, surface materials, attachments
Paint Adapter      coverage, color, material, particles
```

Unsupported optional outputs may be ignored. Missing required outputs are validation errors at the consumer binding, not failures deep inside rendering. This permits one Recipe to expose both a simple surface and richer geometry while each consumer selects what it can use.

## Authoring Model

A recipe is an ordered stack of layers. Each layer either introduces visual content through a **source** or changes the accumulated result through a **modifier**. Layers appear as compact rows rather than large cards or nested graph nodes. Selecting a row reveals all of its useful visual parameters.

Typical sources include:

1. **Shape** — rectangle, rounded rectangle, circle, ellipse, line, tapered line, capsule, polygon, star, blade or blob.
2. **Stroke** — straight, curved and tapered strokes used for branches, vines, roots, cracks and borders.
3. **Pattern/Scatter** — grid, bricks, radial strokes, borders, organic scatter or directional repetition.
4. **Noise** — value noise, cellular noise and other directly generated procedural fields.
5. **Particles** — emitted visual instances and runtime emitter descriptions.

Typical modifiers include Transform, Repeat, Warp, Mask, Breakup, Gradient, Color and Material. Layers can be reordered, duplicated, enabled/disabled and grouped.

Every useful numerical parameter supports a fixed value and, where meaningful, a deterministic variation amount. Each layer has a stable seed offset, so changing layer order does not silently change its random result. The Recipe-level **Reroll** action changes the shared seed and produces another deterministic variation.

Presets are never opaque operations. A Tree preset, for example, expands into ordinary tapered-stroke, color-gradient, radial-stroke, blob-scatter and noise layers. Users can inspect, alter, reorder or delete every part.

### Shared SDF Shape Language

Shapes should be represented as signed-distance fields wherever practical. SDFs provide one reusable language for coverage masks, borders, icons, wearable pieces, UI controls, decals, height fields and later shallow extrusion.

Initial primitives should include:

- Rectangle and rounded rectangle.
- Circle, ellipse, capsule and arc.
- Polygon, star, blade and blob.
- Line, tapered line and curved stroke.
- Union, intersection and subtraction.
- Smooth union, expand, contract and rounding.
- Transform, mirror, repeat and radial repeat.

The same rounded-rectangle expression could define a Tile inset, a button background, a shield emblem or a helmet plate. Consumers decide whether to rasterize it, use it as a mask, paint it onto a surface or convert it to geometry.

```text
Shape Panel
  RoundedRectangle
    size = Input.size
    radius = 0.08

Shape Border
  Subtract(Expand(Panel, 0.03), Panel)
```

SDF support does not require replacing the current frame-based Avatar system. For v1, the Avatar adapter can rasterize Recipe shapes and materials into existing sprite frames. A later Avatar system may use the same shapes as authored body or equipment parts.

## Canonical Recipe Language

Every recipe also has a copy/pastable text representation. The format should read like natural language, but it must be a controlled and versioned language rather than unrestricted prose. This keeps recipes deterministic, validates them without an AI service and prevents their meaning from changing between Eldiron versions.

The language describes generic procedural programs and named outputs. Consumer-specific metadata remains in the consumer binding. For example, a wearable item selects an appearance Recipe and Avatar channels in the ruleset; the Recipe itself does not contain an equipment slot or race name.

```text
Recipe quilted_panel
  Input primary: Color = #805A3C
  Input secondary: Color = #C49A62
  Input wear: Scalar = 0.15
  Input size: F2 = F2(1.0, 1.0)

  Pattern Quilting
    Diamonds
      cells = I2(5, 7)
      rounding = 0.12

  Noise Wear
    type = Gradient
    fractal = FBm
    scale = F2(8.0, 8.0)

  Shape Panel
    RoundedRectangle
      size = Input.size
      radius = 0.08

  Material Cloth
    color = Mix(Input.primary, Input.secondary, Quilting.edge)
    roughness = 0.84
    normal = Quilting.height + Wear * Input.wear

  Output surface
    coverage = Panel
    material = Cloth
```

For example:

```text
recipe "Old Oak"
canvas 96 by 128, clamp

add tapered trunk from bottom to 65%
color with gradient dark-brown to warm-brown

repeat 7 branches radially
vary rotation by 12 degrees
scatter foliage blobs around branch ends
break foliage with cellular noise at 20%

emit falling leaves from foliage
rate 2 per second
lifetime 3 to 6 seconds
```

The visual operation stack and text editor are two views of the same parsed recipe structure. Changes in either view update the other. Parser errors should identify the line, explain the expected form and preserve the last valid preview.

An optional **Describe Recipe** field may accept unrestricted natural language and translate it into the canonical format. The generated canonical recipe—not the original prompt—is saved as the project asset. Copying canonical recipe text must always reproduce the same structure and result.

## Coordinate and Tileability Model

Recipes normally operate in normalized local coordinates. They do not initially know whether the final consumer is a Tile, brush, stamp, Avatar, button or geometry object. The adapter maps its consumer space into a declared Recipe domain.

Useful coordinate domains include:

- **Normalized 2D** for images, masks, UI and sprite regions.
- **Pixel 2D** when pixel-grid behavior must be explicit.
- **Pattern Local** for independently evaluating every brick, scale or scattered instance.
- **Surface UV** for Tiles, painted faces, clothing and props.
- **Part Local** for Avatar torso, limbs, helmets and equipment pieces.
- **Local 3D** for geometry generation.
- **World/Placement** only when explicitly supplied by a placement adapter.

Recipes may transform between compatible domains, but they must not implicitly query a map, Avatar or widget for coordinates. The adapter owns that mapping and supplies stable instance ids for repeated elements.

Every applicable operation supports a wrapping mode:

- **Clamp** for isolated stamps and brush tips.
- **Repeat** for seamless tiles and repeating paint patterns.
- **Mirror** for symmetric seamless repetition.

Noise, distortion, gradients, patterns and shape repetition must all respect the selected wrapping mode. A tile preview should show neighboring repetitions so seams are visible immediately.

Randomness is derived from the recipe seed plus stable operation and instance identifiers. The same recipe and seed therefore generate the same result across editing, saving and runtime rendering.

Color ramps may be broad palette-family requests or art-directed anchored
ramps. An anchored ramp names a base color such as `DarkBrown` or provides an
exact target such as `#3A2418`, then describes relative brightness, saturation
and optional hue movement above and below that base. The adapter selects the
closest coherent colors from the project palette; the base color expresses
authorial intent without bypassing palette constraints.

```text
Colorize
    source = Surface
    base = DarkBrown
    brightness = F2(-0.18, 0.22)
    saturation = F2(-0.08, 0.06)
    hue = F2(-0.01, 0.01)
```

Reusable appearance is authored as another `.recipe` rather than copied into
every consumer. A recipe file may contain several `Material <id>` declarations;
the alias is `<relative-file>/<id>`. Materials can be referenced by Tiles,
wearables, Avatar parts, UI controls, paint, stamps and geometry surfaces.

A material receives only the typed fields declared by its contract. A Tile may
supply `Input.height`; an Avatar wearable may instead supply `Input.coverage`,
`Input.distance`, an item wear amount and canonical garment coordinates. This
avoids permanently coupling reusable Materials to Tile height while still
allowing a consumer to expose its geometry when appropriate.

Materials use the ordinary procedural vocabulary. Value and Gradient noise,
FBm/Ridged/Billow/Turbulence fractals, normalized `U`, `V`, `Radius`, and
`Angle` coordinates, and scalar math such as `Sin`, `Fract`, `Mix`, and
`Smoothstep` can describe wood, marble, cloth, stone, and other surfaces.
Those are recipes made from primitives, not hardcoded material categories.

Colorization has an explicit palette policy. **Strict** maps every ramp step to
the project palette. **BaseOnly** maps the authored base color once and then
builds an unrestricted brightness/saturation/hue ramp around that mapped
anchor. This lets each recipe choose between exact palette discipline and
finer gradients that still begin from the project's color language.

## Recipe Outputs

A recipe may generate several channels at once:

- **Coverage/SDF** — a primary mask, silhouette or signed distance field.
- **Color** — RGBA color or palette-derived color.
- **Material** — direct Roughness, Metallic, Opacity, and Emissive values.
- **Height/Normal** — optional surface relief information.
- **Emission** — visual emissive intensity or color.
- **Particle Emitters** — one or more emitter descriptions and their emission masks.
- **Visual Attachments** — optional named points used to align generated layers and effects with consumer-owned anchors.
- **Geometry** — crate-neutral vertices, faces, curves or constructive geometry operations.

Consumers request the channels they support. Required output mismatches are
reported when a Recipe is bound to a consumer. Geometry output must use a
consumer-neutral intermediate representation; the Geometry adapter converts it
to Eldiron's `GeometryObject` so the Recipe crate does not depend on Rusterix.

## Consumer Adapters

### Tiles

The tile adapter rasterizes the recipe over the tile canvas. Repeating coordinates make seamless textures possible. Particle output becomes an emitter attached to each placed tile, suitable for flames, smoke, dust, sparks, water drops and magical effects.

Tile-specific properties such as `blocking`, cell coverage, placement anchors and map-aware features belong to the Tile binding or `TileRecipe` adapter, not the consumer-neutral Recipe program.

Tile output has an explicit coverage measured in tile cells. A recipe can therefore produce a normal 1×1 tile or one coordinated multi-cell asset such as 2×2, 4×4 or another practical footprint.

```text
output tile
coverage 4 by 4 tiles
resolution 32 pixels per tile
anchor bottom-center
wrap repeat
```

The recipe evaluates continuously across the complete footprint and is then divided into coordinated tile cells. A 4×4 result produces 16 cells without seams between them. The outer boundary follows the recipe's Clamp, Repeat or Mirror wrapping mode.

Tile coverage includes:

- Width and height in tile cells.
- Pixel resolution per cell.
- Placement anchor/origin.
- Outer-edge wrapping mode.
- Optional cell occupancy mask for non-rectangular footprints.

The editor and runtime place, move, remove and reference the cells as one multi-tile asset. Variation remains stable per recipe instance and per cell. Particle emitters are batched across the footprint rather than duplicated blindly for every generated cell.

### Brushes

The brush adapter uses recipe coverage as the brush tip and evaluates color/material channels for every dab. Recipe variation can change individual dabs without losing deterministic results.

Particle output needs an explicit brush mode:

- **Transient** particles are only immediate painting feedback.
- **Persistent** particles paint an emitter mask onto the surface.

Persistent brush emitters must be combined spatially into chunks or masks instead of creating a separate runtime emitter for every dab.

### Stamps

The stamp adapter renders the recipe as an isolated procedural sprite or surface element. External stamp settings still control placement density, overall scale, rotation and exclusion rules.

Particle output remains attached to the stamp. Examples include sparks from a candle, smoke from a chimney, falling leaves around a tree, insects around flowers and dust around rubble.

Stamps and their particle emitters use persistent surface/object attachment, so they follow transformed Geometry Objects.

### Avatar Appearance And Wearables

For v1, Procedural Recipes enhance the existing frame-based Avatar system; they do not replace it. The Avatar retains its authored animations, eight perspectives, marker channels, equipment anchors and atlas-compatible runtime representation.

Wearable items reference a reusable appearance Recipe rather than defining a separate pattern language:

```toml
[items.padded_armor]
avatar_channels = ["torso", "arms"]
appearance_recipe = "wardrobe/quilted_cloth"
appearance_space = "part"
appearance_tiling = [2.0, 3.0]

[items.padded_armor.appearance]
primary = "#805A3C"
secondary = "#C49A62"
wear = 0.22
```

The Avatar adapter:

1. Reads the original semantic marker frame before recoloring.
2. Extracts coverage masks for the item's declared Avatar channels.
3. Maps covered pixels into canonical Avatar or part coordinates.
4. Invokes the appearance Recipe with item parameters and a stable instance seed.
5. Composites Recipe color and material output through the masks.
6. Applies Avatar shape shading, equipment layering and final presentation.

Useful wearable Recipes include cloth weave, stripes, checks, diamonds, quilting, scales, chain links, seams, stitching, trim, heraldry, runes, patches, dirt, scratches, dents and repairs. The same cloth, leather, metal or embroidery Recipe can also be used by Tiles, props, banners, UI or paint.

The item definition supplies allowed/default appearance parameters. The item instance stores its chosen colors, seed and overrides. The same result should be used for the inventory icon, portrait, 2D Avatar and enhanced 3D billboard so an item has one visual identity.

#### Eight-Direction Pattern Projection

Correct projection across eight independent sprite perspectives requires a shared surface coordinate system. Rendering the same Recipe independently into each frame's bounding rectangle causes patterns to resize, mirror and swim.

Each visible wearable pixel should map to canonical garment coordinates:

```text
Avatar frame pixel
    -> semantic body part
    -> canonical part UV
    -> appearance Recipe
    -> color and material output
```

For a torso, `U` represents distance around the body and `V` represents shoulder-to-waist height. Front, diagonal, side and back perspectives expose different ranges of the same canonical surface. Arms and legs use separate capsule/cylindrical part spaces. Left-facing views must not blindly mirror Recipe output because text, runes, heraldry, scratches and asymmetric fasteners must retain their orientation.

The v1 projection system has two levels:

1. **Automatic part projection** derives curved-quad or cylindrical coordinates from the semantic region, perspective direction and stable part identity. This is sufficient for repeating cloth, scales, chain, wear, trim and most patterns.
2. **Optional authored UV guides** provide control points or per-pixel correction for precise chest emblems, belts, collars, helmets and hero equipment.

Useful guide points include shoulders, waist corners, chest, belt, elbows, hands, knees and feet. The Avatar editor generates initial guides from marker regions and lets an author adjust them while previewing all directions and animation frames. Animation frames must preserve coordinate correspondence so patterns do not jump during Idle, Walk, Attack or Cast.

Recommended Recipe projection spaces are:

- **Avatar** — one normalized space across the complete frame.
- **Channel** — one space for a semantic channel such as torso or legs.
- **Part** — separate stable spaces for torso, left/right arms, legs, feet and headwear.
- **Anchor** — local coordinates around chest, belt, shoulder, head or another named anchor.

Broad repeating patterns work with automatic projection. Exact front-facing emblems should use an anchor or authored UV guide so they rotate toward diagonal views and disappear naturally in side/back views.

The Recipe receives canonical coordinates and does not normally need to know the perspective. Direction, animation name, frame and normalized animation phase may still be exposed as optional inputs for intentionally directional effects.

#### Runtime And Caching

Appearance is deterministic from the Recipe signature, Avatar definition, equipment, parameters, seed, direction, animation frame and render LOD. That complete signature keys a bounded cache.

High-resolution enhanced Avatar frames should be generated lazily for the current animation/direction and stored in a small LRU cache rather than expanding and caching every frame for every entity. Identical appearance signatures may share generated results.

For the current renderer, Recipe color and normal-height can be baked into the composed RGBA frame. Roughness, metallic, opacity and emissive outputs should remain available so a later Avatar billboard payload can carry full material data.

#### Recipe-Defined Avatars Later

A future Avatar document may use Recipe SDFs for complete bodies, helmets, equipment pieces, attachments and animation-friendly parts. That is not required for v1 and does not change the immediate wearable-material adapter.

The long-term separation is:

```text
Shared Recipe evaluator
  fields, noise, patterns, SDFs, colors, materials and geometry

Consumer documents
  TileRecipe
  MaterialRecipe
  AvatarRecipe (later)
```

The Avatar asset remains responsible for animation semantics, perspective mapping, gameplay-facing equipment/effect anchors, collision and identity. A Recipe remains visual/procedural data and never becomes Avatar behavior or gameplay logic. Existing hand-drawn atlases remain valid without Recipes.

### UI Controls And Icons

The UI adapter evaluates Recipes at a requested size and supplies theme colors, DPI and state inputs such as normal, hovered, pressed, focused, selected and disabled.

Recipe outputs can define:

- Button and panel coverage.
- Background fill and procedural material.
- Border, bevel and inset masks.
- Icons and state overlays.
- Highlight, shadow and focus-ring masks.

A rounded, quilted or engraved panel Recipe can therefore appear as a button, inventory slot, dialog frame, wearable plate or world prop. UI results are cached by Recipe signature, dimensions, theme inputs and state rather than regenerated every frame.

### Geometry And Props

Generic geometry construction belongs in the shared procedural vocabulary, while placement-aware behavior remains in adapters.

Useful generic operations include:

- Extrude an SDF or polygon.
- Bevel, inset and shell.
- Sweep a profile along a path.
- Revolve a profile around an axis.
- Repeat, mirror and transform instances.
- Boolean cut, union and intersection where supported.
- Assign named Recipe materials to generated surfaces.

Geometry output uses a crate-neutral intermediate form. The Geometry adapter converts it to editable `GeometryObject`s or cached detail geometry.

Map-aware discovery remains in the adapter, but authored geometry is generic.
The Tile/map adapter discovers exposed wall or ceiling placements, supplies a
placement-local basis, and executes the same Box/Add/Subtract/repeat program.
A niche is a subtractive Box and a ceiling beam is an additive Box; neither is
a dedicated Recipe AST feature. This keeps the Recipe core independent from
map queries without hardcoding architectural concepts in the adapter contract.

## Gradients

Gradients are important for describing organic stamps without requiring hand-painted assets. They can control color, opacity, width, density, height, material or particle parameters.

Examples:

- A tapered line with an along-shape gradient creates a trunk or branch.
- Radial gradients create soft foliage clusters, stones, flowers and puddles.
- Distance-to-edge gradients create outlines, worn edges and soft masks.
- A vertical color ramp shades a tree from dark trunk/base colors to brighter leaves.
- A gradient can reduce branch width, leaf density or particle size toward the end of a shape.

## Particle Layers

Particles are not a separate editor or special TileGraph feature. They are recipe layers with parameters such as:

- Emission mask and spawn shape
- Rate and maximum active particle count
- Lifetime
- Direction and velocity range
- Gravity, drag and turbulence
- Size and rotation
- Color, opacity and size gradients over lifetime
- Material/emissive behavior
- Collision mode where supported
- Deterministic seed and variation

Particle limits are applied at recipe, chunk and scene levels. Distant or hidden emitters can be reduced or suspended. Identical recipes should share runtime resources where possible.

## Example Recipes

### Brick Pattern

1. Rounded rectangle shape
2. Brick-grid repetition
3. Row offset
4. Size and edge variation
5. Mortar subtraction
6. Noise breakup
7. Stone color/material ramp

The same recipe can create a seamless tile, a brick paint brush or a wall-detail stamp.

### Tree Stamp

1. Tapered trunk shape
2. Repeated tapered branches with radial variation
3. Foliage blobs scattered around branch endpoints
4. Noise breakup on the foliage mask
5. Vertical and radial color gradients
6. Falling-leaf particle layer using the foliage mask

### Candle Stamp

1. Rounded wax body
2. Small tapered wick
3. Emissive flame shape with a vertical gradient
4. Flame flicker variation
5. Low-rate smoke and spark particle layers

### Dirt Brush

1. Soft radial coverage gradient
2. Cellular noise breakup
3. Small scattered flecks
4. Dirt material/color variation
5. Optional transient dust particles while painting

### Quilted Panel

1. Rounded-rectangle coverage SDF
2. Diamond field in normalized surface coordinates
3. Cloth base and secondary color inputs
4. Stitched edge mask
5. Rough cloth material and micro-normal output
6. Seeded wear and repair patches

The same Recipe can become a padded wall Tile, an armor wearable, a banner, an inventory button or an extruded decorative panel.

### Procedural Helmet

1. Dome, side-plate and nose-guard SDFs
2. Eye-opening subtraction
3. Mirrored rivet pattern with stable ids
4. Metal, leather and cloth materials
5. Head and effect attachments
6. Optional shallow extrusion output

For the current Avatar system the Avatar adapter rasterizes the helmet into each perspective using head-local coordinates and guides. A later Avatar or Geometry adapter may consume the same coverage and geometry outputs directly.

## Editor UI

The recipe editor should feel like editing a compact effect, not programming a graph.

- A top-level **Recipes** entry in the right-side project tree, alongside assets such as Avatars. Each Recipe is represented by one editable name row without nested metadata.
- A large live preview is the main workspace, with Surface, Tile, Brush, Stamp, Avatar, UI and Geometry preview modes.
- Layers are compact rows beside the preview. Each row shows the intermediate result and a short parameter summary. Selecting it opens contextual controls for size, count, spread, taper, variation, seed and other source-specific values.
- A visual operation stack is the default editor. Recipe Text is an optional advanced view for copying, sharing and uncommon commands; normal authoring never requires typing a recipe.
- Adding a layer uses a thumbnail shelf grouped conceptually into Sources, Appearance and Modifiers rather than a dropdown or node menu.
- Presets for common bricks, cobbles, foliage, trees, flowers, dirt, water, flames and smoke expand into the same editable layer primitives.
- Seed field with a reroll button.
- Typed input controls with defaults and saved preview presets.
- Seam preview toggle for tileable output.
- Tile-footprint preview for 1×1 and multi-cell coverage.
- Eight-direction and animation scrub previews for Avatar-bound Recipes.
- Normal, hovered, pressed, focused and disabled previews for UI-bound Recipes.
- Coverage, SDF, color, normal, material and geometry output inspection.
- Solo/mute controls for quickly inspecting one operation.
- Consumer-specific overrides kept outside the shared recipe.

Selecting or maximizing a recipe in the project tree opens the full Recipe Editor. Recipes can be created, renamed, duplicated and removed there in the same general workflow as Avatars.

The individual Tiles, Brushes, Stamps, Avatars, items, UI styles and Geometry tools show the referenced Recipe through a compact picker and thumbnail. Their **Edit Recipe** action opens the shared asset in the Recipe Editor. **Duplicate & Edit** creates a new Recipe for local variation before opening it. A maximized view provides more preview space without creating separate procedural editors for each consumer.

## Sharing and Ownership

Procedural Recipes are named project and ruleset assets with stable UUIDs and aliases. Tiles, brushes, stamps, Avatars, items, UI styles, props and Block Tool content reference those assets rather than embedding unrelated copies.

Recipes can be duplicated for local variation or added to the Block Tool/library as shareable content. Missing recipe references should fall back to a baked preview or a clearly marked placeholder.

Editing a shared Recipe updates every Tile, brush, stamp, Avatar, wearable, UI style and prop that references it. The UI must make this shared ownership visible before editing; **Duplicate & Edit** is the safe path when only one consumer should change.

Runtime assets need persistent Recipe catalogs. Parsed Material and Recipe definitions must not exist only as temporary compiler maps that disappear after Tiles are baked. Project and bundled ruleset Recipes use the same alias resolution and may be overridden according to explicit asset ownership rules.

## Replacing TileGraph

Procedural Recipes become Eldiron's only procedural visual authoring system. TileGraph should be removed from the normal editor and should not remain available as an advanced alternative.

Existing TileGraph content needs a transition path:

- Automatically convert common shapes, gradients, noise, patterns, color/material operations and particles into recipe operations where a clear mapping exists.
- Bake unsupported TileGraph content into ordinary tile frames or textures so old projects keep their appearance without retaining the graph editor.
- Keep a temporary read-only loader only as long as needed for project migration.
- Remove TileGraph assets, UI and runtime evaluation after migration is complete.
- Reuse suitable low-level algorithms internally when useful, without exposing TileGraph nodes or preserving its data model.

The Recipe format becomes the editable and shareable source for all supported procedural needs in Eldiron, including Tiles, materials, brushes, stamps, paint, particles, Avatar appearance, equipment, UI visuals, props and geometry.

## Evolving The Current Crate

The current `procedural_recipes` crate is a strong starting point. The first
consumer-neutral and SDF slices are implemented, while several deeper systems
remain tile-first:

- `RecipeDocument` distinguishes Tile, Material and SDF documents without
  changing existing Tile or Material syntax.
- The current `Recipe` data contains Tile properties such as blocking, cell coverage and placement geometry.
- `RenderOptions` primarily provides a seed offset rather than typed external parameters.
- Material evaluation can receive a consumer-neutral `RenderSurface`; the old
  Tile wrappers use the same path.
- Parsed Materials and SDFs are retained in project/runtime catalogs by alias.
- The Avatar wearable adapter projects Materials through semantic channel masks
  in all eight directions.
- The Avatar headgear adapter fits an SDF coverage recipe to the actual
  per-frame head marker bounds, then colors it with a separate Material recipe.
- Initial SDF support includes ellipse, rounded rectangle, capsule,
  union/subtraction/intersection and expand/contract. Transforms, smooth
  operations, richer primitives and full authored Avatar documents remain.

This should be refactored incrementally rather than replaced in one large rewrite.

### Shared Internal Program

Extract the common evaluator state into a generic internal structure such as `RecipeProgram`:

```text
RecipeProgram
  declared inputs
  fields and patterns
  SDF shapes
  colors and materials
  outputs
  deterministic seed rules
```

Existing `Tile` and `Material` syntax compile into that program plus adapter-specific metadata. Existing serialized assets and parser behavior remain compatible.

### Consumer-Neutral Evaluation Target

Material and field evaluation should consume a generic surface description rather than `RenderedRecipe` specifically:

```text
ProceduralSurface
  width and height
  frame times
  canonical coordinates
  stable sample ids
  typed input fields
  optional coverage, height and distance fields
```

Tiles construct a surface from their footprint. Avatar wearables construct one from semantic part masks and canonical UVs. UI constructs one from the widget rectangle and state. Geometry can sample fields over paths, surfaces or volumes.

### Asset Catalog And Compilation

Recipe source is parsed and validated once into an immutable program. Projects and rulesets retain those programs in an asset catalog keyed by UUID and alias. Consumers reference the catalog entry and provide invocation parameters.

Cache keys include:

- Recipe/program signature and schema version.
- Consumer adapter and output contract.
- Dimensions and coordinate mapping.
- Seed and stable instance id.
- Typed input values.
- Time/frame when animated.
- Consumer-specific masks or geometry signatures.

CPU evaluation may initially remain the common path. The program and output contracts should not prevent later GPU evaluation of compatible fields, but GPU execution is not required for the abstraction.

### Geometry Compatibility

The Recipe crate should retain its small dependency surface and must not import Rusterix map or geometry types. A crate-neutral geometry IR carries positions, faces, curves, material names, attachments and operation metadata. Adapters perform final conversion, placement, collision policy and baking.

## Suggested Implementation Order

1. Define the consumer-neutral `RecipeProgram`, typed inputs/outputs, deterministic seed behavior, coordinate domains and adapter contracts.
2. Compile existing Tile and Material documents into the shared program without changing their current output.
3. Add persistent project/ruleset Recipe catalogs and stable alias/UUID resolution.
4. Add a consumer-neutral procedural surface API and named scalar/color/vector/state inputs.
5. Implement the Avatar wearable adapter using existing marker-channel masks, part-local projection and one appearance Recipe per equipped item.
6. Add stable appearance seeds, bounded caching and the same Recipe-derived preview in inventory, portraits, 2D and enhanced 3D Avatars.
7. Add automatic eight-direction garment projection, animation-stable part ids and optional authored UV/anchor guides.
8. Add shared SDF shapes, transforms, gradients, repeat/scatter, mask combining and expanded pattern primitives.
9. Add the UI adapter with size, DPI and interaction-state inputs.
10. Retrofit stamps, brushes and paint patterns onto the same invocation API.
11. Add particle layers and runtime emitter batching.
12. Extend the initial crate-neutral Box/Add/Subtract/repeat geometry IR and Tile/map adapter with more primitives, transforms, and prop bindings.
13. Convert built-in patterns, stamps and applicable UI visuals into bundled editable Recipes.
14. Add TileGraph migration/baking and remove the TileGraph editor and runtime path.
15. After v1, evaluate a full `AvatarRecipe` document using the same materials, SDFs, inputs and outputs.

The first abstraction milestone should prove that one `quilted_panel` Recipe can produce:

- A padded or decorative Tile.
- A patterned wearable on all eight Avatar perspectives.
- A UI button or inventory panel background.

All three must share the same pattern and material program while supplying different dimensions, colors, seeds, coordinates and adapter metadata. If that works without Tile-, Avatar- or UI-specific branches inside the core evaluator, the architecture is sound.
