# Image / Tile Source

Image / Tile Source is a stock source in the Image category. It samples a bitmap
with straight alpha, or a sequence of equally sized frames. The image and optional
Roughness, Metallic, Opacity and Emission planes are embedded in the material
recipe, separate from shader code. A saved recipe is usable without its source
file or Eldiron project.

In standalone MaterialZ, add this tile and use **Properties → Choose Image**.
Multiple selected images (or frames in an animated image) form an animation. Replace Image and Clear Image use normal document undo.

In Eldiron, add the tile from **Sources → Image**, then select it and open
**Props → Source**. Import an image, choose an existing bitmap tile, or clear the
source. A linked tile supplies its animation frames and material maps. Editing
that project tile refreshes the embedded cache and dependent material bake;
undoing the assignment restores the previous recipe. If a linked tile is absent,
the embedded pixels remain available. Importing does not create a Tilepicker tile.

## Sampling and composition

- **UV Domain:** Centered samples -0.5…0.5 (standalone default, useful for icons);
  Tile samples 0…1. Eldiron imports images in centered coordinates to match its
  tile previews and baked icon domain; world-composition bitmap inputs explicitly
  use Tile coordinates.
- **Edges:** Transparent (default), Clamp or Repeat. Material-level Repeat and
  Seamless settings continue to apply to the complete recipe.
- **Filtering:** Nearest (default) or Linear. Linear filtering interpolates
  premultiplied colors internally and returns straight alpha to prevent fringes.
- **Channel:** Color, Roughness, Metallic, Opacity or Emission. Color includes
  image alpha multiplied by its optional opacity plane. Opacity returns that
  coverage as a scalar RGB value. Missing material maps use roughness 0.5,
  metallic 0, opacity 1 and emission 0.

Connect Color to the material's Color output and W/Alpha to Opacity, or connect
RGBA to Alpha Over's Foreground input. Use Transform, Grid, image Border and other
ordinary tiles to place, repeat, mask and compose the image. Images have no signed
distance field. Duplicate the source tile and select another Channel to feed
additional PBR outputs.

Animated sources use normalized loop time. Importing sets at least the required
number of material animation frames. Eldiron plays baked frames on the normal
server/tile animation tick. Image sources support up to 32 frames, 1048576 pixels
per frame and 4194304 pixels across the animation. Each plane may have one static
frame or the common animation frame count.

## Shared document/API contract

`image_source::ImageSource::rgba8` accepts top-to-bottom RGBA8 frames in sRGB.
`set_plane` adds a channel in `srgb` or `linear`; alpha is always linear. Attach
this data to `format::Tile::image`. A host may set `reference` to a stable asset
identity and resolve it itself; the core never reads filesystem paths or URLs.
The source's sampling controls must be constants. Pixel changes at matching
size/frame count reuse GPU pipelines and replace parameter-buffer data.

```text
project 1
material "image" called "Image"
  mode pbr
  tiling raw
  row "bitmap"
    use "Image / Tile Source" id "bitmap"
      image size 2 1
      image frame color srgb ff0000ff0000ff80
  output base color from row "bitmap" xyz
  output opacity from row "bitmap" w
```

Repeated `image frame` statements for a channel append animation frames. Optional
`image reference "host-asset-id"` preserves a host link. The image data roundtrips
through Rust format writing, editable bundles and the Swift authoring model.
