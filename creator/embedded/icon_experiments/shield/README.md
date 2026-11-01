# Procedural shield icon experiment

A fixed **32×32**, transparent, pixel-art round shield made with the existing
MaterialZ core. Steel/Blue, Bronze/Red and Iron/Green share one graph and use
Eldiron palette indices for every authored color. The world material pixelization
setting is independent of this icon bake.

![Rendered variants, enlarged and actual size](../../../../notes/experiments/procedural-icons/shield-variants.png)

## Editable sources

- `shield.material`: complete PBR row recipe using stock material tiles.
- `steel-blue.palette.json`, `bronze-red.palette.json`, `iron-green.palette.json`:
  palette-index bindings for the rim, face, emblem, boss, outlines and rivets.
- `../../../../notes/experiments/procedural-icons/`: actual 32px PNGs, a
  nearest-neighbor comparison sheet, and standalone `.materialz` exports which
  can be opened in MaterialZ.

The `.materialz` exports contain resolved linear RGB for standalone MaterialZ.
The editable Eldiron sources above retain the palette indices. Changing a color
variant updates GPU parameters and reuses the compiled shader pipeline.

## Shape and composition workflow

The rows execute from top to bottom:

1. Circle → Directional Gradient → Color Ramp → Border: the metal rim.
2. The same stock tile chain with different palette tones: the painted face.
3. Two rounded rectangles combined with Union: the raised cross.
4. Circle → Directional Gradient → Color Ramp → Border: the central boss.
5. One Circle → Color → Border → Transform: the reusable rivet.
6. Grid, with **Layout = Rings**: radial repetition of that rivet.
7. Alpha Over rows: face, emblem, boss and rivets over the rim.
8. Compare/Select: a binary opacity mask for the final pixel silhouette.

Directional Gradient supplies the upper-left lighting direction to a normal
Color Ramp. Alpha Over accepts a foreground RGBA row over its background input.
Compare/Select and Compose Channels harden the rivets' opacity. Every operation
is a stock tile available to any material in Eldiron and standalone MaterialZ;
the recipe contains no custom tile algorithms.

## Radial repetition

Radial repetition already exists in **Grid → Rings**. For this shield, Cells is
`5, 1`, Cell Offset is `0, 0.140625`, and Gap is zero. This puts **16 rivets** on
one ring, with a radius of `0.359375` (11.5 icon pixels). The template Transform
compensates for the different tangential and radial dimensions of a polar cell.
Pixel-mask thresholding keeps the small rivets readable at 32px.

The current Rings controls describe grid density. A dedicated Count/Radius
repeater would make this particular authoring task easier. The experiment uses
the existing Grid tile for radial repetition.

## Reproduce

From the Eldiron repository root, on a machine with a native GPU:

```sh
cargo test -p eldiron-creator --lib shield_icon_experiment -- --ignored --nocapture
```

The renderer writes `/tmp/eldiron-shield-experiment` by default. Set
`ELDIRON_ICON_OUTPUT` to choose another folder. It exports transparent 32px PNGs,
a comparison sheet, standalone MaterialZ recipes, and Eldiron editable bundles
which preserve palette bindings.

The CPU check validates palette-bound save/reopen, all 16 rivet centers, and the
transparent silhouette. The GPU check validates 32px RGBA output, binary alpha,
distinct palette variants, and shader reuse.

These are experiment assets, referenced explicitly by the render harness. They
are not automatically installed as world tiles or substituted for ruleset icons.
