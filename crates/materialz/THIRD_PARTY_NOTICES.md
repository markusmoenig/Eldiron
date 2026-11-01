# Third-party code

The shared Rust engine and MaterialZ app are accompanied by these separately
attributed sources:

- Curated Material-Z shader sources and Brian Sharpe's Wombat Perlin2D derivative:
  see `shaders/PROVENANCE.md` and the curated files in `shaders/original/`.
- The locally owned Metal renderer module in `apple/Packages/MaterialZRenderer`
  was copied from the user's DenrimRendererKit project on 2026-10-27. It retains
  the Apache License 2.0 in that directory's LICENSE. Local changes are described
  in LOCAL_CHANGES.md. No upstream CLI assets, example images or scene assets
  are distributed with this copy.
