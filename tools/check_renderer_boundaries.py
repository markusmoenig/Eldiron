#!/usr/bin/env python3
"""Check the rewrite's independently resolved Cargo feature boundaries.

Run from any directory. This intentionally resolves each consumer separately:
checking a combined workspace build can mask a broken headless configuration.
"""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
GPU = {"wgpu", "wgpu-core", "wgpu-hal", "renderkit", "winit", "pixels", "softbuffer"}
TEXT_UI = {"parley", "swash", "zeno"}


def tree(package, features=None, target=None):
    args = ["cargo", "tree", "--offline", "-p", package, "--edges", "normal,features", "--prefix", "none"]
    if target is not None:
        args.extend(["--target", target])
    if features is not None:
        args.extend(["--no-default-features", "--features", features])
    return subprocess.check_output(args, cwd=ROOT, text=True)


def check(package, forbidden, features=None, no_widget_ui=False, forbidden_features=(), target=None):
    output = tree(package, features, target)
    packages = set(re.findall(r"^([A-Za-z0-9_-]+) v[0-9]", output, re.MULTILINE))
    failures = sorted(packages & forbidden)
    if no_widget_ui and 'theframework feature "ui"' in output:
        failures.append('theframework/ui')
    failures.extend(feature for feature in forbidden_features if feature in output)
    if failures:
        raise RuntimeError(f"{package}: unexpected dependencies: {', '.join(failures)}")
    print(f"{package}{' [' + features + ']' if features else ''}{' (' + target + ')' if target else ''}: boundary passed")


def main():
    check("theframework-core", GPU | TEXT_UI | {"theframework", "fontdue"})
    check("eldiron-ruleset", GPU | TEXT_UI | {"theframework", "fontdue"})
    check("eldiron-runtime", GPU | TEXT_UI, no_widget_ui=True)
    check("eldiron-client-terminal", GPU | TEXT_UI, no_widget_ui=True)
    check("theframework", GPU, features="ui-paint")
    check("theframework", {"winit", "pixels", "softbuffer"}, features="render-wgpu", no_widget_ui=True)
    check("theframework", {"pixels", "softbuffer"}, features="platform-winit", no_widget_ui=True)
    check("eldiron-client", {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-client-wgpu", {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-client-wgpu", {"winit", "pixels", "softbuffer"}, features="native-player")
    check("eldiron-creator", {"pixels", "softbuffer"}, features="native-host", forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-creator", {"pixels", "softbuffer", "open", "arboard", "ureq", "font-kit"}, features="browser-host", forbidden_features=('scenevm feature "windowing"', 'theframework feature "winit_app"'), target="wasm32-unknown-unknown")
    for package in ("eldiron-client", "eldiron-client-wgpu"):
        check(package, {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',), target="wasm32-unknown-unknown")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
