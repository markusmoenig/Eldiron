# Complete material mixing

A material may evaluate a mixer over other complete materials. Inputs refer to
stable material IDs, independent of labels and canvas positions. Mixers can feed
other mixers, materials can be reused, and cycles are rejected before rendering.
Unconnected inputs use the standard surface defaults. Every BSDF surface field
participates, including opacity, height, normal map and advanced lobes. Distance
remains a row capability, rather than a material channel passed between mixers.

```text
material "result" called "Paint over metal"
  mix "Mask"
  mixing input 1 from material "metal"
  mixing input 2 from material "paint"
  mixing mask from row "mask" xyz
  mixing blend true
  row "mask"
    use "Perlin Noise"
```

Mask interpolates base color with the RGB mask, and other properties with its X
component, following the curated shader. Exact black selects Material 1; exact
white selects Material 2. `mixing mask to 0.5 0.5 0.5` supplies a constant instead
of a row. With Blend off, intermediate values become transparent. The original
shader used negative bump as an invisible sentinel; the new engine uses explicit
zero opacity and otherwise takes Material 1 at these points.

Bump Height selects the complete material with greater Height per UV point.
Ties choose Material 1. Its historical name is retained, but it uses the unified
Height channel and works with either preview height mode.

Overlay selects the overlay where its average RGB base color exceeds Level.
Where selected, the result adds base and overlay heights; all other fields come
from the overlay. Otherwise it uses the base. The level is the first value of
`mixing levels 0 0.5 0.75`; the unused values remain stored for stable editing.

UV Position selects up to four materials using the original UV + 0.5 coordinate,
`mixing direction x` (or y), and `mixing levels 0.25 0.5 0.75`. Each boundary uses
<= and is checked in order; the coordinate is not implicitly repeated.

Explicit local row outputs override individual mixed channels. Mixed channels
otherwise come from the input materials, including their unconnected defaults;
local fallback defaults do not replace the mixer result.

The fixed 28-point mixing strip on the right contains material input terminals.
Mix operations do not create canvas widgets. Use the canvas toolbar Mix menu to
add an operation. Each material has one whole-material output at the right edge
of its header. Drag that output into a mix input. Select a terminal to show the
mix controls in Properties; right-click to disconnect. Mask row selection lives
in Properties, without an extra row-to-mixer wire. `mixing mask-material "metal"`
identifies the material that owns the mask row.

There is no separate Result terminal or output-to-Result loop. A single material
previews automatically. With multiple materials the mixed result is selected
implicitly; `result from material "stable-id"` persists the chosen result in the
project format. Mixer output terminals are exposed only when multiple mixers
need nesting or reuse. Connections and controls support undo/redo.

The compiler resolves the material dependency DAG and lowers each mixed channel
to typed IR nodes before normal cycle and complexity validation. CPU evaluation,
Metal generation, live parameter buffers and the C ABI use the same lowering.
Mixing does not bypass node, depth, expanded-call or operation limits. Automatic
pass splitting and CPU fallback are still future work.

## Editor terminal stack

The right edge of the material canvas always shows four fixed terminals. Drag a
whole-material output onto one. The first connected terminal is the base; each
connected terminal below combines with the preceding result. Gaps are skipped.
Click a terminal for its material, Blend/Mask amount or row mask, Height mode,
or Overlay level. Right-click it to disconnect. With no terminal connected the
preview follows the selected material. Selecting a terminal does not replace the
selected material or edit the project.

Stack mixer records carry `stack slot 1` through `stack slot 4`. Slots are unique,
are retained across save/load, and use the existing whole-material mixer IR.
The editor stores the chain as explicit material inputs and the final `result`;
the first connection is passed through without mixing with a default material.
