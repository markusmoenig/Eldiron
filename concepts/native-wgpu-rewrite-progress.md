# Native WGPU rewrite progress

Started: 2026-11-11 (Asia/Bangkok). Overall Goal: active; full M0–M9 scope remains.

## Baseline and isolation

- Original checkout: `/Users/markusmoenig/Eldiron`, master at `f8d880110366e8638305493ac7c4bc6c58b9313d`.
- Implementation checkout: `/Users/markusmoenig/.codex/worktrees/native-wgpu-rewrite/Eldiron`, branch `codex/native-wgpu-rewrite`.
- Tracked source was clean at startup; the untracked approved rewrite and networking concept documents were copied into this checkout. Original files are untouched.
- Baseline: `cargo check --locked --offline -p eldiron-creator -p eldiron-client -p eldiron-client-wgpu -p eldiron-client-terminal` passed, 26.18 s. Log: `/private/tmp/eldiron-wgpu-baseline-check.log`.
- Toolchain: rustc/cargo 1.99, Xcode installed. Installed targets include native arm64/x86_64 macOS, iOS/device/simulator, wasm32, x86_64 Windows GNU, Android arm64. Runtime/platform validation is not implied by target installation.
- Managed worktree write access requires approved escalated commands; automatic review approved initialization. No permission settings were changed.

## Decisions

- Keep WGPU 29.0.3; inject one explicit GPU context for scenes and presentation.
- Text: Parley 0.9 layout/fallback/bidi plus required Swash 0.2.10 glyph images. These are already available in the local registry; version compatibility will be compiled and checked.
- Paths: Lyon tessellation with GPU rasterization; static Zeno masks remain only an optional asset/reference path. Coverage/AA quality remains a visual verification gate.
- Preserve serialized legacy core values through identity-compatible re-exports; new CPU image data has size/pixels without UI position/display scale.

## Milestone state

- M0: in progress. Baseline compile, explicit ownership/paint/text decisions and GPU visual fixtures exist; release performance baselines and complete workflow inventory remain.
- M1: in progress. Shared GPU context, scene output leases, explicit captures and native/foreign surface presentation work; application multiview/isometric/recovery integration remains.
- M2: in progress. Paint/Swash/Parley pipeline and native/foreign hosts work; batching, resource lifetime budgets and full text/input parity remain.
- M3: in progress. Every active built-in TheWidget implementation now has a command paint path, including procedural controls and the legacy node canvas. Complex Creator integration, full widget catalog parity and legacy adapter retirement remain.
- M4: in progress. CPU schema/identity extraction, shared `GameSession`, CPU startup preparation and terminal adoption are implemented and tested. Graphical/Creator adoption, remaining bitmap/value data extraction and engine-global routing isolation remain.
- M5/M6/M7/M8/M9: application conversion/platform acceptance pending. Browser and iOS framework compile, and detached CAMetalLayer presentation pass; this is not complete application/platform parity.

## Implementation and verification

- Extracted TheColor/ThePalette/TheTime/TheDim into `theframework-core` with existing module compatibility re-exports.
- Added backend-neutral ImageHandle and validated CPU ImageAsset.
- Added persisted-data shape and image storage contract tests. Passed: 2 contract tests.

## Next work

1. Integrate the paint/scene contracts into actual Creator panels and clients; keep unsupported/custom application drawing explicit while converting it.
2. Extract the shared CPU runtime/session and graphical presentation boundaries, then adopt the terminal and graphical launchers.
3. Complete scene/isometric/preview ownership, text/input/platform services, renderer batching/resource budgets/idle behavior and all platform/workflow acceptance. Checkpoints below retain detailed evidence and known gaps; continue without routine milestone stops.

## Blockers

None preventing implementation. Full visual/platform validation is outstanding, not passed.

## Foundation checkpoint (2026-11-11)

Implemented:

- `renderkit`: explicit shared device context; registry-owned generational image handles; GPU-only images; opt-in asynchronous capture; transfer counters; host-owned WGPU surfaces for owned Rust windows/canvases and foreign CAMetalLayer handles.
- SceneVM: injected context/atlas constructors, retained GPU view targets, GPU output image leases, lazy staging allocation in the legacy texture bridge, propagated rendering errors. Fixed per-layer blend mode uniform ownership (a shared buffer overwritten before submit made layers observe the last mode).
- TheFramework: backend-neutral ordered paint, affine transforms, rectangular/path stencil clips, isolated opacity groups, GPU path tessellation/MSAA, bounded Swash glyph CPU cache and atlas pages, Parley layout/caret mapping, graph paint-list adapter.
- GPU renderers remain additive foundations: canonical Creator/client UI still uses legacy drawing until conversion. None of M1/M2/M3 is declared fully complete.

Verification passed:

1. `cargo test --offline -p theframework-core`: 2 persistence/image contracts.
2. `cargo check --offline -p renderkit -p theframework`: passed.
3. `cargo test --offline -p renderkit --test gpu_contract -- --ignored`: 2 actual native GPU tests passed, including WGPU surface creation/configuration/resizing/suspend from a retained CAMetalLayer with no Winit host.
4. `cargo test --offline -p scenevm --test gpu_outputs -- --ignored`: independent red/green GPU outputs passed; targets contain no CPU pixels or staging storage before explicit capture.
5. `cargo test --offline -p theframework --no-default-features --features text --lib`: 12 tests passed, including Swash cache reuse and layout/caret hit testing.
6. `cargo test --offline -p theframework --no-default-features --features ui,render-wgpu --test paint_gpu -- --ignored`: native GPU clip/group/text/curve test passed; a second identical frame caused no additional image upload or glyph rasterization, and no readback occurred before explicit capture.
7. `cargo check --offline -p eldiron-creator -p eldiron-client -p eldiron-client-wgpu -p eldiron-client-terminal`: passed (33.75 s).
8. Captured `native-wgpu-evidence/paint-foundation.png` at 150% density and visually inspected text/curve/clip/group rendering.

The first GPU pipeline run exposed a WGPU 29 depth/stencil descriptor validation error; corrected explicit depth settings, then reran successfully. The integration fixture also checks overlapping opaque content inside an opacity group to detect incorrect per-primitive opacity.

Next actions:

- Connect the paint contract to the widget/layout tree and native GPU application host, converting widget families explicitly. Keep inventory of unsupported/legacy callers and avoid a root framebuffer fallback.
- Extend graphics checks to browser/iOS compile targets; test the graph adapter and real controls in the GPU host.
- Extract shared local session orchestration and remaining CPU data dependencies without widening runtime/UI coupling.

Outstanding: full Creator/client migration, isometric GPU-only sprite submission, runtime/session extraction, complete view/temporal state integration, IME and all locale/interaction parity, browser surface and Xcode wrapper cutover, release performance baselines. A real CAMetalLayer foundation test is not full Xcode application validation.

## UI and host checkpoint (2026-11-11)

The paint foundation now drives real TheFramework UI trees through an application-owned WGPU host. This remains an intermediate implementation checkpoint; Creator and client entry points have not switched, and the full rewrite Goal remains active.

Implemented:

- `platform::{GpuApplication,UiSession,GpuPresenter}` separates application/UI state from device/surface ownership. `platform::winit` creates and presents a native WGPU window directly, supports asynchronous browser startup, resize/density changes, suspension and replacement generations, and rebuild callbacks. There is no destination-pixel callback in this contract.
- The same `GpuPresenter` accepts Rust surfaces created over a host-retained CAMetalLayer. The native test actually submitted and presented UI/text through this path without Winit.
- Neutral physical-key, committed-text, preedit, pointer, relative-motion, scroll, focus and file-drop events; explicit consumption separate from redraw; focused-widget undo/clipboard routing. Layout records screen transforms for inverse input mapping, including captured drags and scrolled/embedded controls.
- Canvas trees and converted scrolling/tree/stack/tab/split layouts emit ordered paint instead of composing raster caches. GPU layout retains dimensions without allocating pixel storage. A constraint-aware measurement hook lets wrapped tree text use actual shaping widths.
- Standard buttons, bars, separators, menus, labels, checkboxes, sliders, tabs, list/tree rows, dropdowns/popups, scrollbars, image views and GPU-image render views have explicit paint methods. New paint/preparation methods preserve existing controllers, IDs and authoring/event semantics. Unsupported families return an error; there is no raster fallback behind the new interface.
- Single/multiline editors share Parley layout for GPU glyph runs, selections, caret geometry and grapheme hit testing. Atomic committed text produces one undo entry; transient composition does not alter the document. Syntax foreground, matches/errors, line numbers, scrollbars and debugging backgrounds have GPU paint paths. Drag labels now use paint and Swash text rather than an intermediate text bitmap.
- File pickers are neutral queued requests. The desktop host adapter uses optional desktop dialogs; the shared UI no longer requires unsupported desktop dialog/clipboard dependencies on iOS. Foreign hosts can service requests and report completion.
- Added explicit device-loss tracking to the shared GPU context; application/UI state is retained outside the presenter.

Verification passed:

1. 88 framework library tests, GPU paint integration, and four actual-GPU UI integration tests (canvas storage/cache reuse/resize; Unicode/IME/atomic undo; inverse scrolled-form hit testing; tree embedded editing and dropdown overlay selection). Log: `/private/tmp/eldiron-ui-checkpoint-tests.log`.
2. Actual UI presentation on a foreign CAMetalLayer, plus zero-size deferral, with no CPU canvas or readback: `/private/tmp/eldiron-apple-presenter-test.log`.
3. Native Winit example (`native_gpu_ui --smoke`) presents three real UI frames and exits; no canvas pixels. The new host dependency graph contains neither Pixels nor Softbuffer.
4. Browser-target `platform-winit` compile and iOS-simulator `ui,render-wgpu` compile passed. Logs: `/private/tmp/eldiron-ui-checkpoint-web.log`, `/private/tmp/eldiron-ui-checkpoint-ios.log`.
5. Creator/classic/WGPU/terminal client compile checks passed; existing entry points remain usable during conversion.
6. Captures `native-wgpu-evidence/ui-foundation.png` and `native-wgpu-evidence/editors-foundation.png` were visually inspected at 150% density.

The first iOS UI compile exposed the desktop-dialog dependency; moving requests to the host boundary resolved it. The embedded-editor fixture initially clicked past the first glyph while expecting the start position; its pointer was corrected to the visible left edge and all interaction assertions passed.

Next work (continue autonomously):

- Remaining widget families: RGBA/tile/palette views, color/direction pickers, icon grid/row, switch/time controls, rich text/Markdown, and legacy node canvas. The unreferenced legacy code-layout source is not part of the active UI module tree.
- Complete UI parity, including rich styles, IME candidate positioning/composition layout, forward-delete/page/function-key mapping, platform requests/cursor locking, modal and drag coverage, resource eviction, batching and idle scheduling. Existing editor controllers are retained, not declared fully rearchitected or fully parity-tested.
- Migrate Creator/controllers/tool HUDs and the graphical clients to the new contract; extract runtime/session and presentation crates; finish GPU-only scene/isometric and preview paths.
- Browser runtime execution and complete Xcode wrapper/FFI/document integration remain outstanding. Compile checks and detached CAMetalLayer presentation are not full platform acceptance. Device-loss recovery callbacks need application-level fault-injection verification.

### Startup/structural invalidation follow-up

An additional real-window geometry assertion caught a regression introduced when preinput geometry preparation was added: input arriving before GPU/application readiness could resize an empty canvas, then a later `set_layout` did not invalidate dimensions. The original presentation-only smoke check still submitted frames but did not detect empty child geometry.

Corrected the host ordering (input/resume callbacks wait for initialization), added canvas structural invalidation for installed widgets/layouts/child canvases, and included UI tree invalidation in host redraw decisions. Zero-size/absent targets no longer schedule rapid retry redraws. The regression fixture installs and replaces a layout at an unchanged size, verifies nonzero child geometry, and checks that a repaint is requested. The strengthened native smoke fixture verifies both widgets have nonzero assigned rectangles on every presented frame (11 paint operations). Six UI tests including actual CAMetalLayer presentation, all 88 framework library tests, and the native smoke run passed. Logs: `/private/tmp/eldiron-native-geometry-test.log`, `/private/tmp/eldiron-native-geometry-check.log`.

### RGBA/tile view conversion

`TheRGBAView` now emits an uploaded source-image blit plus GPU overlays for zoom/centering/scrolling, rectangular grids, sparse used/hover/drop cells, connected selection boundaries, icon selection, translucent paste previews, and anchor guides. Source and preview dirty flags are distinct from display invalidation, so moving/changing a selection does not upload the source image again. Sparse overlays iterate their state rather than every source cell when zoomed far out. Editable pixel storage remains an explicit source asset.

Validation: native GPU assertions check rectangular grid geometry, preservation of shared selection edges, paste opacity, no source upload on selection changes, and exactly one source-asset upload after a pixel edit. Seven UI integration tests (six actual GPU tests and one layout regression), the paint integration, and all 88 framework library tests passed; all four application compile checks passed. Logs: `/private/tmp/eldiron-tile-checkpoint-tests.log`, `/private/tmp/eldiron-tile-checkpoint-apps.log`. `native-wgpu-evidence/tiles-foundation.png` was captured and visually inspected. Overall rewrite remains active; remaining widget families and application/runtime migration continue.

### Palette controls and backend-neutral UI boundary

Palette grids, single-index dropdowns, and multi-swatch row pickers now use command paint and shared popup positioning/chrome. Their controller/selection/reordering model is preserved. A GPU fixture selects palette cells and popup values using their visible screen locations; explicit capture confirms the blue popup cell's actual GPU pixel color. The palette capture was visually inspected.

Tightened the architecture before further control migration: `UiImageResources` is the UI's image-handle service; `render_wgpu::UiImages` owns the concrete GPU cache. UI widgets and `TheUI::paint_commands` no longer require a device or registry. The standalone `ui-paint` feature enables UI/text/commands without a renderer. Neutral input is available independently of GPU host/session adapters. Renamed the command-mode/paint-preparation interfaces accordingly; pure paint helper modules use `_paint.rs` names.

A headless test constructs a real widget tree, shapes text, resolves an image via a recording service, validates its paint commands, and verifies empty canvas pixel storage. Its normal dependency graph excludes WGPU, renderkit, Winit, Pixels and Softbuffer. This checks an actual module boundary rather than merely omitting GPU initialization at runtime.

Validation: 88 library tests, one headless command UI test, paint GPU integration, and eight UI integration tests passed (seven actual GPU fixtures and one structural-layout regression). Palette color/cell picking passed with explicit pixel capture. Native-window smoke, existing four application compile checks, browser host compile, and iOS simulator UI/presenter compile passed. Logs: `/private/tmp/eldiron-ui-neutral-test.log`, `/private/tmp/eldiron-palette-boundary-tests.log`, `/private/tmp/eldiron-palette-boundary-final-tests.log`, `/private/tmp/eldiron-palette-colors-test.log`, `/private/tmp/eldiron-palette-boundary-{apps,web,ios,host}.log`.

Remaining widget families: rich text/Markdown, icon grid/scrollable row, switch/time controls, color/direction pickers, SDF view, and legacy node canvas. Continue with these, then Creator/client/runtime/presentation cutover. Full idle/invalidation behavior, resource eviction, draw batching, dynamic fonts/IME parity and platform application acceptance remain verification/work items; no completion claim for the overall rewrite.

### Rich text and Markdown

`TheTextView` now uses shaped command paint for foreground spans, backgrounds, underlines, hover styling, selection, link hit testing and scrolling. Markdown now projects its AST into plain text plus grapheme spans in a separate document adapter. This fixes byte-offset link ranges after non-ASCII text and keeps links active inside emphasis. Code/paragraph text is preserved by the projection. Existing style/controller interfaces remain in use.

A shared text viewport planner now supplies scroll geometry for multiline editing, rich text and Markdown. Preparation receives an immutable theme explicitly, so theme-dependent text styles and code-highlight themes resolve before layout/paint rather than rebuilding layout from inside painting. Widget paint still accesses no GPU/device/registry; image requests remain behind the backend service.

Validation: 90 library tests (including two Markdown document projection tests), headless command UI, the paint GPU integration, and ten UI integration tests passed. New actual-GPU fixtures verify foreground/underline preservation during link hover, exactly one rich-text link action, and exactly one Markdown URL-request event with a non-ASCII prefix/combining accent and nested emphasis. URL requests were observed in the event stream; no external URL was opened. Rich-text and Markdown captures were visually inspected at 150% density. All four application compile checks and browser/iOS-simulator compile checks passed. Logs: `/private/tmp/eldiron-markdown-neutral-tests.log`, `/private/tmp/eldiron-text-checkpoint-tests.log`, `/private/tmp/eldiron-text-checkpoint-{apps,web,ios}.log`.

Next widget families: switch/time controls, icon grid/row, color/direction pickers, SDF view, legacy node canvas. Application/runtime migration and final acceptance/performance/platform work remain outstanding; continue without a milestone stop.

### Procedural controls, collections and node canvas

Converted the remaining active built-in widget implementations: switch/time controls, icon grid and scrollable row, color/direction pickers, SDF view and node canvas. Layouts invoke a shared widget entry point that skips unassigned rectangles, preventing controls from drawing stray geometry at the origin. Collection preparation computes visible hit rectangles; image content invalidation is independent of hover/selection/scrolling.

The backend-neutral paint contract now includes colored triangle meshes with explicit sRGB interpolation, analytic nearest-distance fields/materials, and explicit repeating image sampling. The color picker emits a geometric hue ring and triangle; direction paint uses its own widget center. SDF views send analytic shapes/materials, preserving nearest-shape overlap rather than replacing it with painter-order fills. Fixed pre-existing signed-distance errors in rhombus/rectangular interiors and a zero-length direction normalization case. Analytic fields composite premultiplied linear color with density-aware edge coverage.

The node canvas retains interaction rectangles and emits node chrome, Swash labels, sockets, curves, cut feedback and focus through commands. Its command path clears legacy viewport pixel storage. CPU previews/overlays remain explicit source assets; GPU previews can be supplied as opaque image handles without UI/device coupling. Repeated overlays use one quad regardless of tile size. Preview/overlay content dirtiness is separate from interaction redraw. Hit testing honors the selected node's actual topmost draw order; deletion clears the stale selected index and preserves/remaps GPU preview handles. Application preview producers still need migration to the GPU handle API, and legacy wire cutting still needs its hit geometry reconciled with the painted curve.

Validation passed: 91 library tests, three renderer-free command UI tests, paint GPU integration and twelve UI integration tests (eleven actual GPU fixtures plus one structural-layout regression). New fixtures verify visible icon selection after scrolling, one-thumbnail invalidation, no upload on selection, unassigned-widget suppression, time-track agreement, actual color-ring/SDF GPU pixels, nearest-material overlap, zero procedural raster-asset uploads, live GPU-only node previews, one-quad tiled overlays, exactly one terminal-connection event and node-drag event. Procedural and node captures were visually inspected at 150% density. All four application compatibility checks, browser host compile and iOS-simulator UI/presenter compile passed. Logs: `/private/tmp/eldiron-icon-headless-tests.log`, `/private/tmp/eldiron-widget-checkpoint-tests.log`, `/private/tmp/eldiron-widget-checkpoint-{apps,web,ios}.log`. Evidence: `native-wgpu-evidence/procedural-controls.png`, `native-wgpu-evidence/node-canvas.png`.

A first GPU shader run rejected a reserved WGSL identifier; it was renamed and the complete GPU regression suite then passed. A direction-line pixel assertion was adjusted to account for fractional-DPI MSAA coverage; logical geometry and visible pixel darkness are both verified. These are intermediate checks; full Creator/client migration, performance and platform acceptance remain outstanding, and the overall Goal stays active.

### CPU session, schema boundary and terminal adoption

Added `eldiron-runtime::GameSession`: it owns project/engine assets/server/current map, an elapsed-time-driven clock and one collector for region messages, speech, choices, audio requests, world text and container requests. Render rate does not advance simulation. Bounded catch-up retains debt; paused sessions clear elapsed wall time. A host selects output channels and takes pending region events once. Started-session shutdown belongs to the session owner; constructing/dropping an unstarted session does not clear engine state.

The terminal now owns a `GameSession` and uses its startup/tick/output APIs. Removed its duplicate content preparation, character/item/template startup, local-player setup and server-update loop. Terminal text/roguelike/AI presentation remains its own controller. CPU project preparation now lives in `shared::runtime_assets`; existing Creator/graphical entry points use thin `shared::rusterix_utils` adapters for device/overlay invalidation. Runtime startup owns world-map lookup, independent of client setup. This is an intermediate module boundary, not graphical application cutover.

Moved persisted graph/schema/branch data, `TheId` and `TheTileMask` into `theframework-core` with identity-compatible public reexports. Ruleset compilation depends on the CPU core instead of the UI framework. The legacy `TheValue` (including its CPU bitmap variant) is now outside the UI feature. Workspace dependencies no longer force UI for every engine consumer. Extracted appearance-data fingerprints and light flicker from renderer ownership; CPU lighting uses the same light-data evaluator. Pure CPU geometry batches/render settings no longer incorrectly depend on the graphics feature.

An isolated terminal build originally failed because graphics-gated symbols were referenced unconditionally; the combined four-application check masked this through feature unification. The repaired isolated runtime and terminal build without the framework UI feature and without WGPU, renderkit, Winit, Pixels, Softbuffer, Parley, Swash or Zeno. `tools/check_renderer_boundaries.py` independently resolves and checks CPU core, ruleset, runtime, terminal and renderer-free UI graphs. Core/ruleset graphs also exclude the legacy framework/font rasterizer. Legacy CPU bitmap/value carriers still bring the UI-disabled framework/fontdue transitively into engine/shared data; completing that data-storage extraction remains work.

Validation passed:

- Three runtime tests: render-rate-independent clock/catch-up, pause/reset and exactly-once event collection; actual `test_projects/3DStarter.eldiron` startup/player registration/world lookup, archive round-trip and shutdown without GPU initialization. Final log `/private/tmp/eldiron-session-runtime-final.log` (27.96 s integration run).
- Ten terminal behavior tests, including its existing AI-provider fakes, command handling and rules formatting: `/private/tmp/eldiron-session-terminal.log`. Fixed a stale test expectation from palette index 10 to the unchanged official training-sword index 20; the ruleset data/formatter were not changed.
- Nine CPU asset-preparation/shared tests: valid/invalid rules publication, cache reuse, saved material planes, editable model/bitmap precedence, worn-material edits, offscreen equipment and unified palette preservation: `/private/tmp/eldiron-session-assets.log` (98.85 s). Tests that actually invoke the legacy graphical icon presenter are correctly gated by graphics.
- Three persisted core/image/schema tests and the moved light-data test with GPU features disabled: `/private/tmp/eldiron-session-core.log`, `/private/tmp/eldiron-session-light.log`.
- All five independent dependency-boundary checks: `/private/tmp/eldiron-session-boundaries.log`.
- Framework 91 library tests, three headless UI tests, paint GPU integration and twelve UI integration tests all passed after extraction. Browser host and iOS-simulator UI/presenter compilation passed: `/private/tmp/eldiron-session-{ui,web,ios}.log`.
- Existing four-application compatibility compilation passed: `/private/tmp/eldiron-session-apps-final.log`.
- End-to-end offline text terminal smoke loaded current `test_projects/Hideout2D.eldiron`, showed the ship description, handled `look` and `inventory` (including Wooden Arrows x20), and quit successfully with AI disabled: `/private/tmp/eldiron-session-cli-2d-smoke.log`. The 3D fixture verifies registration/storage; its lack of a 2D text sector makes it unsuitable for the text-room smoke assertion.

Known compatibility issue found during fixture selection: the older `starters/projects/Roguelike2D.eldiron` contains retired `Fighter` class references and is rejected by existing shared entity/rules validation. Current project fixtures use the accepted classes. No authored class data or rules semantics were silently renamed; this remains a workflow-compatibility item to resolve before final acceptance.

Outstanding M4 ownership: engine region/player routing registries remain process globals, so multiple simultaneously active local sessions are not yet isolated. Graphical clients and Creator Play still use legacy owner adapters and must adopt the session/event boundary. CPU startup still prepares some shared appearance data required by legacy assets; presentation cache/resource ownership, raw font registration, and remaining CPU image/value extraction are not declared finished. Overall rewrite Goal remains active. Continue with shared GPU presentation/application integration, remaining UI/input/performance work and platform acceptance without a milestone stop.

### Application-owned scene views and GPU domains

Created `eldiron-presentation::SceneViews`. Each stable view ID retains its own injected `SceneHandler`/SceneVM, camera/uniform/history state and GPU target, while views share an atlas. The manager binds to the host registry/device domain; it never owns a window or simulation clock. Outputs carry opaque image handles, descriptors, output revisions and caller content revisions. Unchanged outputs/handles are reused; zero-sized views suspend; resize/removal invalidates replaced registrations. Explicit captures return revision-stamped readback tickets. Applications must call `remove`/`clear` when releasing views while retaining the registry.

`SceneHandler::with_vm` enables application injection without initializing a compatibility GPU. SceneRenderTarget now retains one stable GPU image lease rather than constructing a new wrapper every render. Added explicit context generations to injected SceneVM state and VM layers; target/state domain mismatches are rejected before submission. Legacy constructor paths do not satisfy the new injected `render_gpu` ownership contract.

The replacement-device test exposed a real ownership bug: WGPU 29's core device equality proxy compares its device ID, which can coincide between separate WGPU instances. A device-equality-based atlas/cache check reused a texture from the wrong instance and failed during bind-group creation. The canonical path now tags atlas/texture resources with renderkit's explicit generation, recreates GPU atlas storage from retained CPU sources when that generation changes, and keeps mip pipelines with their texture resources. Removed the thread-global mip-pipeline cache. Legacy raw-device APIs remain compatibility paths pending retirement.

Stronger residency checks also found that intermediate VM layer and ping-pong textures still allocated CPU pixel vectors even though the final SceneRenderTarget did not. Those renderer-owned intermediates now use GPU-only storage too. Explicit legacy capture remains an opt-in CPU consumer. Fixed atlas packing/metadata preparation order so a newly supplied shared atlas has valid tile indices before the first geometry build; atlas layout changes invalidate dependent geometry.

Validation passed: two actual GPU scene-view fixtures compose independently transformed textured views, verify shared atlas identity, stable output/image leases, idle reuse, zero CPU layer/history storage, ping-pong targets, suspend/resume, resizing, retained capture revisions, removal and foreign registry/context rejection. A second fixture reuses CPU atlas sources across separately created GPU contexts and successfully renders on the replacement domain. SceneVM's independent-output GPU fixture, framework 91 library tests, paint GPU integration and twelve UI integration tests passed after these changes. Four-application compatibility compilation passed. Browser and iOS-simulator presentation compilation passed. Logs: `/private/tmp/eldiron-view-checkpoint-{gpu,scenevm,ui,apps,web,ios}.log`.

This remains a scene ownership/composition checkpoint. Game-screen/HUD presentation and actual Creator/client adoption continue next. The new crate does not yet move the legacy aggregate Rusterix/controller outward or eliminate the legacy shared graphical startup bridges. Opaque raster-2D view pixels/transforms are verified; transparent scene-output color/alpha normalization, 3D/isometric application parity, shared geometry-buffer ownership, picking/async revisions, complete application recovery, resource budgets and release performance remain work. No M1/M5/M6 completion claim; overall Goal remains active.

## Glyph submission checkpoint (2026-11-11)

- Ordered glyph runs now accumulate consecutive glyphs using the same atlas page and tint into one indexed batch. Color glyph boundaries and atlas eviction retain their image leases and order. Raster scale, physical phase snapping, transforms and clips are unchanged.
- Added public per-frame `FrameStats` (actual submitted draw calls and glyph quad count, including opacity subpasses) to support performance acceptance measurements.
- Actual GPU dense-text test verifies over 400 visible glyph quads with at most one draw per shaped run plus a page transition; a second frame adds neither image uploads nor Swash rasterization. Explicit capture verifies text coverage.
- Passed: two paint GPU tests and the existing UI GPU regression suite. Logs: `/private/tmp/eldiron-glyph-batches-gpu.log`, `/private/tmp/eldiron-glyph-batches-ui.log`.
- This removes per-glyph GPU buffer/bind allocations for a run. General primitive geometry/uniform arenas, scratch pooling, cache budgets and full application performance acceptance remain open.

## Game-screen paint checkpoint (2026-11-11)

- Began game-screen/HUD emission in `eldiron-presentation::screen`, outside the authoritative runtime and framework UI. `ScreenPainter` consumes existing CPU model/controller state and emits text, text-input, choice, stat, decoration, authored artwork, repeating spatial pages and nine-slice commands. It receives no destination pixels or server.
- Extracted text placeholder resolution and stat value/geometry descriptions from CPU drawing. The legacy adapter and new presenter use the same content and rules semantics. Choice interaction and painting use the same public rectangles.
- Added named font registration, so an authored project font asset name selects its bytes regardless of its internal font-family name. Verified the shaped glyph face uses those exact bytes.
- `ScreenImageCache` owns source registrations in the host image registry. It reuses unchanged uploads, checks domain ownership, retains both source revisions used in one frame and evicts unused assets to a byte budget. No screen-sized CPU overlay is registered. Fingerprinting still scans source bytes; revision-driven asset invalidation and the overall resource/performance acceptance remain work.
- Passed: three actual GPU tests for controls/Unicode/choice selection/stat geometry, cache ownership/reuse/eviction, and animation/spatial-page/nine-slice pixel addressing. `/private/tmp/eldiron-screen-art-gpu.log`. Captured and visually inspected `native-wgpu-evidence/game-screen-controls.png`.
- Passed: three existing text-content controller tests (`/private/tmp/eldiron-screen-controllers.log`), five independent dependency boundaries (`/private/tmp/eldiron-screen-boundaries.log`), browser and iOS-simulator presentation compilation (`/private/tmp/eldiron-screen-web.log`, `/private/tmp/eldiron-screen-ios.log`).
- Remaining: buttons/item/portrait/profile/message families, static screen geometry and full HUD composition; graphical app and Creator adoption. Current legacy screen setup still allocates bitmap storage, and this checkpoint does not switch a canonical launcher or claim full M5 acceptance.
- All four existing application entry points compile after this extraction: `/private/tmp/eldiron-screen-apps.log` (36.69 s).

## GPU game-view preparation checkpoint (2026-11-11)

- Added explicit GPU surface dimensions to the legacy game controller and separated camera/scene preparation from CPU destination/upscale allocation. Legacy CPU rendering still allocates its own output only when explicitly selected.
- `SceneViews::configure_game` installs the actual engine 2D shader, configuration and atlas on the injected base VM. `prepare_game` accepts borrowed CPU map/assets/time/render-state input, prepares each widget against its own SceneHandler and resizes only that GPU view. It never advances or drains a server.
- Reduced GPU resolution preserves logical 2D camera extent; physical render dimensions no longer change the visible map area. Native preparation releases any previous CPU adapter buffers. Zero-size views suspend. Text/isometric game presentation currently returns an explicit unsupported-path error; their command/sprite conversions remain required.
- Three actual GPU view tests pass, including a textured map prepared by the real game camera/chunk builder in two independent views at full/half resolution, camera extent equality, no output/upscale/layer CPU storage, source upload/device replacement and explicit capture. Log: `/private/tmp/eldiron-game-views-gpu.log`.
- All four existing apps compile (`/private/tmp/eldiron-game-views-apps.log`), and presentation compiles for browser and iOS simulator (`/private/tmp/eldiron-game-views-web.log`, `/private/tmp/eldiron-game-views-ios.log`).
- Canonical app cutover, native screen setup without bitmap allocations, full HUD families, isometric sprite rendering and session/controller ownership adoption remain outstanding. Continue the whole rewrite.

## Screen geometry submission checkpoint (2026-11-11)

- Added validated backend-neutral `ImageVertex`/`PaintOp::ImageMesh` and GPU execution through the existing ordered paint/clip/transform/composition contract. Prepared screen triangles now submit source images and geometry directly, with no software rasterizer or destination pixels.
- `ScreenPainter::unlit_scene` consumes CPU builder output. Spatial-page boundaries clip triangles in UV space, retain concave empty areas, and address animation independently from page position. Added independent X/Y repeat/clamp samplers. Single-axis repeat follows the existing tile contract (spatial pages are addressed only by RepeatXY).
- Five actual GPU screen tests pass, covering real D2Builder concave geometry, transform application, spatial animation, per-axis clamp/repeat, source lifetime/cache reuse, and prior controls/artwork tests. `/private/tmp/eldiron-screen-mesh-final.log`.
- Existing framework paint and actual GPU UI regressions pass (2 paint tests plus 11 GPU UI tests); browser and iOS presentation compilation pass. Logs: `/private/tmp/eldiron-screen-mesh-framework.log`, `/private/tmp/eldiron-screen-mesh-web.log`, `/private/tmp/eldiron-screen-mesh-ios.log`.
- This is a geometry submission path, not complete screen/launcher adoption. Lit/3D screen scenes and line primitives currently return explicit errors from this adapter; full screen composition, remaining widget families and canonical app cutover remain required.
