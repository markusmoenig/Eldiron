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
