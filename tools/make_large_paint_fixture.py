#!/usr/bin/env python3
"""Extend a current-paint archive with distinct, unvisited original UV pages.

Keeps existing geometry/paint/bake sidecars intact. Streaming the extra pages
avoids constructing a second full source document just to generate the fixture.
Usage: python3 tools/make_large_paint_fixture.py input.eldiron output.eldiron
"""
import argparse
import json
from pathlib import Path
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--pages", type=int, default=2051)
    args = parser.parse_args()
    if Path(args.input).resolve() == Path(args.output).resolve():
        parser.error("output must be a separate fixture archive")
    compact = lambda value: json.dumps(value, separators=(",", ":")).encode()
    with zipfile.ZipFile(args.input) as source:
        project = json.loads(source.read("project.json"))
        layer = project["regions"][0]["iso_paint"]
        chunks = layer["baked_chunks"]
        if not chunks or args.pages <= len(chunks):
            parser.error("input needs current paint; --pages must exceed its page count")
        prototype = next(iter(chunks.values()))
        if len(prototype["color_rgba"]) != 16384 or len(prototype["material_rgba"]) != 16384:
            parser.error("fixture requires original 64x64 RGBA page pairs")
        marker = "__eldiron_large_paint_chunks__"
        layer["baked_chunks"] = marker
        prefix, suffix = compact(project).split(compact(marker))
        with zipfile.ZipFile(args.output, "w", compression=zipfile.ZIP_DEFLATED) as target:
            for item in source.infolist():
                if item.filename != "project.json":
                    target.writestr(item, source.read(item.filename))
            with target.open("project.json", "w", force_zip64=True) as output:
                output.write(prefix + b"{")
                for number, (key, chunk) in enumerate(chunks.items()):
                    if number:
                        output.write(b",")
                    output.write(compact(key) + b":" + compact(chunk))
                for number in range(len(chunks), args.pages):
                    chunk = dict(prototype)
                    chunk["origin"] = [number * 64, 8192]
                    chunk["color_rgba"] = prototype["color_rgba"].copy()
                    chunk["material_rgba"] = prototype["material_rgba"].copy()
                    chunk["color_rgba"][:4] = [number % 256, number // 256 % 256, 72, 255]
                    chunk["material_rgba"][1] = number % 256
                    key = "large-unvisited:" + str(number)
                    output.write(b"," + compact(key) + b":" + compact(chunk))
                output.write(b"}" + suffix)
    print(json.dumps({"pages": args.pages, "logical_plane_bytes": args.pages * 32768,
                      "output": args.output}))


if __name__ == "__main__":
    main()
