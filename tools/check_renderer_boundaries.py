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


def tree(package, features=None):
    args = ["cargo", "tree", "--offline", "-p", package, "--edges", "normal,features", "--prefix", "none"]
    if features is not None:
        args.extend(["--no-default-features", "--features", features])
    return subprocess.check_output(args, cwd=ROOT, text=True)


def check(package, forbidden, features=None, no_widget_ui=False):
    output = tree(package, features)
    packages = set(re.findall(r"^([A-Za-z0-9_-]+) v[0-9]", output, re.MULTILINE))
    failures = sorted(packages & forbidden)
    if no_widget_ui and 'theframework feature "ui"' in output:
        failures.append('theframework/ui')
    if failures:
        raise RuntimeError(f"{package}: unexpected dependencies: {', '.join(failures)}")
    print(f"{package}{' [' + features + ']' if features else ''}: boundary passed")


def main():
    check("theframework-core", GPU | TEXT_UI | {"theframework", "fontdue"})
    check("eldiron-ruleset", GPU | TEXT_UI | {"theframework", "fontdue"})
    check("eldiron-runtime", GPU | TEXT_UI, no_widget_ui=True)
    check("eldiron-client-terminal", GPU | TEXT_UI, no_widget_ui=True)
    check("theframework", GPU, features="ui-paint")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
