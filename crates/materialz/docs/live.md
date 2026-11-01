# Live editing and the Swift bridge

## Document ownership

`live::Document` owns a parsed document and its validated `Program`.
`set_constant(material_id, row_id, tile_index, parameter_id, values)` updates one
constant, including an implicit default. The tile index is zero-based within the
row. IDs locate the computation independently of display labels.

Every edit validates on a candidate copy before committing. Wrong types, unknown
IDs, nonfinite numbers, singular divisors, and invalid booleans are rejected.
The original project and evaluator remain unchanged on error. A connected
parameter cannot be implicitly disconnected by setting a constant.

`Document::text` / `dm_project_text` serialize committed values in canonical format
1. Reloading that text produces the edited material. As before, comments and
original whitespace are not retained. Structural edits currently require editing
and recompiling project text; there is no mutation API for connections yet.

## Buffered shader contract

`metal::compile` / `dm_shader_compile` creates an owned shader package:

- Generated Metal source for one material channel.
- An ordered layout of reachable constant parameters, including implicit defaults.
- Packed current values, one 16-byte float4 per parameter.

All generated functions receive the same constant parameter array. Scalars occupy
`.x`, vector2 `.xy`, vector3 `.xyz`, and colors `.xyzw`. Unused components are zero.
Booleans occupy `.x` as 0 or 1 and load through an explicit comparison with zero.
The fixed stride avoids differences between Rust, Swift, and Metal struct padding.

The compute function `material_preview` binds:

| Index | Data |
| --- | --- |
| 0 | Output float4 pixels, width × height × 16 bytes |
| 1 | Dimensions as two UInt32 values |
| 2 | Constant parameter float4 array from the package |

`parameter_bytes()` / CLI `.params` serializes little-endian float32. The C bridge
copies native float values to caller-owned memory; Apple platforms use the same
byte ordering. Bind the array at a valid Metal buffer offset (zero is suitable).
Keep buffers alive until their command buffers complete, and avoid overwriting
parameter data that an in-flight frame is still reading.

After a successful edit, refresh the shader package and upload its parameter data.
The generated source and layout remain identical, so the existing Metal pipeline
can be reused. A mapping, connection, kernel, or parameter-type change requires a
new package/pipeline. `refresh` detects structure/layout changes and returns an
error without changing the existing package.

The initial implementation re-lowers the small document on edit and regenerates
source on the CPU to check compatibility on refresh. It avoids GPU recompilation;
it does not yet implement incremental compilation, dirty-output tracking, or
automatic buffer uploads. Each shader snapshot refreshes explicitly. Other channel
snapshots retain their old values until refreshed, so an app must coordinate its
own frame updates.

`metal::generate` / `dm_metal` emit standalone diagnostic source with embedded
constants. Such source does not require buffer(2).

## JSON schemas

`metadata::catalog_json` / `dm_catalog_json` / `materials catalog` returns:

```json
{
  "version": 1,
  "sources": [{
    "source_id": "perlin",
    "help_key": "tile.perlin",
    "name": "Perlin Noise",
    "category": "Noise",
    "description": "Perlin noise with 2D/3D sampling.",
    "references": "original reference text",
    "requires_input": false,
    "groups": [{"label": "Perlin Noise", "expanded": true}],
    "parameters": [{
      "id": "globalScale", "label": "Global Scale",
      "group": "Perlin Noise", "type": "float",
      "default": [1], "range": [0.001, 10], "tooltip": null
    }]
  }]
}
```

Types are `float`, `vec2`, `vec3`, `color`, and `bool`. The catalog also supplies
`creates_distance` and `shape_combinations` for native shape controls.
Defaults are numeric arrays;
boolean arrays contain 0 or 1. Range and tooltip can be null. Group order and
parameter order follow the original shader metadata. Descriptions/reference text
are preserved data; some original reference text contains HTML.

`Shader::layout_json` / `dm_shader_layout_json` returns:

```json
{
  "version": 1, "material": "demo", "channel": "base color",
  "buffer_index": 2, "stride_bytes": 16, "float_count": 4,
  "slots": [{
    "row": "shape", "tile": 1, "parameter": "globalValue",
    "type": "float", "index": 0, "byte_offset": 0, "width": 1
  }]
}
```

This illustrative layout has one parameter; real layouts include all constants
reachable from the selected output. `index` addresses float4 elements, not floats;
`float_count` includes padding. A parameter controlled by a row has no constant
slot of its own; its dependency's constants appear instead. Check schema version
before decoding. A `preview` object identifies row/tile/mode for thumbnail shaders
and is null for ordinary material outputs. `examples/swift-smoke.swift` provides
concrete Codable types, including the distance-capable source definitions.

## C/Swift lifetime rules

1. `dm_compile` creates a document handle. `dm_shader_compile` takes a shared read
   of it and produces an independent shader handle.
2. Obtain owned source/layout strings, create the Metal pipeline, and copy initial
   parameter data using `dm_shader_float_count` / `dm_shader_copy_parameters`.
3. Apply edits with `dm_set_constant`, refresh via `dm_shader_refresh`, and copy
   updated values into a safe per-frame GPU buffer.
4. Retrieve canonical project text when saving.
5. Free owned strings with `dm_string_free`, shaders with `dm_shader_free`, and
   documents with `dm_program_free`. A shader does not borrow its document.

Mutations and destruction require exclusive access to the affected handle. Shared
read operations can run concurrently. An operation initializes the optional error
pointer to null, or supplies an owned diagnostic on failure; free old diagnostics
before reusing the pointer storage. Null handles are diagnosed where operations
return errors. Freeing null handles/strings is permitted. Dangling pointers,
double frees, and insufficiently allocated C buffers remain caller errors.

Build the experimental static library and its clients together.

Source catalog entries also expose `output_channels` (component/label pairs) and
optional `default_channel`. Connections store the chosen swizzle, e.g. `yyy` for
Voronoi nearest distance expanded into an RGB input.

`source_id` and `help_key` are stable resource identities, independent of display
names and generated library indices. The host resolves localized help; help text
does not participate in procedural evaluation.
