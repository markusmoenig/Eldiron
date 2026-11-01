# Procedural sword icon experiment

A fixed **32×32**, transparent sword built with the existing MaterialZ shape and
row system. Steel/Blue, Bronze/Red and Iron/Green use the same graph with different
palette-index bindings, matching the shield experiment's three styles.

![Enlarged and actual-size swords](../../../../notes/experiments/procedural-icons/sword/sword-variants.png)

## Editable assets

- `sword.material`: complete PBR recipe using stock material tiles.
- `*.palette.json`: authored Eldiron palette indices for the blade, groove,
  crossguard, grip, pommel, outlines and inlay.
- `../../../../notes/experiments/procedural-icons/sword/`: 32px PNGs, comparison
  sheet, and standalone `.materialz` exports which can be opened in MaterialZ.

The standalone exports contain resolved linear RGB. The source palette files
retain the authoritative indices. Palette variants reuse one compiled shader.

## Construction

1. A triangle (Polygon with three sides) intersects a narrow Rounded Rectangle,
   making a sharp tip and straight blade sides. Directional Gradient → Color Ramp
   shades the blade across its width, and Border supplies the outline.
2. A thin rounded rectangle supplies the central blade groove.
3. A rounded bar unions with two circles to form the crossguard and its finials.
4. A separate rounded-rectangle grip mask feeds Coordinates → Vector Math (Dot)
   → Multiply → Fract → Color Ramp, producing diagonal leather wrapping while
   retaining the grip's transparent silhouette.
5. A shaded circle forms the pommel; a smaller palette-colored circle is its inlay.
6. Alpha Over rows assemble the parts. One final Transform poses the
   sword at 45 degrees and scales it by 1.14. Compare/Select makes the output
   opacity binary for a clean pixel silhouette.

Blade width, tip, guard proportions, grip wraps, pose, and palette choices remain
editable. The icon bake uses 32px independently of world-material pixelization.
Directional Gradient and Alpha Over are general tiles in both material cores;
this recipe contains no custom tile algorithms.

## Render

From the repository root on a machine with a native GPU:

```sh
cargo test -p eldiron-creator --lib sword_icon_experiment -- --ignored --nocapture
```

Output defaults to `/tmp/eldiron-sword-experiment`; `ELDIRON_ICON_OUTPUT` overrides
that folder. The render harness also writes Eldiron editable bundles containing
the palette bindings. CPU checks cover save/reopen, the point and guard, continuous
coverage through the blade and handle, and transparent corners. GPU checks cover
32px RGBA output, binary alpha, distinct variants and shader reuse.

These assets are an experiment and do not replace live ruleset icons or create
Tilepicker entries automatically.
