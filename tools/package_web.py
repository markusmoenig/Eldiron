#!/usr/bin/env python3
"""Build/package the shared WebGPU browser host plus its CPU-only codec worker.

Run from any directory; output is a self-contained local HTTP directory.
Project archives are supplied separately (index.html?project=Project.eldiron).
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("application", choices=["creator", "player"])
    parser.add_argument("output", type=Path)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    base = ["--locked"] + (["--offline"] if args.offline else [])
    def cargo(command, options):
        subprocess.run(["cargo", command] + base + options, cwd=ROOT, check=True)
    cargo("build", ["-p", "rusterix", "--no-default-features", "--features", "browser-codec-worker", "--bin", "eldiron-bake-codec-worker", "--release", "--target", "wasm32-unknown-unknown"])
    if args.application == "creator":
        binary = "eldiron-creator-web"
        package = "eldiron-creator"
        features = "browser-host"
    else:
        binary = "eldiron-client-wgpu"
        package = "eldiron-client-wgpu"
        features = "native-window"
    cargo("build", ["-p", package, "--bin", binary, "--no-default-features", "--features", features, "--release", "--target", "wasm32-unknown-unknown"])
    for name in ["eldiron-bake-codec-worker", binary]:
        cargo("run", ["-p", "run-wasm", "--bin", "eldiron-package-web", "--", str(target / "wasm32-unknown-unknown/release" / (name + ".wasm")), str(output)])
    shutil.copyfile(ROOT / "web/eldiron-bake-codec-worker-bootstrap.js", output / "eldiron-bake-codec-worker-bootstrap.js")
    (output / "index.html").write_text('''<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Eldiron WebGPU</title></head><body><script type="module">import('./''' + binary + '''.js').then(m=>m.default()).catch(error=>{document.documentElement.dataset.eldironState='error';document.body.textContent=String(error);console.error(error);});</script></body></html>''')
    print("Packaged", args.application, "and CPU codec worker into", output)

if __name__ == "__main__":
    main()
