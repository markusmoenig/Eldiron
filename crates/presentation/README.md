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
points still need adoption; game-screen/HUD emission is not implemented here
yet. Opaque raster-2D composition, independent transforms, resource domains,
resize/suspend and capture are tested. Transparent scene color/alpha contracts,
3D/isometric application parity, shared geometry buffers, picking revisions and
full application device-loss recovery remain acceptance work.
