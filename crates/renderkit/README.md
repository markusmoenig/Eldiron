# Renderkit

Explicit GPU context, resource-domain image registry, surfaces and opt-in capture.
There is no application event loop, widget tree or gameplay state here.

`ImageRegistry::upload_linear` prepares decoded/authored RGBA8 source assets as
RGBA16F **linear premultiplied** textures. Color decoding and premultiplication
happen once when a source changes, before GPU interpolation. Transparent source
edges therefore interpolate coverage and radiance correctly. CPU conversion is
source-asset preparation; it never draws a UI, scene or window framebuffer.

For RGBA8 source data, `AlphaMode::Premultiplied` means multiplication in the
tagged source color space. Encoded sRGB premultiplication is removed before
linearization and applied again in linear light. Zero-alpha colors become zero;
`Opaque` overrides stored alpha. Swash outline masks, layered color outlines and
bitmap glyph sources are normalized before atlas upload using this same contract.

`upload` retains a source's raw RGBA8 representation for callers requiring byte
identity, including source capture. Source caches used by command UI use
`upload_linear`. Explicit capture currently supports RGBA8/BGRA8 targets;
RGBA16F export and normalization of externally rendered SceneVM outputs remain
separate integration work. CPU asset normalization does not normalize scene output.

GPU images are owned by one context generation and registered in one registry
owner domain. Generational handles reject removed/replaced registrations. Normal
render targets allocate no CPU pixels or readback staging; capture allocates those
only when requested. Rust surfaces can wrap host-retained Apple CAMetalLayer
handles as well as owned Winit windows/canvases.
