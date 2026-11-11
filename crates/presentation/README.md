# Eldiron presentation

`SceneViews` owns the GPU presentation of application views. Construct it with
the host/presenter's image registry and a shared source atlas. It creates no
window, surface, server or simulation clock.

Each `ViewId` has a separate `SceneHandler`/SceneVM, camera/uniform state,
history and render target. `scene_mut` marks only that view dirty. The caller
prepares scene commands and calls `render` with its content revision. An
unchanged view reuses its output; a rendered view advances its output revision.
The returned `ViewImage` contains an opaque image handle and descriptor for UI
composition. Its registry/device domain is checked before rendering.

Sizes are physical pixels. A zero size suspends a view and returns no output;
resuming at the retained size reuses its target. Resizing replaces the target
and invalidates the old handle. Call `remove` or `clear` with the host registry
when closing views or switching projects. The registry owns registrations, so
this explicit release is required while retaining the host.

`capture` is the explicit staging/readback path. It returns a ticket stamped
with the view, output revision and content revision. A pending capture still
refers to the requested frame if its view subsequently resizes or is removed.
Normal targets, intermediate layer textures and ping-pong history have no CPU
pixel storage.

Recreate this GPU owner after device replacement from retained application
controllers/documents and re-prepare scenes. CPU atlas sources can be retained:
the atlas rebinds using the explicit renderkit generation. Device equality is
not used as a cross-instance ownership identity. Mip pipelines live with the
textures they serve, rather than in a process/thread-global cache.

This is the initial scene-view boundary. Creator and graphical player entry
points still need adoption; `screen::ScreenPainter` now emits text, stat, text-input, choice, decoration
and authored artwork commands from CPU controller state. Other screen families
and the complete HUD composition/controller extraction are still being migrated. Opaque raster-2D composition, independent transforms, resource domains,
resize/suspend and capture are tested. Transparent scene color/alpha contracts,
3D/isometric application parity, shared geometry buffers, picking revisions and
full application device-loss recovery remain acceptance work.

`ScreenImageCache` registers authored/generated source images in the host registry.
It fingerprints sources to reuse unchanged uploads, retains same-frame revisions,
and evicts unused entries to its byte budget after frame assembly. Current-frame
working sets remain live even when larger than the budget; resident bytes expose
that case. Discard old command lists before beginning another cache frame, and
call `clear` before replacing this owner while retaining its registry.

The screen painter owns neither a destination raster nor a simulation clock.
Content/placeholder resolution and stat/choice geometry are shared with the CPU
controller adapters. Fonts can be registered under project asset names with
`TextSystem::register_font_named`; all text still uses Parley and Swash.

`configure_game` installs the engine program/configuration and source atlas on a
created view. `prepare_game` consumes a borrowed `GameSceneInput` and the game
widget's camera/controller, sizes the GPU target and prepares its scene without
CPU output/upscale buffers. Resolution reduction preserves logical 2D framing.
It suspends zero-sized views. Text and isometric game presentation are currently
explicit unsupported paths here, pending their dedicated command/sprite adapters;
there is no implicit raster fallback.

`ScreenPainter::unlit_scene` transfers prepared 2D batches to backend-neutral
textured meshes. It preserves authored UVs, transforms, concave triangulation,
per-axis repeating and time-major spatial pages; page boundaries clip geometry
in UV space. It does not use the software rasterizer or allocate projected
pixel/depth output. Lit/3D scenes and line primitives currently require dedicated
scene/path adapters and are explicitly rejected by this method.

Screen source images now use RGBA16F linear-premultiplied upload normalization.
The screen cache byte budget counts GPU texel storage (eight bytes per pixel),
including superseded revisions retained for the current frame. Normalization is
performed only for changed source assets; unchanged frames reuse registrations.
