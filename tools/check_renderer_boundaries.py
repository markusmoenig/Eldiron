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
CPU_ONLY = GPU | TEXT_UI | {"theframework", "fontdue", "rfd"}


def tree(package, features=None, target=None):
    args = ["cargo", "tree", "--locked", "--offline", "-p", package, "--edges", "normal,features", "--prefix", "none"]
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


def check_shared_graphical_owner_imports():
    # Project data may use CPU engine types; graphical aggregate setup belongs
    # outward in presentation even when graphics features are unified.
    for path in (ROOT / "crates/shared/src").rglob("*.rs"):
        source = path.read_text()
        imports = re.findall(r"\buse\s+rusterix::[^;]+;", source)
        if any(re.search(r"\b(?:Rusterix|Client)\b", item) for item in imports) or re.search(r"\brusterix::(?:Rusterix|Client)\b", source):
            raise RuntimeError(f"{path.relative_to(ROOT)}: graphical aggregate must live in presentation")
    print("shared source: no graphical aggregate imports")


def main():
    check_shared_graphical_owner_imports()
    check("theframework-core", CPU_ONLY)
    check("eldiron-ruleset", CPU_ONLY)
    check("scenevm", CPU_ONLY, features="")
    check("rusterix", CPU_ONLY, features="")
    check("eldiron-shared", CPU_ONLY, features="")
    check("eldiron-runtime", CPU_ONLY)
    check("rusterix", CPU_ONLY, features="browser-codec-worker", target="wasm32-unknown-unknown")
    check("eldiron-client-terminal", CPU_ONLY)
    for package in ("scenevm", "rusterix", "eldiron-shared", "eldiron-runtime"):
        check(package, CPU_ONLY, features="", target="wasm32-unknown-unknown")
    check("theframework", GPU, features="ui-paint")
    check("theframework", {"winit", "pixels", "softbuffer"}, features="render-wgpu", no_widget_ui=True)
    check("theframework", {"pixels", "softbuffer"}, features="platform-winit", no_widget_ui=True)
    check("eldiron-client", {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-client-wgpu", {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-client-wgpu", {"winit", "pixels", "softbuffer"}, features="native-player")
    check("eldiron-creator", {"pixels", "softbuffer"}, forbidden_features=('scenevm feature "windowing"',))
    check("eldiron-creator", {"pixels", "softbuffer", "open", "arboard", "ureq", "font-kit"}, forbidden_features=('scenevm feature "windowing"', 'theframework feature "winit_app"'), target="wasm32-unknown-unknown")
    check("eldiron-creator", {"winit", "pixels", "softbuffer"}, features="staticlib", forbidden_features=('scenevm feature "windowing"',))
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
