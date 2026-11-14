#!/usr/bin/env python3
"""Copy a current painted archive and add ordered original organic stamps.

The two world anchors share identical artwork. The default 300 placements exceed
Creator's former worst-case stamp-count guard while occupying little actual
sprite storage. Existing geometry, original UV paint and bake sidecars remain.
"""
import argparse
import json
from pathlib import Path
import uuid
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--stamps", type=int, default=300)
    args = parser.parse_args()
    if Path(args.input).resolve() == Path(args.output).resolve() or args.stamps < 1:
        parser.error("use a separate output and a positive stamp count")
    with zipfile.ZipFile(args.input) as source:
        project = json.loads(source.read("project.json"))
        layer = project["regions"][0]["iso_paint"]
        stamps = []
        for number in range(args.stamps):
            stamps.append({
                "id": str(uuid.uuid4()), "order": number, "kind": "grass",
                "screen": [8, 8], "world": [-0.4 if number % 2 == 0 else 0.4, 0.03, 0.25],
                "owner": None, "surface_uv": None, "paint_geo": None,
                "surface_normal": None, "sort_depth": 8.0, "size": 1.0,
                "camera_scale": 3.0, "viewport_size": [256, 256],
                "variant": "default", "rotation": 0.0, "variation": 12345,
                "material_id": 0, "color": [230, 70, 30, 255],
                "palette_indices": [], "palette_colors": [], "opacity": 1.0,
                "screen_bounds": [3, -5, 13, 13],
            })
        layer["visible"] = True
        layer["chunks"] = {"0:0": {
            "origin": [0, 0], "revision": 1, "stamp_revision": 1,
            "strokes": [], "stamps": stamps,
        }}
        with zipfile.ZipFile(args.output, "w", compression=zipfile.ZIP_DEFLATED) as output:
            for item in source.infolist():
                data = (json.dumps(project, separators=(",", ":")).encode()
                        if item.filename == "project.json" else source.read(item.filename))
                output.writestr(item, data)
    print(json.dumps({"stamps": args.stamps, "output": args.output}))


if __name__ == "__main__":
    main()
