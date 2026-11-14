//! Seekable decoded-byte addressing directly over the original Base64 string.
//! The PNG reader can seek without allocating a decoded PNG payload.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::io::{self, Read, Seek, SeekFrom};

pub(super) struct Base64Source<S> {
    source: S,
    len: u64,
    at: u64,
}
impl<S: AsRef<str>> Base64Source<S> {
    pub(super) fn new(source: S) -> Result<Self, String> {
        let text = source.as_ref();
        if text.len() % 4 != 0 {
            return Err("Invalid original Base64 length".into());
        }
        let padding = if text.ends_with("==") {
            2
        } else if text.ends_with('=') {
            1
        } else {
            0
        };
        let len = (text.len() / 4 * 3)
            .checked_sub(padding)
            .ok_or("Invalid original Base64 padding")? as u64;
        Ok(Self { source, len, at: 0 })
    }
    pub(super) fn len(&self) -> u64 {
        self.len
    }
    #[cfg(test)]
    pub(super) fn validate(&mut self) -> Result<(), String> {
        let previous = self.at;
        self.at = 0;
        let mut scratch = [0u8; 3072];
        loop {
            if self.read(&mut scratch).map_err(|e| e.to_string())? == 0 {
                break;
            }
        }
        self.at = previous;
        Ok(())
    }
}
impl<S: AsRef<str>> Read for Base64Source<S> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.at >= self.len || out.is_empty() {
            return Ok(0);
        }
        let text = self.source.as_ref().as_bytes();
        let mut written = 0;
        while written < out.len() && self.at < self.len {
            let start = (self.at / 3 * 4) as usize;
            let end = (start + 4096).min(text.len());
            let skip = (self.at % 3) as usize;
            let mut decoded = [0u8; 3072];
            let len = STANDARD
                .decode_slice(&text[start..end], &mut decoded)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            if len <= skip {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid original Base64 window",
                ));
            }
            let count = (len - skip).min(out.len() - written);
            out[written..written + count].copy_from_slice(&decoded[skip..skip + count]);
            written += count;
            self.at += count as u64;
        }
        Ok(written)
    }
}
impl<S: AsRef<str>> Seek for Base64Source<S> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let next = match from {
            SeekFrom::Start(at) => at as i128,
            SeekFrom::Current(delta) => self.at as i128 + delta as i128,
            SeekFrom::End(delta) => self.len as i128 + delta as i128,
        };
        if !(0..=u64::MAX as i128).contains(&next) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Original Base64 seek is out of range",
            ));
        }
        self.at = next as u64;
        Ok(self.at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeks_and_small_unaligned_reads_match_original_binary_without_decode_copy() {
        for length in [0, 1, 2, 3, 4095, 4096, 4097, 15031] {
            let bytes: Vec<u8> = (0..length).map(|i| (i * 37) as u8).collect();
            let encoded = STANDARD.encode(&bytes);
            let mut source = Base64Source::new(encoded).unwrap();
            source.validate().unwrap();
            assert_eq!(source.len(), length as u64);
            for at in (0..length).step_by(7) {
                source.seek(SeekFrom::Start(at as u64)).unwrap();
                let mut actual = [0; 13];
                let got = source.read(&mut actual).unwrap();
                assert_eq!(&actual[..got], &bytes[at..(at + 13).min(length)]);
            }
            assert!(source.seek(SeekFrom::Start(0)).is_ok());
            assert!(source.seek(SeekFrom::Current(-1)).is_err());
        }
    }
}
