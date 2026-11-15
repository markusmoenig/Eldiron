#!/usr/bin/env python3
"""Run optimized checked-in UIKit player/text owners in an isolated simulator.

Creates and cleans up its own simulator by default. --device reuses an explicitly
supplied booted device and leaves its lifecycle alone. Never selects `booted`.
This is simulator evidence, not physical iOS/IME acceptance or a speed benchmark.
"""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
OUTPUT = Path(tempfile.mkdtemp(prefix="eldiron-uikit-release-"))
BUNDLE = "com.eldiron.nativeuikitacceptance"


def run(name, command, timeout=300):
    with (OUTPUT / (name + ".log")).open("w") as log:
        completed = subprocess.run(command, cwd=ROOT, stdout=log,
                                   stderr=subprocess.STDOUT, timeout=timeout)
    if completed.returncode:
        raise RuntimeError(f"{name}: exit {completed.returncode}; {OUTPUT / (name + '.log')}")
    print(f"{name}: passed ({OUTPUT / (name + '.log')})", flush=True)


def simctl(*args):
    return subprocess.check_output(["xcrun", "simctl", *args], text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", help="Exact already booted acceptance simulator UUID")
    parser.add_argument("--runtime", default="com.apple.CoreSimulator.SimRuntime.iOS-26-5")
    parser.add_argument("--creator", action="store_true", help="Exercise the actual Creator UIKit wrapper")
    args = parser.parse_args()
    if sys.platform != "darwin":
        raise RuntimeError("Requires macOS, Xcode, simulator runtime and Metal")
    if args.device and not re.fullmatch(r"[0-9A-Fa-f-]{36}", args.device):
        raise RuntimeError("--device must be an exact simulator UUID")
    os.environ.setdefault("MACOSX_DEPLOYMENT_TARGET", "26.1")
    os.environ.setdefault("IPHONEOS_DEPLOYMENT_TARGET", "26.1")
    sdk = subprocess.check_output(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"], text=True).strip()
    # The desktop shell can export a macOS SDKROOT. Both C build scripts and
    # Swift's linker subprocess must see the selected simulator SDK instead.
    os.environ["SDKROOT"] = sdk
    target = "aarch64-apple-ios-sim"
    package = "eldiron-creator" if args.creator else "eldiron-client-wgpu"
    features = "staticlib" if args.creator else "native-apple"
    bundle = BUNDLE + (".creator" if args.creator else ".player")
    run("rust", ["cargo", "rustc", "--release", "--locked", "--offline", "-p",
                 package, "--no-default-features", "--features", features,
                 "--target", target, "--lib", "--", "--print", "native-static-libs"], timeout=900)
    match = re.search(r"native-static-libs: (.+)", (OUTPUT / "rust.log").read_text())
    if not match:
        raise RuntimeError("Rust did not report its native link dependencies")
    app = OUTPUT / "NativeUIKitAcceptance.app"
    app.mkdir()
    sources = [ROOT / "Xcode/Client/SceneVM" / (name + ".swift") for name in
               ["SceneVMDocument", "SceneVMFFI", "SceneVMView", "NativeTextHost",
                "NativeTextClient", "NativeUIKitTextClient"]]
    sources += [ROOT / "Xcode/Client/Tests" / (name + ".swift") for name in
                ["NativeUIKitTextClientCheck", "NativeUIKitHostCheck"]]
    bridge = []
    library = "scenevm_unified_app"
    if args.creator:
        sources = [ROOT / "Xcode/Eldiron/Eldiron Shared" / (name + ".swift")
                   for name in ["Renderer", "RMKTView", "Keycodes", "Utils", "Math"]]
        sources += [ROOT / "Xcode/Client/SceneVM" / (name + ".swift")
                    for name in ["NativeTextHost", "NativeTextClient", "NativeUIKitTextClient"]]
        sources += [ROOT / "Xcode/Eldiron/Eldiron iOS/GameViewController.swift",
                    ROOT / "Xcode/Eldiron/Tests/NativeCreatorUIKitCheck.swift",
                    ROOT / "Xcode/Client/Tests/NativeUIKitTextClientCheck.swift"]
        bridge = ["-import-objc-header", str(ROOT / "Xcode/Eldiron/Eldiron Shared/Bridge.h")]
        library = "eldiron_creator"
    flags = ["-warnings-as-errors", "-swift-version", "5", "-default-isolation", "MainActor"]
    for feature in ["DisableOutwardActorInference", "InferSendableFromCaptures",
                    "GlobalActorIsolatedTypesUsability", "MemberImportVisibility",
                    "InferIsolatedConformances", "NonisolatedNonsendingByDefault"]:
        flags += ["-enable-upcoming-feature", feature]
    run("swift", ["xcrun", "swiftc", "-O", "-g", "-parse-as-library", *flags,
                  "-sdk", sdk, "-target", "arm64-apple-ios26.1-simulator", *bridge, *map(str, sources),
                  "-L", str(TARGET / target / "release"), "-l" + library,
                  *shlex.split(match.group(1)), "-o", str(app / "NativeUIKitAcceptance")])
    info = {
        "CFBundleExecutable": "NativeUIKitAcceptance", "CFBundleIdentifier": bundle,
        "CFBundleName": "Native UIKit Acceptance", "CFBundleVersion": "1",
        "CFBundleShortVersionString": "1.0", "CFBundlePackageType": "APPL",
        "MinimumOSVersion": "26.1", "LSRequiresIPhoneOS": True,
        "UIDeviceFamily": [1, 2], "UILaunchScreen": {},
        "UIApplicationSceneManifest": {"UIApplicationSupportsMultipleScenes": False,
            "UISceneConfigurations": {"UIWindowSceneSessionRoleApplication": [
                {"UISceneConfigurationName": "Acceptance"}]}},
    }
    (app / "Info.plist").write_bytes(plistlib.dumps(info))
    shutil.copyfile(ROOT / "test_projects/3DStarter.eldiron", app / "game.eldiron")
    run("sign", ["codesign", "--force", "--sign", "-", str(app)])
    device = args.device
    created = False
    try:
        if not device:
            device = simctl("create", "Eldiron UIKit Release Acceptance",
                            "com.apple.CoreSimulator.SimDeviceType.iPad-A16", args.runtime)
            created = True
            run("boot", ["xcrun", "simctl", "boot", device])
            run("bootstatus", ["xcrun", "simctl", "bootstatus", device, "-b"])
        run("install", ["xcrun", "simctl", "install", device, str(app)])
        data = Path(simctl("get_app_container", device, bundle, "data"))
        result_path = data / "Documents/NativeUIKitAcceptance.json"
        result_path.unlink(missing_ok=True)
        run("runtime", ["xcrun", "simctl", "launch", "--console", "--terminate-running-process",
                        device, bundle], timeout=130)
        result = json.loads(result_path.read_text())
        (OUTPUT / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        if (result.get("status") != "passed" or not result.get("uiKitTextProtocolPassed")
                or result.get("firstPresentations", 0) < 3
                or result.get("finalPresentations", 0) <= result["firstPresentations"]
                or result.get("readbackBytes") != 0):
            raise RuntimeError(f"UIKit acceptance failed: {result}")
        print(f"Release UIKit {package}/text runtime passed: {OUTPUT / 'result.json'}", flush=True)
    finally:
        if device:
            subprocess.run(["xcrun", "simctl", "terminate", device, bundle],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if created:
            subprocess.run(["xcrun", "simctl", "shutdown", device], check=False)
            subprocess.run(["xcrun", "simctl", "delete", device], check=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"{error}\nEvidence directory: {OUTPUT}", file=sys.stderr)
        sys.exit(1)
