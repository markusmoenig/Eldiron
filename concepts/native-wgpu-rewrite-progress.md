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

- M0: in progress (baseline compile and initial contracts/decisions recorded; visual/performance baselines still needed).
- M1: starting explicit GPU infrastructure and scene target interfaces.
- M2/M3: pending paint/text/backend and widget conversion.
- M4: CPU core data extraction started; runtime/session extraction pending.
- M5/M6/M7/M8/M9: pending; no platform/runtime parity claimed.

## Implementation and verification

- Extracted TheColor/ThePalette/TheTime/TheDim into `theframework-core` with existing module compatibility re-exports.
- Added backend-neutral ImageHandle and validated CPU ImageAsset.
- Added persisted-data shape and image storage contract tests. Passed: 2 contract tests.

## Next work

1. Compile and test core extraction; preserve baseline builds.
2. Implement renderkit context, native/Xcode surface targets, texture registry and explicit readback.
3. Inject context into SceneVM and expose GPU scene outputs; then paint/Swash pipeline and application migration.

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
