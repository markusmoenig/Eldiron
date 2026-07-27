#!/usr/bin/env python3

"""Turn a generated square material image into an exactly seamless pixel-art tile."""

from __future__ import annotations

import argparse
import math
from pathlib import Path

from PIL import Image, ImageChops


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--source-size", type=int, default=768)
    parser.add_argument("--output-size", type=int, default=256)
    parser.add_argument("--blend-width", type=int, default=32)
    args = parser.parse_args()

    image = Image.open(args.input).convert("RGB")
    crop_size = min(args.source_size, image.width, image.height)
    left = (image.width - crop_size) // 2
    top = (image.height - crop_size) // 2
    base = image.crop((left, top, left + crop_size, top + crop_size))
    tile = base.resize(
        (args.output_size, args.output_size),
        Image.Resampling.NEAREST,
    )

    size = args.output_size
    blend_width = max(2, min(args.blend_width, size // 2))

    def blend_channel(value: int, average: float, strength: float) -> int:
        return round(value * (1.0 - strength) + average * strength)

    horizontal_source = tile.copy()
    source_pixels = horizontal_source.load()
    pixels = tile.load()
    for y in range(size):
        for distance in range(blend_width):
            strength = (math.cos(math.pi * distance / (blend_width - 1)) + 1.0) * 0.5
            left_pixel = source_pixels[distance, y]
            right_x = size - 1 - distance
            right_pixel = source_pixels[right_x, y]
            average = tuple((left_pixel[c] + right_pixel[c]) * 0.5 for c in range(3))
            pixels[distance, y] = tuple(
                blend_channel(left_pixel[c], average[c], strength) for c in range(3)
            )
            pixels[right_x, y] = tuple(
                blend_channel(right_pixel[c], average[c], strength) for c in range(3)
            )

    vertical_source = tile.copy()
    source_pixels = vertical_source.load()
    pixels = tile.load()
    for x in range(size):
        for distance in range(blend_width):
            strength = (math.cos(math.pi * distance / (blend_width - 1)) + 1.0) * 0.5
            top_pixel = source_pixels[x, distance]
            bottom_y = size - 1 - distance
            bottom_pixel = source_pixels[x, bottom_y]
            average = tuple((top_pixel[c] + bottom_pixel[c]) * 0.5 for c in range(3))
            pixels[x, distance] = tuple(
                blend_channel(top_pixel[c], average[c], strength) for c in range(3)
            )
            pixels[x, bottom_y] = tuple(
                blend_channel(bottom_pixel[c], average[c], strength) for c in range(3)
            )

    left_edge = tile.crop((0, 0, 1, size))
    right_edge = tile.crop((size - 1, 0, size, size))
    top_edge = tile.crop((0, 0, size, 1))
    bottom_edge = tile.crop((0, size - 1, size, size))
    if ImageChops.difference(left_edge, right_edge).getbbox() is not None:
        raise RuntimeError("left and right edges do not match")
    if ImageChops.difference(top_edge, bottom_edge).getbbox() is not None:
        raise RuntimeError("top and bottom edges do not match")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    tile.save(args.output, format="PNG", optimize=True)


if __name__ == "__main__":
    main()
