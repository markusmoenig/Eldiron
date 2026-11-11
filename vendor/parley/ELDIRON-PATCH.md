# Eldiron text-analysis patch

Vendored upstream Parley **0.9.0**, from the crates.io source distribution.
Repository: https://github.com/linebender/parley
Upstream package checksum: `8fad031076f48f0d4d85ce1aea9b94b4e715a4d636a030a123038f8f5b5e4343`.
Upstream VCS revision: `65fd4ad705deabc89b732cf7e8de9245466a2450`.
Upstream Apache-2.0 / MIT licenses are retained.

The dictionary production change is confined to `src/analysis/mod.rs`; its diff
is in `eldiron-complex-segmentation.patch`. The additional editing API adapters
below are isolated in `eldiron-editor-selection.patch` and
`eldiron-editor-glyph-clusters.patch` for review/rebasing.

A test-only `eldiron-segmentation-tests` feature in the manifest/lib harness
isolates the self-contained dictionary contract test. The published package's
other unit tests reference the unpublished workspace helper `parley_dev`; they
cannot link from the crates.io bundle. This feature does not change production
behavior, and is not enabled by Eldiron. Full upstream fixture tests are not
claimed as passed. Run the scoped regression with:

```sh
CARGO_TARGET_DIR=/path/to/target cargo test --offline --manifest-path vendor/parley/Cargo.toml --lib --features eldiron-segmentation-tests eldiron_dictionary
```

Parley's default analyzer explicitly selects ICU's non-complex-script-only
segmenters. Japanese input emitted `No segmentation model for language: ja`
and lacked dictionary word boundaries. Eldiron caches ICU's baked dictionary
word/line segmenters per layout context instead. These include Chinese/Japanese
and Southeast Asian script data, using the existing ICU `compiled_data`
dependency. No rasterization, shaping, font selection or Swash code is replaced.

The root Cargo patch pins this source. Dependency tests exercise dictionary word/line boundaries directly for
Japanese and Thai; native transcript/choice tests exercise the same
provider through actual shaped text and GPU paint. Parley cursor movement
also treats line-break opportunities as navigation stops; this patch does not
redefine those editing semantics. Remove this local patch when
an upstream release provides equivalent configurable/full segmentation.

## Editing selection API

Two public adapter methods in `src/editing/{editor,selection}.rs` allow the
framework to retain Parley's selection state rather than reducing it to byte
indices. `PlainEditorDriver::set_selection` validates UTF-8 endpoints, refreshes
the selection against the current layout and uses the editor's existing
selection/generation update path. `Selection::map_cursors` retains bidi affinity,
preferred horizontal position and word/line drag anchors when applying the
framework's extended-grapheme cursor policy. Existing Parley methods are unchanged.

The framework, not this patch, filters visual shaping-cluster stops through
Unicode extended-grapheme boundaries and snaps pointer/vertical endpoints to the
nearest valid shaped caret. Undo/IME cancellation restores the full selection,
including which visual side of a bidi boundary it occupied. Actual regressions
reproduced interior Tamil/combining stops and a bidi undo caret jump before these
adapters were added. The framework's text-only controller tests verify them; the
scoped upstream dictionary test command above still covers only analysis data.
Remove these API extensions when an upstream public API provides equivalent
selection restoration/mapping. No font rasterizer, glyph generation or shaping
algorithm is replaced.

## Paint-only editor glyph colors

`GlyphRun::positioned_glyphs_with_clusters` in `src/layout/line.rs` exposes the
existing fully positioned glyph iterator with each glyph's original UTF-8
cluster range. It uses exactly the same visual-cluster traversal, style slice,
offset and baseline as `positioned_glyphs`; shaping and line breaking are
unchanged. Its isolated diff is `eldiron-editor-glyph-clusters.patch`.

The framework uses these ranges to recolor already shaped glyphs for native
code-editor syntax/diagnostics. It never lays out a second highlighted version
for painting, so editing carets, bidi order, preedit, wrapping and glyph positions
stay aligned. A paint span intersecting part of one shaped cluster colors that
cluster as a unit. Overlap resolution honors the last span and uses normalized
intervals/binary lookup rather than scanning every syntax token for every glyph.
Framework text-only and headless GPU tests cover position/identity preservation,
overlapping colors, composition, cache reuse and actual colored output. Replace
this extension when an upstream public API supplies equivalent cluster-addressed
positioned glyph iteration. Upstream's full unpublished fixture suite is still
not claimed as passed.
