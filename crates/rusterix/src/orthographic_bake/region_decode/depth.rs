use super::{OriginalRegion, base64_source::Base64Source};
use std::io::{Read, Seek, SeekFrom};

pub struct DepthRegionDecoder<S: AsRef<str>> {
    source: Base64Source<S>,
    original: u32,
    region: OriginalRegion,
    output: Vec<f32>,
    row: Vec<u8>,
    next: u32,
    validated: bool,
    ready: bool,
    failure: Option<String>,
}
impl<S: AsRef<str>> DepthRegionDecoder<S> {
    pub fn new(encoded: S, original: u32, region: OriginalRegion) -> Result<Self, String> {
        let pixels = region.validate(original)?;
        let source = Base64Source::new(encoded)?;
        let expected = u64::from(original)
            .checked_mul(u64::from(original))
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or("Original depth dimensions overflow")?;
        if source.len() != expected {
            return Err("Original depth dimensions do not match encoded bytes".into());
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(pixels)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            source,
            original,
            region,
            output,
            row: vec![0; region.size[0] as usize * 4],
            next: 0,
            validated: false,
            ready: false,
            failure: None,
        })
    }
    /// One unit validates <=3072 source bytes or copies one original row
    /// segment. NaN/infinity/payload bits are preserved without numeric conversion.
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
            if !self.validated {
                let mut scratch = [0u8; 3072];
                if self.source.read(&mut scratch).map_err(|e| e.to_string())? != 0 {
                    continue;
                }
                self.validated = true;
                continue;
            }
            if self.next == self.region.size[1] {
                self.ready = true;
                continue;
            }
            let y = self.region.origin[1] + self.next;
            let offset =
                (u64::from(y) * u64::from(self.original) + u64::from(self.region.origin[0])) * 4;
            self.source
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            self.source
                .read_exact(&mut self.row)
                .map_err(|e| e.to_string())?;
            self.output.extend(
                self.row
                    .chunks_exact(4)
                    .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap())),
            );
            self.next += 1;
        }
        Ok(self.ready)
    }
    pub fn decoded_rows(&self) -> u32 {
        self.next
    }
    pub fn finish(self) -> Result<Vec<f32>, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.ready {
            return Err("Original depth rectangle is incomplete".into());
        }
        Ok(self.output)
    }
}
