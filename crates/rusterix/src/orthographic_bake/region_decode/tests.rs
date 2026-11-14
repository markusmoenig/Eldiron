use super::*;
use crate::orthographic_bake::{BASE64, decode_png};
use base64::Engine as _;
use std::io::Write;

fn crop(bytes: &[u8], size: u32, region: OriginalRegion) -> Vec<u8> {
    let mut result = Vec::new();
    for y in region.origin[1]..region.origin[1] + region.size[1] {
        let at = (y as usize * size as usize + region.origin[0] as usize) * 4;
        result.extend_from_slice(&bytes[at..at + region.size[0] as usize * 4]);
    }
    result
}
fn encoded_png(size: u32, color: ::png::ColorType, bits: ::png::BitDepth) -> String {
    let channels = color.samples();
    let mut bytes = Vec::new();
    for at in 0..size as usize * size as usize * channels {
        if bits == ::png::BitDepth::Sixteen {
            bytes.extend_from_slice(&((at.wrapping_mul(1531)) as u16).to_be_bytes());
        } else {
            bytes.push((at * 37) as u8);
        }
    }
    let mut png_bytes = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut png_bytes, size, size);
    encoder.set_color(color);
    encoder.set_depth(bits);
    if color == ::png::ColorType::Indexed {
        let palette: Vec<u8> = (0..256)
            .flat_map(|i| [i as u8, (255 - i) as u8, (i * 3) as u8])
            .collect();
        encoder.set_palette(palette);
        encoder.set_trns((0..256).map(|i| i as u8).collect::<Vec<_>>());
    }
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&bytes).unwrap();
    writer.finish().unwrap();
    BASE64.encode(png_bytes)
}
#[test]
fn row_rectangles_match_current_full_decoder_for_8_and_16_bit_color_and_transparency() {
    let region = OriginalRegion {
        origin: [3, 5],
        size: [9, 7],
    };
    for color in [
        ::png::ColorType::Grayscale,
        ::png::ColorType::GrayscaleAlpha,
        ::png::ColorType::Rgb,
        ::png::ColorType::Rgba,
        ::png::ColorType::Indexed,
    ] {
        for bits in [::png::BitDepth::Eight, ::png::BitDepth::Sixteen] {
            if color == ::png::ColorType::Indexed && bits == ::png::BitDepth::Sixteen {
                continue;
            }
            let source = encoded_png(23, color, bits);
            let expected = decode_png(&source, 23).unwrap();
            let mut work = PngRegionDecoder::new(source, 23, region).unwrap();
            assert!(!work.advance(0).unwrap());
            while !work.advance(1).unwrap() {}
            assert_eq!(work.decoded_rows(), 23);
            assert_eq!(work.output_bytes(), 9 * 7 * 4);
            assert_eq!(
                work.finish().unwrap(),
                crop(&expected, 23, region),
                "{color:?}/{bits:?}"
            );
        }
    }
}

// A tiny independent PNG encoder builds Adam7 scanlines so the oracle checks
// exact pass placement rather than sharing the decoder's row schedule.
fn interlaced(size: u32) -> String {
    let mut scanlines = Vec::new();
    for (x0, y0, dx, dy) in [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ] {
        if x0 >= size || y0 >= size {
            continue;
        }
        for y in (y0..size).step_by(dy) {
            scanlines.push(0);
            for x in (x0..size).step_by(dx) {
                scanlines.extend_from_slice(&[
                    x as u8,
                    (y * 3) as u8,
                    (x + y) as u8,
                    (x * 7 + y) as u8,
                ]);
            }
        }
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&scanlines).unwrap();
    let compressed = z.finish().unwrap();
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc = 0xffff_ffffu32;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320u32.wrapping_mul(crc & 1));
            }
        }
        out.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = size.to_be_bytes().to_vec();
    header.extend_from_slice(&size.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 1]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &compressed);
    chunk(&mut out, b"IEND", &[]);
    BASE64.encode(out)
}
#[test]
fn odd_interlaced_originals_scatter_into_exact_small_rectangles() {
    for size in [1, 2, 3, 7, 13, 31] {
        let source = interlaced(size);
        let expected = decode_png(&source, size).unwrap();
        let region = OriginalRegion {
            origin: [size / 2, size / 3],
            size: [size - size / 2, size - size / 3],
        };
        assert_eq!(
            decode_png_region(&source, size, region).unwrap(),
            crop(&expected, size, region)
        );
    }
}
#[test]
fn depth_regions_preserve_original_ieee_payload_bits_and_cooperative_retirement() {
    let size = 23;
    let bits: Vec<u32> = (0..size * size)
        .map(|at| match at % 5 {
            0 => f32::INFINITY.to_bits(),
            1 => 0x7fc01234,
            2 => (at as f32 * 0.013).to_bits(),
            3 => 0x80000000,
            _ => f32::NEG_INFINITY.to_bits(),
        })
        .collect();
    let bytes: Vec<u8> = bits.iter().flat_map(|n| n.to_le_bytes()).collect();
    let source = BASE64.encode(bytes);
    let region = OriginalRegion {
        origin: [4, 7],
        size: [9, 11],
    };
    let mut work = DepthRegionDecoder::new(source.clone(), size, region).unwrap();
    assert!(!work.advance(1).unwrap());
    assert!(work.finish().is_err());
    let result = decode_depth_region(&source, size, region).unwrap();
    for y in 0..region.size[1] {
        for x in 0..region.size[0] {
            assert_eq!(
                result[(y * region.size[0] + x) as usize].to_bits(),
                bits[((y + region.origin[1]) * size + x + region.origin[0]) as usize]
            );
        }
    }
    assert!(DepthRegionDecoder::new(source, u32::MAX, region).is_err());
}
#[test]
fn large_valid_original_retains_rows_and_requested_pixels_without_a_full_image() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../creator/tests/fixtures/oversized-current-bake.json"
    ))
    .unwrap();
    let source = fixture["tiles"][0]["color_png_base64"].as_str().unwrap();
    let region = OriginalRegion {
        origin: [1999, 1977],
        size: [49, 71],
    };
    let mut work = PngRegionDecoder::new(source, 2048, region).unwrap();
    let mut yields = 0;
    while !work.advance(16).unwrap() {
        yields += 1;
    }
    assert!(yields >= 128);
    assert_eq!(work.decoded_rows(), 2048);
    assert_eq!(work.max_row_bytes(), 8192);
    assert_eq!(work.output_bytes(), 49 * 71 * 4);
    assert_eq!(work.finish().unwrap(), [48, 80, 160, 255].repeat(49 * 71));
}
#[test]
fn invalid_rectangles_headers_and_late_corruption_cannot_publish_partial_pixels() {
    let good = encoded_png(23, ::png::ColorType::Rgba, ::png::BitDepth::Eight);
    for region in [
        OriginalRegion {
            origin: [0; 2],
            size: [0, 1],
        },
        OriginalRegion {
            origin: [22, 0],
            size: [2, 1],
        },
        OriginalRegion {
            origin: [u32::MAX, 0],
            size: [1, 1],
        },
        OriginalRegion {
            origin: [0; 2],
            size: [513, 1],
        },
    ] {
        assert!(PngRegionDecoder::new(&good, 23, region).is_err());
    }
    let region = OriginalRegion {
        origin: [0; 2],
        size: [9; 2],
    };
    assert!(
        PngRegionDecoder::new(&good, 23, region)
            .unwrap()
            .finish()
            .is_err()
    );
    let mut bytes = BASE64.decode(&good).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    let mut corrupt = PngRegionDecoder::new(BASE64.encode(bytes), 23, region).unwrap();
    while corrupt.advance(1).is_ok_and(|done| !done) {}
    assert!(corrupt.finish().is_err());
    assert!(decode_png_region(&good, 24, region).is_err());
    assert!(decode_png_region(&(good + "!!!!"), 23, region).is_err());
}

#[test]
fn owned_original_text_survives_source_retirement_without_an_encoded_copy() {
    let original = theframework_core::SharedText::from(encoded_png(
        23,
        ::png::ColorType::Rgba,
        ::png::BitDepth::Eight,
    ));
    let retained = original.clone();
    assert_eq!(original.as_ptr(), retained.as_ptr());
    let region = OriginalRegion {
        origin: [4, 7],
        size: [9, 11],
    };
    let expected = crop(&decode_png(original.as_str(), 23).unwrap(), 23, region);
    let mut decoder = PngRegionDecoder::new(retained, 23, region).unwrap();
    drop(original);
    assert!(!decoder.advance(1).unwrap());
    while !decoder.advance(8).unwrap() {}
    assert_eq!(decoder.finish().unwrap(), expected);
}
