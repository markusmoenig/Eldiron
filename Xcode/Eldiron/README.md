# Creator Xcode GPU host

Creator's Xcode wrapper uses `NativeCreatorApplication` and the same paint,
Swash glyph-atlas and GPU view composition as the Rust window host. Swift owns
the view and retains its CAMetalLayer; Rust creates the WGPU surface and is the
only drawable consumer/presenter. There is no `rust_draw` bitmap ABI or Swift
texture-copy renderer.

Build a matching Rust library slice before building the wrapper:

```sh
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build -p eldiron-creator --lib --release \
  --no-default-features --features staticlib --target aarch64-apple-darwin
xcodebuild -project Xcode/Eldiron/Eldiron.xcodeproj -scheme 'Eldiron macOS' -configuration Release \
  MACOSX_DEPLOYMENT_TARGET=26.1 CODE_SIGNING_ALLOWED=NO build
```

For iOS, use `aarch64-apple-ios` or `aarch64-apple-ios-sim`, setting
`IPHONEOS_DEPLOYMENT_TARGET=26.1`. `ELDIRON_RUST_LIB_DIR` selects the library
directory, defaulting to the matching target's release directory. Override it
when using a shared Cargo target directory. These commands
describe the verified SDK configuration; older OS versions remain unverified.

`Bridge.h` documents the main-thread v1 ABI. The Swift `Renderer` retains the
nonzero handle from `eldiron_creator_create_v1(1, ...)`, supplies it on every call,
and destroys it before releasing its layer. Invalid/obsolete handles fail, and
handles are never recycled. Different layers can own independent Creator hosts,
with separate documents, controllers, sessions, GPU domains and pending requests.
A layer already owned by another live host is rejected on creation/replacement.
Unversioned exports address only the host created by `rust_init`; a scoped v1
forward cannot change that legacy owner. Separate document tabs remain supported
inside each host. macOS close approvals belong to the closing host; Quit checks
all remaining windows. Menu actions follow the key window, its sheet parent or
the main Creator, and do not pick an arbitrary owner when several are inactive.

Resize dimensions are physical pixels; input is in logical points. Keep the
layer alive until `eldiron_creator_destroy_v1` or a successful
`eldiron_creator_replace_layer_v1` returns. Swift never reads `currentDrawable`.
Display callbacks honor the application's update cadence and only present dirty
frames. Suspend/resume and device/layer replacement retain the CPU application.
Clipboard completions carry the original host and request ID; Edit menu commands
use the same controller route as keyboard shortcuts.

Versioned hosts receive addressed Open/Save/Save As requests from toolbar and
keyboard commands. The host reads/writes bytes; Rust handles project decoding,
migrations, activation, undo and dirty state. The byte API captures the active
document UUID before asynchronous reads, and uses one immutable save snapshot
identified by a monotonic ticket. Query/copy save bytes into host-owned memory,
then finish that ticket exactly once for success, failure or cancellation. Rust
marks only its source tab clean, and only when canonical authored source still
matches the saved snapshot. ZIP metadata does not participate in that comparison.
No borrowed Rust document pointer survives a call. Free returned JSON strings
with `rust_free_string`; Bridge.h describes response envelopes and buffer rules.

macOS uses asynchronous panels and background atomic file writes. iOS uses the
system document picker, retaining an export file until success or cancellation.
Callbacks capture their original host/document/ticket; a tab switch or host
recreation cannot redirect a delayed load/save. Native Scepter listener lifetime
also belongs to the Creator instance and ends during teardown.

`Tests/NativeCreatorHostCheck.swift` links to the actual static library and checks
presentations, zero root bitmap/readback, resize/density, suspend/resume, layer
replacement, file-request routing, save/reopen/copy limits, handle recreation, simultaneous independent Creator hosts, legacy routing and
stale completions on retained detached layers. Obtain native link arguments with
`cargo rustc --release -p eldiron-creator --lib --no-default-features --features staticlib --
--print native-static-libs`; pass those arguments, `-leldiron_creator`, the library
search directory and `-import-objc-header 'Xcode/Eldiron/Eldiron Shared/Bridge.h'`
to `swiftc` when linking that executable. Run it on the main thread with Metal
available. It is a detached-layer check, separate from physical window evidence.

`tools/check_native_creator_apple.py` builds optimized Rust/Swift owner checks.
Use `--windows` on an unlocked Mac for strict actual AppKit presentation/teardown.
`--window-lifetimes` separately verifies eager close and stale handles while
controllers/windows remain retained; it does not accept physical presentation.
Window exit/teardown only closes that owner, stops its callbacks, destroys its
Rust host and then releases the backing layer.

`tools/check_native_uikit.py --creator` builds optimized Rust/Swift and executes
the checked-in UIKit controller/view/renderer on a separate simulator. It checks
positive presentations, zero root framebuffer/readback, asynchronous Swift file
load, addressed save/copy/finish, disk reopen and stale-host rejection while the
closed controller remains retained. The default creates and cleans up its own
simulator; `--device UUID` uses only the explicitly supplied booted acceptance
device and leaves its lifecycle alone. Player mode additionally checks that one
CPU session survives actual UIKit view/layer reconstruction.

Native macOS/iOS composition/accessibility, complete physical input/document
workflows, full mobile Play/touch/IME coverage, physical multiwindow interaction and
distribution remain in the rewrite acceptance matrix. Creator reuses the client's
`NativeTextHost`, AppKit input context and UIKit UITextInput responders. The
shared Rust UTF-16 protocol validates source identity/revision and Unicode
boundaries before edits. Source text is queried on demand and selection-only
updates reuse it. Physical game controls and logical keyboard shortcuts remain
separate from IME text insertion. SDK compilation alone does not verify an
actual input method or mobile runtime interaction.

On macOS, a physical key consumed by a focused Rust editor still passes through
the native text context when it represents printable text. This preserves the
system keyboard layout and dead-key interpretation; handled shortcuts and
navigation do not execute twice. Creator's Select All menu uses the first
responder, so document panels keep their standard editing behavior too.
