#!/usr/bin/env python3
"""Build/run release Creator FFI ownership checks on retained Metal layers.

Requires macOS, Xcode and Metal. This checks actual independent foreign hosts;
it does not establish physical window, IME or mobile runtime acceptance.
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
OUTPUT = pathlib.Path(tempfile.mkdtemp(prefix="eldiron-creator-apple-host-"))


def run(name, command, timeout=600):
    path = OUTPUT / (name + ".log")
    with path.open("w") as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log,
                                stderr=subprocess.STDOUT, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{name}: exit {result.returncode}; {path}")
    print(f"{name}: passed ({path})", flush=True)


def main():
    if sys.platform != "darwin":
        raise RuntimeError("This check requires macOS/Xcode/Metal")
    os.environ.setdefault("MACOSX_DEPLOYMENT_TARGET", "26.1")
    run("rust-release", ["cargo", "rustc", "--release", "--locked", "--offline",
                        "-p", "eldiron-creator", "--no-default-features",
                        "--features", "staticlib", "--lib", "--",
                        "--print", "native-static-libs"])
    match = re.search(r"native-static-libs: (.+)", (OUTPUT / "rust-release.log").read_text())
    if not match:
        raise RuntimeError("Rust did not report native link dependencies")
    executable = OUTPUT / "NativeCreatorHostCheck"
    run("swift-release", ["xcrun", "swiftc", "-O", "-g", "-parse-as-library", "-swift-version", "5",
                          "-target", f"{platform.machine()}-apple-macosx26.1",
                          "-import-objc-header", str(ROOT / "Xcode/Eldiron/Eldiron Shared/Bridge.h"),
                          str(ROOT / "Xcode/Eldiron/Tests/NativeCreatorHostCheck.swift"),
                          "-L", str(TARGET / "release"), "-leldiron_creator",
                          *shlex.split(match.group(1)), "-o", str(executable)])
    run("host", [str(executable)], timeout=180)
    print(f"All detached-layer Creator checks passed: {OUTPUT}")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
