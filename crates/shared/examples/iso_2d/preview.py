"""Compose the verified page's sprite calls; requires Pillow, no browser/GPU.

These previews cover sprite placement/occlusion, not DOM styling or Canvas shadows.
"""
import json
import sys
from pathlib import Path
from PIL import Image

out = Path(sys.argv[1] if len(sys.argv) > 1 else "generated/mz-iso2d")
data = json.loads((out / "preview-frames.json").read_text())
manifest = json.loads((out / "manifest.json").read_text())
images = {key: Image.open(out / f"{key}.png").convert("RGBA") for key in manifest["images"]}


def composite(calls, transparent=False, walls_only=False):
    result = Image.new("RGBA", (1000, 660), (0, 0, 0, 0) if transparent else (17, 34, 29, 255))
    for call in calls:
        key = call["image"]
        if walls_only and not key.startswith(("wall", "corner")):
            continue
        sx, sy, sw, sh, dx, dy, dw, dh = call["rect"]
        sprite = images[key].crop((sx, sy, sx + sw, sy + sh))
        assert (sw, sh) == (dw, dh), "Unexpected runtime sprite scaling"
        result.alpha_composite(sprite, (round(dx), round(dy)))
    return result


for snapshot in data["snapshots"]:
    composite(snapshot["calls"]).save(out / f"{snapshot['name']}.png")
walls = composite(data["snapshots"][0]["calls"], transparent=True, walls_only=True)
box = walls.getbbox()
closeup = walls.crop((box[0] - 8, box[1] - 8, box[2] + 8, box[3] + 8))
closeup.resize((closeup.width * 3, closeup.height * 3), Image.Resampling.NEAREST).save(out / "wall-joins.png")
# Check all 16 joins, at several masonry heights and at the coping.
# A 3x3 neighborhood accommodates pixel-centre rasterization at projected edges.
count = 0
for t in (3.5, 4.5, 5.5, 6.5):
    for x, z in ((t, 3.11), (t, 7.11), (3.11, t), (7.11, t)):
        for h in (.2, .5, .8, 1.05, 1.2):
            px = round(data["origin"]["x"] + (x + z) * 32)
            py = round(data["origin"]["y"] + (z - x) * 16 - h * data["projection"][2])
            alpha = [walls.getpixel((px + dx, py + dy))[3] for dx in (-1, 0, 1) for dy in (-1, 0, 1)]
            assert max(alpha) == 255, f"Transparent wall joint at {x}, {z}, height {h}"
            count += 1
frames = [composite(calls).resize((800, 528), Image.Resampling.NEAREST).convert("RGB") for calls in data["walk"]]
frames[0].save(out / "walk.gif", save_all=True, append_images=frames[1:], duration=200, loop=0)
print(f"Verified {count} opaque wall/corner join samples; saved five scene previews, wall-joins.png and walk.gif.")
