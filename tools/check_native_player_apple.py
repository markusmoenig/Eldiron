#!/usr/bin/env python3
"""Build/run the actual Swift/common-player facade on detached Metal layers.

macOS + Xcode + a Metal adapter are required. No Winit/window or signing is used;
this does not establish physical Xcode, IME, iOS runtime or release acceptance.
"""
import os
import pathlib
import platform
import re
import shlex
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGET = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
OUTPUT = pathlib.Path(tempfile.mkdtemp(prefix="eldiron-apple-host-"))


def run(name, command, timeout=300):
    with (OUTPUT / (name + ".log")).open("w") as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log,
                                stderr=subprocess.STDOUT, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{name}: exit {result.returncode}; {OUTPUT / (name + '.log')}")
    print(f"{name}: passed ({OUTPUT / (name + '.log')})", flush=True)


def main():
    if sys.platform != "darwin":
        raise RuntimeError("This check requires macOS/Xcode/Metal")
    # Match the checked-in Xcode deployment target for native C build scripts
    # too; otherwise they can silently inherit this host's newer OS minimum.
    os.environ.setdefault("MACOSX_DEPLOYMENT_TARGET", "26.1")
    run("rust", ["cargo", "rustc", "--offline", "-p", "eldiron-client-wgpu",
                 "--no-default-features", "--features", "native-player", "--lib",
                 "--", "--print", "native-static-libs"])
    log = (OUTPUT / "rust.log").read_text()
    match = re.search(r"native-static-libs: (.+)", log)
    if not match:
        raise RuntimeError("Rust did not report its native link dependencies")
    # The compatibility facade must retain every existing exported ABI name.
    reference = (ROOT / "clients/client-wgpu/src/apple_reference.rs").read_text()
    expected = set(re.findall(r'pub unsafe extern "C" fn ([a-z_]+)', reference))
    archive = TARGET / "debug/libscenevm_unified_app.a"
    members = subprocess.check_output(["ar", "-t", str(archive)], text=True).splitlines()
    application = [name for name in members
                   if name.startswith("scenevm_unified_app-") and name.endswith(".o")]
    if not application or len(expected) < 30:
        raise RuntimeError("Application objects or established ABI inventory are missing")
    # Apple's nm cannot parse the newer LLVM bitcode bundled in Rust's std.
    # Inspect every compiled application Mach-O object instead; these own all
    # facade exports. Do not accept partial output from a failed whole-archive nm.
    objects = OUTPUT / "application-objects"
    objects.mkdir()
    paths = []
    for member in application:
        if pathlib.Path(member).name != member:
            raise RuntimeError(f"Unexpected archive member path: {member}")
        path = objects / member
        path.write_bytes(subprocess.check_output(["ar", "-p", str(archive), member]))
        paths.append(str(path))
    run("abi", ["nm", "-gjU", *paths])
    symbols = (OUTPUT / "abi.log").read_text()
    exported = {line.removeprefix("_") for line in symbols.splitlines()
                if line.startswith(("_unified_app_runner_", "_eldiron_player_runner_"))}
    if expected - exported:
        raise RuntimeError(f"Missing compatibility exports: {sorted(expected - exported)}")
    print(f"abi: {len(expected)} established exports preserved", flush=True)
    sources = [ROOT / "Xcode/Client/SceneVM" / (name + ".swift")
               for name in ["SceneVMDocument", "SceneVMFFI", "SceneVMView", "NativeTextClient", "NativeUIKitTextClient"]]
    # Match Xcode's Swift 5 mode and Approachable Concurrency flags. A plain
    # typecheck can otherwise miss closure actor-isolation diagnostics in the
    # actual bundle build.
    swift_flags = ["-warnings-as-errors", "-swift-version", "5", "-default-isolation", "MainActor"]
    for feature in ["DisableOutwardActorInference", "InferSendableFromCaptures",
                    "GlobalActorIsolatedTypesUsability", "MemberImportVisibility",
                    "InferIsolatedConformances", "NonisolatedNonsendingByDefault"]:
        swift_flags += ["-enable-upcoming-feature", feature]
    sdk = subprocess.check_output(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"], text=True).strip()
    run("ios-sdk", ["xcrun", "swiftc", "-typecheck", *swift_flags, "-sdk", sdk,
                    "-target", "arm64-apple-ios26.1-simulator", *map(str, sources),
                    str(ROOT / "Xcode/Client/Tests/NativeUIKitTextClientCheck.swift")])
    executable = OUTPUT / "NativePlayerHostCheck"
    run("swift", ["xcrun", "swiftc", *swift_flags, "-target",
                  f"{platform.machine()}-apple-macosx26.1", *map(str, sources),
                  str(ROOT / "Xcode/Client/Tests/NativePlayerHostCheck.swift"),
                  str(ROOT / "Xcode/Client/Tests/NativeTextClientCheck.swift"),
                  "-L", str(TARGET / "debug"), "-lscenevm_unified_app",
                  *shlex.split(match.group(1)), "-o", str(executable)])
    run("host", [str(executable), str(ROOT / "test_projects/3DStarter.eldiron")], timeout=90)
    print(f"All detached-layer Swift checks passed: {OUTPUT}")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
