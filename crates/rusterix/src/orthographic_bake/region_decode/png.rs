use super::{OriginalRegion, base64_source::Base64Source};
use std::io::{BufReader, Read, Seek, SeekFrom};

// Independent of the original image area; metadata which is not used for raw
// original channels is ignored. PNG row/filter/inflater storage stays bounded.
const CODEC_BYTES: usize = 8 * 1024 * 1024;
type Reader<S> = ::png::Reader<BufReader<Base64Source<S>>>;

/// Retains the caller's encoded source owner, bounded PNG rows and one small
/// output rectangle. S can be a borrowed str or an owned/shared source handle.
pub struct PngRegionDecoder<S: AsRef<str>> {
    source: Option<Base64Source<S>>,
    reader: Option<Reader<S>>,
    original: u32,
    region: OriginalRegion,
    output: Vec<u8>,
    rows: Rows,
    channels: usize,
    sample_bytes: usize,
    validated_bytes: u64,
    decoded_rows: u64,
    max_row_bytes: usize,
    ready: bool,
    failure: Option<String>,
}
impl<S: AsRef<str>> PngRegionDecoder<S> {
    pub fn new(encoded: S, original: u32, region: OriginalRegion) -> Result<Self, String> {
        let pixels = region.validate(original)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(pixels * 4)
            .map_err(|e| e.to_string())?;
        output.resize(pixels * 4, 0);
        Ok(Self {
            source: Some(Base64Source::new(encoded)?),
            reader: None,
            original,
            region,
            output,
            rows: Rows::new(original, false),
            channels: 0,
            sample_bytes: 0,
            validated_bytes: 0,
            decoded_rows: 0,
            max_row_bytes: 0,
            ready: false,
            failure: None,
        })
    }
    /// Each unit validates at most3072 original binary bytes or decodes one
    /// original row/pass-row. Header/finalization is a separate unit. Owners can
    /// yield/cancel between advances and publish only after `finish` succeeds.
    pub fn advance(&mut self, units: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.advance_inner(units);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(&mut self, units: usize) -> Result<bool, String> {
        for _ in 0..units {
            if self.ready {
                break;
            }
            if let Some(source) = &mut self.source {
                let mut bytes = [0u8; 3072];
                let count = source.read(&mut bytes).map_err(|e| e.to_string())?;
                self.validated_bytes += count as u64;
                if count > 0 {
                    continue;
                }
                let mut source = self.source.take().unwrap();
                source.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
                let mut decoder = ::png::Decoder::new_with_limits(
                    BufReader::new(source),
                    ::png::Limits { bytes: CODEC_BYTES },
                );
                decoder.set_transformations(::png::Transformations::EXPAND);
                decoder.set_ignore_text_chunk(true);
                decoder.set_ignore_iccp_chunk(true);
                // Check worst-case 16-bit RGBA row storage before the decoder
                // can allocate its row/filter buffers, including malformed or
                // extremely wide compressed originals.
                let info = decoder.read_header_info().map_err(|e| e.to_string())?;
                if (info.width, info.height) != (self.original, self.original) {
                    return Err("Original PNG dimensions do not match the bake tile".into());
                }
                if u64::from(info.width) * 8 > CODEC_BYTES as u64 {
                    return Err("Original PNG row exceeds bounded codec scratch".into());
                }
                let reader = decoder.read_info().map_err(|e| e.to_string())?;
                if (reader.info().width, reader.info().height) != (self.original, self.original) {
                    return Err("Original PNG dimensions do not match the bake tile".into());
                }
                self.channels = match reader.output_color_type().0 {
                    ::png::ColorType::Grayscale => 1,
                    ::png::ColorType::GrayscaleAlpha => 2,
                    ::png::ColorType::Rgb => 3,
                    ::png::ColorType::Rgba => 4,
                    _ => return Err("Original PNG color type was not expanded".into()),
                };
                self.sample_bytes = match reader.output_color_type().1 {
                    ::png::BitDepth::Eight => 1,
                    ::png::BitDepth::Sixteen => 2,
                    _ => return Err("Original PNG bit depth was not expanded".into()),
                };
                self.rows = Rows::new(self.original, reader.info().interlaced);
                self.reader = Some(reader);
                continue;
            }
            let reader = self
                .reader
                .as_mut()
                .ok_or("Original PNG decoder has no source")?;
            let Some((y, x0, step)) = self.rows.next() else {
                if reader.next_row().map_err(|e| e.to_string())?.is_some() {
                    return Err("Original PNG has unexpected rows".into());
                }
                reader.finish().map_err(|e| e.to_string())?;
                self.ready = true;
                continue;
            };
            let row = reader
                .next_row()
                .map_err(|e| e.to_string())?
                .ok_or("Original PNG ended before its declared rows")?;
            let row = row.data();
            let samples = (self.original - x0).div_ceil(step) as usize;
            if row.len() != samples * self.channels * self.sample_bytes {
                return Err("Original PNG pass row has invalid dimensions".into());
            }
            self.decoded_rows += 1;
            self.max_row_bytes = self.max_row_bytes.max(row.len());
            if y < self.region.origin[1] || y >= self.region.origin[1] + self.region.size[1] {
                continue;
            }
            for local_x in 0..self.region.size[0] {
                let x = self.region.origin[0] + local_x;
                if x < x0 || (x - x0) % step != 0 {
                    continue;
                }
                let offset = (x - x0) as usize / step as usize * self.channels * self.sample_bytes;
                let component = |at: usize| -> u8 {
                    if self.sample_bytes == 1 {
                        row[offset + at]
                    } else {
                        let index = offset + at * 2;
                        let value = u16::from_be_bytes([row[index], row[index + 1]]) as u32;
                        ((value * 255 + 32767) / 65535) as u8
                    }
                };
                let pixel = match self.channels {
                    1 => {
                        let value = component(0);
                        [value, value, value, 255]
                    }
                    2 => {
                        let value = component(0);
                        [value, value, value, component(1)]
                    }
                    3 => [component(0), component(1), component(2), 255],
                    4 => [component(0), component(1), component(2), component(3)],
                    _ => unreachable!(),
                };
                let at = ((y - self.region.origin[1]) as usize * self.region.size[0] as usize
                    + local_x as usize)
                    * 4;
                self.output[at..at + 4].copy_from_slice(&pixel);
            }
        }
        Ok(self.ready)
    }
    pub fn decoded_rows(&self) -> u64 {
        self.decoded_rows
    }
    pub fn validated_bytes(&self) -> u64 {
        self.validated_bytes
    }
    pub fn max_row_bytes(&self) -> usize {
        self.max_row_bytes
    }
    pub fn output_bytes(&self) -> usize {
        self.output.len()
    }
    pub fn finish(self) -> Result<Vec<u8>, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.ready {
            return Err("Original PNG rectangle is incomplete".into());
        }
        Ok(self.output)
    }
}

/// Own the standard Adam7 row schedule without relying on png's private pass
/// fields or allocating an expanded interlaced image. Coordinates are absolute.
struct Rows {
    size: u32,
    passes: &'static [(u32, u32, u32, u32)],
    pass: usize,
    line: u32,
}
impl Rows {
    fn new(size: u32, interlaced: bool) -> Self {
        const NORMAL: &[(u32, u32, u32, u32)] = &[(0, 0, 1, 1)];
        const ADAM7: &[(u32, u32, u32, u32)] = &[
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        Self {
            size,
            passes: if interlaced { ADAM7 } else { NORMAL },
            pass: 0,
            line: 0,
        }
    }
    fn next(&mut self) -> Option<(u32, u32, u32)> {
        loop {
            let &(x, y, dx, dy) = self.passes.get(self.pass)?;
            let row = u64::from(y) + u64::from(self.line) * u64::from(dy);
            if x >= self.size || row >= u64::from(self.size) {
                self.pass += 1;
                self.line = 0;
                continue;
            }
            self.line += 1;
            return Some((row as u32, x, dx));
        }
    }
}
