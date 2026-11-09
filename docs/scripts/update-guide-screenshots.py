#!/usr/bin/env python3
"""Curate actual Creator captures for current guides. Requires Pillow.

First run the native documentation_screenshots capture described in
docs/static/img/creator/README.md. Crops retain the original UI pixels.
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'generated/docs-current'
DESTINATION = ROOT / 'docs/static/img/creator'
CAPTURES = [
    ('creator-overview', 'overview', None),
    ('rect-tool', 'rect-tool', None),
    ('rect-tool', 'tile-picker', (50, 698, 1060, 960)),
    ('tile-metadata', 'tile-metadata', (1060, 510, 1440, 770)),
    ('tileset-editor', 'tileset-editor', None),
    ('character-attributes', 'entity-nodes', (250, 440, 1050, 630)),
    ('behavior-nodes', 'behavior-nodes', (270, 410, 1045, 660)),
    ('game-settings', 'game-settings', (50, 87, 1060, 805)),
    ('authoring', 'authoring', None),
    ('screen-editor', 'screen-editor', None),
    ('linedef-tool', 'linedef-tool', (0, 87, 1060, 699)),
    ('selected-sector', 'selected-sector', (0, 87, 1060, 699)),
    ('selected-sector', 'map-hud', (0, 87, 1060, 160)),
    ('applied-tile', 'applied-tile', None),
    ('3d-editor', '3d-editor', None),
    ('3d-paint', '3d-paint', None),
    ('prefab-tool', 'prefab-tool', None),
]


def main():
    missing = sorted({str(SOURCE / f'{source}.png') for source, _, _ in CAPTURES
                      if not (SOURCE / f'{source}.png').is_file()})
    if missing:
        raise SystemExit('Run the native capture first. Missing:\n' + '\n'.join(missing))
    DESTINATION.mkdir(parents=True, exist_ok=True)
    for source, name, crop in CAPTURES:
        with Image.open(SOURCE / f'{source}.png') as image:
            if image.size != (1440, 960):
                raise SystemExit(f'Unexpected capture dimensions: {source}: {image.size}')
            if crop:
                image = image.crop(crop)
            image.save(DESTINATION / f'{name}.png', optimize=True)
    print(f'Updated {len(CAPTURES)} native screenshots in {DESTINATION}')


if __name__ == '__main__':
    main()
