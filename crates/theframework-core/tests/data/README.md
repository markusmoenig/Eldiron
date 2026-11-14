# Pre-extraction data fixtures

Captured from `77b3625d` TheFramework definitions before moving bitmap/value types.
`legacy-values.json` and `legacy-values.bin` contain all value variants, key codes,
comparison/assignment operators, bitmap/region/tile data, UTF-8 and fractional/unit
density images. JSON round-trips all 52 values. Binary serialization is byte exact.

The original bincode decoder fails with UnexpectedEof for the mixed-density record:
the legacy bitmap omits unit `render_scale` while bincode struct decoding expects
that field. An isolated harness built from the original git definitions reproduced
this before the migration (`/private/tmp/eldiron-core-pre-move-readable.log`). This
extraction preserves the existing format; it does not silently change it to fix
that pre-existing binary-format ambiguity. `legacy-readable-values.bin` contains
51 values and fractional-density bitmaps with all fields present; the original
reader successfully round-tripped it. Both serialization and decoding are checked.

`legacy-bitmap-text.png` captures explicit CPU reference text/alpha at 150% density
using the existing Roboto-Bold font. Native text remains Parley/Swash; the fontdue
reference adapter stays outside the CPU core.

`legacy-rgb.json`/`legacy-rgb.bin` were independently captured from the original
77b3625d three-channel bitmap definition and round-trip through both codecs.
