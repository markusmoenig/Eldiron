#!/usr/bin/env python3
"""Curate native Eldiron captures into the docs; no image synthesis or UI overlays.

Run the capture tests described in docs/static/img/materialz/README.md first.
Requires Pillow. Generated projects, full captures and logs remain ignored.
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'docs/static/img/materialz'
CAPTURES = [
    ('/tmp/eldiron-procedural-workspace.png', 'material-editor.png', (820, 0, 1200, 850)),
    ('generated/mz-recipe-browser/browser.png', 'recipe-browser.png', None),
    ('generated/mz-recipe-browser/derived-publisher.png', 'publish-controls.png', (168, 55, 934, 724)),
    ('generated/mz-recipe-browser/inputs-filtered.png', 'recipe-inputs.png', None),
    ('generated/mz-docs/iso-placement-sidebar.png', 'placement.png', (1020, 0, 1400, 400)),
    ('generated/mz-docs/mesh-controls.png', 'mesh-controls.png', (1020, 0, 1400, 840)),
    ('generated/mz-docs/connector-editor.png', 'connector-editor.png', (1020, 90, 1400, 845)),
    ('generated/mz-iso2d-recipe-playground/outside.png', 'room.png', None),
    ('generated/mz-iso2d-recipe-playground/inside.png', 'room-interior.png', None),
    ('generated/mz-iso2d-recipe-playground/connector-room.png', 'connector-room.png', None),
    ('generated/iso-terrain-paint/painted.png', 'terrain-painted.png', None),
    ('generated/iso-terrain-paint/sculpted.png', 'terrain-sculpted.png', None),
]

def main():
    missing = [str(ROOT / source) for source, _, _ in CAPTURES if not (ROOT / source).is_file()]
    if missing:
        raise SystemExit('Run native capture tests first. Missing:\n' + '\n'.join(missing))
    DEST.mkdir(parents=True, exist_ok=True)
    for source, name, crop in CAPTURES:
        with Image.open(ROOT / source) as image:
            if crop:
                image = image.crop(crop)
            image.save(DEST / name, optimize=True)
    print(f'Updated {len(CAPTURES)} native screenshots in {DEST}')

if __name__ == '__main__':
    main()
