---
title: "Language Fundamentals"
sidebar_position: 3
---

Recipe files are indentation-based documents made from declarations, named blocks, fields, and scalar expressions. They are data, not unrestricted scripts: recipes cannot loop arbitrarily, access the filesystem, or mutate gameplay state.

## Document kinds

A document contains one of the following root kinds:

- one `Tile` declaration;
- one or more `Material <id>` declarations; or
- one or more `Sdf <id>` declarations.

Root kinds cannot be mixed in one file. Keeping them separate lets each consumer request a clear output contract while sharing the same procedural vocabulary.

## Syntax rules

- Use the `.recipe` extension.
- Indent blocks with spaces. Tabs are rejected.
- `//` starts a line comment.
- Block and field names are case-insensitive.
- Strings use double quotes.
- Booleans are `true` and `false`.
- `I2(x, y)` is an integer pair and `F2(x, y)` is a floating-point pair.
- Unknown fields, unknown references, duplicate definitions, and cyclic scalar references are errors.

```recipe
// One named field feeding another block.
Noise Grain
  type = Gradient
  scale = F2(8.0, 8.0)

Value Face
  source = Smoothstep(0.2, 0.8, Grain)
```

## Scalar expressions

Scalar fields are shared by Tile height, pattern masks, material channels, and color mixing.

```recipe
source = Clamp(Stones.height * 0.85 + Grain * 0.15, 0.0, 1.0)
```

Expressions support:

- literals, parentheses, and `+`, `-`, `*`, `/`;
- named `Noise`, `Height`, and `Value` fields;
- pattern channels such as `Stones.height`, `Stones.edge`, and `Stones.center`;
- normalized coordinates `U`, `V`, `Radius`, and `Angle`;
- functions including `Abs`, `Invert`, `Sin`, `Cos`, `Fract`, `Sqrt`, `Min`, `Max`, `Pow`, `Clamp`, `Mix`, `Smoothstep`, `Random`, and `Wave`.

Final output channels are clamped to `0..1`. Division by zero evaluates to zero.

## Noise and patterns

`Noise` creates reusable scalar variation. Its root seed, local seed, scale, fractal, octave count, and persistence make the result deterministic.

```recipe
Noise Wear
  type = Gradient
  fractal = Ridged
  scale = F2(9.0, 9.0)
  octaves = 3
  persistence = 0.48
  seed = 3
```

Implemented noise kinds are `Value` and `Gradient`; `Perlin` aliases `Gradient`. Implemented fractals are `FBm`, `Ridged`, `Billow`, and `Turbulence`.

`Pattern` divides a domain into stable units. Implemented generators are `Bricks`, `Voronoi`, and `Discs`. They expose `.height`, `.edge`, `.center`, and a stable `.id` for keyed variation.

## Coordinate domains

Recipes normally evaluate in normalized `U` and `V` coordinates. A consumer maps its surface into that domain.

- `Global` evaluates continuously over the complete target.
- `<Pattern>.local` restarts coordinates within every unit of that pattern.
- Avatar bindings additionally choose how the consumer constructs the target surface before the material is evaluated.

Domain selection is what lets a stone vary independently per Voronoi cell or a wood material restart inside every plank without hard-coding a special stone or wood shader.

## Wrapping and tiling

Root `wrap` accepts:

| Mode | Behavior |
| --- | --- |
| `Clamp` | Hold sampling at the closest outer edge. |
| `Repeat` | Wrap both axes seamlessly. |
| `Mirror` | Reflect alternate repetitions. |

Consumer bindings may also provide a `tiling` value. Tiling scales coordinates before the recipe's wrapping mode is applied. Values above `1` repeat more often; positive values below `1` enlarge the authored repeat.

## Determinism

A recipe produces the same result for the same source, palette, root seed, render options, and consumer mapping. Use:

- the root `seed` for the overall result;
- per-block `seed` values to decorrelate operations;
- stable pattern `.id` values with `Random` or `key`;
- consumer-provided seed offsets for stable per-item variants.

Do not derive important variation from frame order or wall-clock time. Animation is explicit through an `Animation` block and `Wave(...)`, or through a consumer-provided evaluation time.

## Diagnostics

Parser errors include stable codes such as `PR0001` for syntax, `PR0004` for duplicates, and `PR0008` for unknown references. Tooling should match the structured code rather than parsing the explanatory message.

The exhaustive block-by-block reference currently remains in `crates/procedural_recipes/README.md`. It will be migrated into this chapter as the public language stabilizes.
