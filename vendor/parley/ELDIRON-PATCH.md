# Eldiron text-analysis patch

Vendored upstream Parley **0.9.0**, from the crates.io source distribution.
Repository: https://github.com/linebender/parley
Upstream package checksum: `8fad031076f48f0d4d85ce1aea9b94b4e715a4d636a030a123038f8f5b5e4343`.
Upstream VCS revision: `65fd4ad705deabc89b732cf7e8de9245466a2450`.
Upstream Apache-2.0 / MIT licenses are retained.

The production change is confined to `src/analysis/mod.rs`. The complete diff
is in `eldiron-complex-segmentation.patch` for review/rebasing.

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
