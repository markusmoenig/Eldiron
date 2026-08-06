---
title: "Getting Started"
sidebar_position: 2
---

Recipes are named project or ruleset assets. Consumers resolve them through a shared recipe catalog, so the same Material or shape can be selected by Tiles, Avatar equipment, and future Creator tools.

## Choose an authoring surface

- **Eldiron Creator:** Recipes will appear as editable project assets with visual controls, live consumer previews, and an advanced text view. Until that integration is complete, use the canonical text workflow below.
- **Canonical text:** Save `.recipe` documents in a project's recipe catalog. Eldiron Source projects currently discover them recursively inside `recipes/`.
- **Command line:** Use `procedural-recipes` to validate and render previews from the same documents without building or running a game.

The examples in this chapter use canonical text because it is copyable, versionable, and independent of the authoring surface.

## Create a material

Create `recipes/cloth.recipe`:

```recipe
Material cloth
  name = "Red Woven Cloth"
  wrap = Repeat
  seed = 41

  Noise Weave
    type = Gradient
    fractal = FBm
    scale = F2(12.0, 16.0)
    octaves = 2
    persistence = 0.45

  Value Bands
    source = Smoothstep(0.35, 0.65, Abs(Sin(U * 6.0 + V * 2.0)))

  Color Shadow
    exact = #542c35

  Color Highlight
    exact = #b85d58

  Color Cloth
    source = Mix(Shadow, Highlight, Clamp(Bands * 0.8 + Weave * 0.2, 0.0, 1.0))

  Surface
    color = Cloth
    palette = BaseOnly
    roughness = 0.85
    metallic = 0.0
    opacity = 1.0
    emission = 0.0
    normal = Weave
    normal_strength = 0.18
```

A catalog entry containing one material uses its path without `.recipe` as its short alias. The example above is `cloth`. An entry at `wardrobe/cloth.recipe` is `wardrobe/cloth`. In a text project those paths are normally below the project's `recipes/` directory; Creator will manage the same aliases as project assets.

## Use it from a tile

Create `recipes/cloth-panel.recipe`:

```recipe
Tile
  name = "Cloth Panel"
  size = I2(64, 64)
  coverage = I2(1, 1)
  wrap = Repeat
  seed = 7
  material = cloth

  Noise Relief
    type = Gradient
    fractal = FBm
    scale = F2(5.0, 5.0)
    octaves = 3

  Height Surface
    source = Relief

  Output
    height = Surface
```

The tile owns geometric height. The material owns color and renderer surface channels. Keeping those responsibilities separate makes `cloth` reusable on a Tile, Avatar wearable, or future consumer.

## Validate and preview from the CLI

With the `procedural-recipes` command available:

```sh
procedural-recipes validate recipes/cloth.recipe
```

```sh
procedural-recipes render recipes/cloth.recipe \
  --palette palette.hex \
  --output previews/cloth.png
```

Use `--watch` while editing to rerender after each saved change. A palette can be a local `.hex` file or a Lospec slug such as `lospec:resurrect-64`.

CLI rendering is useful even when a recipe is normally edited in Creator: it provides reproducible previews for documentation, automated checks, and comparing palette or seed variants.

## Use recipes in a text project

Eldiron Source currently loads the text catalog and embeds its results during a normal build:

```sh
target/release/eldiron-source build path/to/project
```

Tile recipes are rendered during the build. Material and SDF documents remain in the compiled project so runtime consumers such as Avatar equipment can resolve their aliases. Parse and render failures stop the build and identify the document path, line, column, and stable diagnostic code.

## Alias rules

- Aliases are relative to the recipe catalog root. In an Eldiron Source project this is `recipes/`.
- The `.recipe` extension is omitted.
- Alias matching is case-insensitive at runtime; lowercase aliases are recommended.
- A file with one `Material` or `Sdf` declaration receives the file alias.
- Grouped declarations use `<file-alias>/<declaration-id>`.
- Duplicate aliases are build errors.

For example, a catalog entry `dungeon.recipe` containing `Material stone` and `Material mortar` exports `dungeon/stone` and `dungeon/mortar`.

## Next steps

- Learn the common syntax in [Language Fundamentals](./language.md).
- Build layered Tile surfaces in [Materials and Color](./materials.md).
- Apply a material to equipment in [Avatars and Wearables](./avatars.md).
