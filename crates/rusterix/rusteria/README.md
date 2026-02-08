# Rusteria

Rusteria is a fast, shader-like programming language designed for procedural texture and material generation. It compiles to a stack-based bytecode VM and executes shade functions in parallel using [rayon](https://crates.io/crates/rayon).

Rusteria is part of the [Eldiron](https://github.com/markusmoenig/Eldiron) project, where it drives procedural material generation for the game engine's rendering pipeline.

| Marble | Wood | Wood Ring |
|--------|------|-----------|
| ![Marble](https://raw.githubusercontent.com/markusmoenig/Eldiron/master/crates/rusterix/rusteria/examples/marble.png) | ![Wood](https://raw.githubusercontent.com/markusmoenig/Eldiron/master/crates/rusterix/rusteria/examples/wood.png) | ![Wood Ring](https://raw.githubusercontent.com/markusmoenig/Eldiron/master/crates/rusterix/rusteria/examples/wood_ring.png) |

## Features

- **C-like syntax** with `let`, `fn`, `if/else`, `for` loops, and user-defined functions (including recursion)
- **GLSL-inspired builtins** -- `sin`, `cos`, `mix`, `smoothstep`, `dot`, `cross`, `normalize`, `clamp`, `fract`, `mod`, and many more
- **Vec2/Vec3 constructors** -- `vec2(x, y)` and `vec3(x, y, z)`
- **Swizzling** -- `v.x`, `v.y`, `v.xy`, `v.yx`, etc.
- **Built-in noise textures** -- `sample(uv, "perlin")`, `sample(uv, "fbm_perlin")`, `sample(uv, "value")`, `sample(uv, "fbm_value")`, `sample(uv, "bricks")`, `sample(uv, "tiles")`, `sample(uv, "blocks")`
- **Material outputs** -- write to `color`, `roughness`, `metallic`, `emissive`, `opacity`, `bump`, `normal` directly
- **Parallel shading** -- the `shade()` function is executed per-pixel across tiles using rayon
- **Bytecode compiler with optimizer** -- compiles to a compact `NodeOp` instruction set

## Quick Start

```rust
use rusteria::*;
use std::sync::{Arc, Mutex};

let mut r = Rusteria::new();
let palette = r.create_default_palette();

// Parse, compile, and shade in one call
let source = r#"
fn shade() {
    let n = sample(uv * 4.0, "fbm_perlin");
    color = mix(vec3(0.2, 0.1, 0.0), vec3(0.9, 0.85, 0.7), n);
    roughness = 0.5 + 0.3 * n;
}
"#;

let module = r.parse_str(source).unwrap();
r.compile(&module).unwrap();

let shade_index = r.context.program.user_functions_name_map["shade"];
let mut buffer = Arc::new(Mutex::new(RenderBuffer::new(512, 512)));
r.shade(&mut buffer, shade_index, &palette);

buffer.lock().unwrap().save("output.png".into());
```

## Example: Procedural Marble

```rusteria
fn shade() {
    let uv2 = uv * 1.0;

    let n1 = sample(uv2, "fbm_perlin");
    let n2 = sample(uv2 * 2.0, "fbm_perlin");
    let turb = 0.6 * n1 + 0.4 * n2;

    let bands = uv2.x + turb * 0.6;
    let s = sin(bands * 8.0);
    let veins = pow(1.0 - abs(s), 3.0);

    let base_col = vec3(0.92, 0.93, 0.96);
    let vein_col = vec3(0.18, 0.20, 0.24);
    color = mix(base_col, vein_col, veins);

    let m = sample(uv2 * 0.5, "value");
    color *= (0.9 + 0.1 * m);
}
```

## Running Examples

The crate includes example scripts with reference images in the `examples/` directory.

```bash
# Render a script to PNG (default 512x512)
cargo run --example shade -- examples/marble.rusteria

# Custom output path and resolution
cargo run --example shade -- examples/wood.rusteria wood.png 1024 1024

# Release mode for faster rendering
cargo run --release --example shade -- examples/wood_ring.rusteria
```

## Builtin Functions

### Math
`abs`, `floor`, `ceil`, `round`, `fract`, `mod`, `sqrt`, `log`, `pow`, `min`, `max`, `clamp`, `mix`, `step`, `smoothstep`

### Trigonometry
`sin`, `cos`, `tan`, `atan`, `atan2`, `radians`, `degrees`, `rotate2d`

### Vector
`length`, `length2`, `length3`, `normalize`, `dot`, `dot2`, `dot3`, `cross`

### Texture Sampling
`sample(uv, pattern)` -- samples a built-in noise/pattern texture at the given UV coordinates.

Available patterns: `"value"`, `"fbm_value"`, `"perlin"`, `"fbm_perlin"`, `"bricks"`, `"tiles"`, `"blocks"`

### Constructors
`vec2(x, y)`, `vec3(x, y, z)`

## Material Outputs

Inside a `shade()` function, write directly to these built-in variables:

| Variable    | Type  | Description                     |
|-------------|-------|---------------------------------|
| `color`     | vec3  | Surface albedo color            |
| `roughness` | float | Surface roughness (0.0 - 1.0)  |
| `metallic`  | float | Metallic factor (0.0 - 1.0)    |
| `emissive`  | float | Emissive intensity              |
| `opacity`   | float | Surface opacity (0.0 - 1.0)    |
| `bump`      | float | Bump/height offset              |
| `normal`    | vec3  | Surface normal override         |

Read-only inputs: `uv` (vec2, normalized texture coordinates), `time` (float, animation time).

## License

Dual-licensed under Apache-2.0 and MIT.
