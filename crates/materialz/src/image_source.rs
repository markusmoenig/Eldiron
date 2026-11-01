//! Portable image assets. Bytes are document data, never generated shader source.
use crate::{Error, Result};
use std::fmt::Write;
/// A plane contains one static frame or the same number of frames as Color.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlane {
    pub channel: String,
    pub space: String,
    pub frames: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ImageSource {
    pub width: usize,
    pub height: usize,
    /// Optional host asset identity. Embedded pixels remain usable without the host.
    pub reference: Option<String>,
    pub planes: Vec<ImagePlane>,
}
impl ImageSource {
    pub fn rgba8(width: usize, height: usize, frames: &[Vec<u8>]) -> Result<Self> {
        let mut image = Self {
            width,
            height,
            reference: None,
            planes: vec![],
        };
        image.set_plane("color", "srgb", frames)?;
        image.validate()?;
        Ok(image)
    }
    pub fn set_plane(&mut self, channel: &str, space: &str, frames: &[Vec<u8>]) -> Result<()> {
        let frames = frames
            .iter()
            .map(|bytes| {
                let mut hex = String::with_capacity(bytes.len() * 2);
                for b in bytes {
                    write!(hex, "{b:02x}").unwrap();
                }
                hex
            })
            .collect();
        let plane = ImagePlane {
            channel: channel.into(),
            space: space.into(),
            frames,
        };
        let old = self.clone();
        self.planes.retain(|p| p.channel != channel);
        self.planes.push(plane);
        if let Err(e) = self.validate() {
            *self = old;
            return Err(e);
        }
        Ok(())
    }
    pub fn frame_count(&self) -> usize {
        self.planes
            .iter()
            .map(|p| p.frames.len())
            .max()
            .unwrap_or(1)
    }
    pub fn validate(&self) -> Result<()> {
        let count = self
            .width
            .checked_mul(self.height)
            .filter(|n| *n > 0 && *n <= 1_048_576)
            .ok_or_else(|| Error("Image dimensions must contain 1–1048576 pixels".into()))?;
        let frames = self.frame_count();
        if frames > 32 || count.checked_mul(frames).is_none_or(|n| n > 4_194_304) {
            return Err(Error(
                "Image source exceeds 32 frames or 4194304 total pixels".into(),
            ));
        }
        if !self.planes.iter().any(|p| p.channel == "color") {
            return Err(Error("Image source needs a Color plane".into()));
        }
        let mut channels = std::collections::HashSet::new();
        for p in &self.planes {
            if !matches!(
                p.channel.as_str(),
                "color" | "roughness" | "metallic" | "opacity" | "emission"
            ) || !channels.insert(&p.channel)
                || !matches!(p.space.as_str(), "linear" | "srgb")
                || !(p.frames.len() == 1 || p.frames.len() == frames)
            {
                return Err(Error(
                    "Invalid image channel, color space or frame count".into(),
                ));
            }
            for hex in &p.frames {
                if hex.len() != count * 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(Error("Invalid image RGBA8 frame payload".into()));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn json(&self) -> String {
        use crate::json::string as j;
        let planes = self
            .planes
            .iter()
            .map(|p| {
                format!(
                    "{{\"channel\":{},\"space\":{},\"frames\":[{}]}}",
                    j(&p.channel),
                    j(&p.space),
                    p.frames.iter().map(|v| j(v)).collect::<Vec<_>>().join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"width\":{},\"height\":{},\"reference\":{},\"planes\":[{planes}]}}",
            self.width,
            self.height,
            self.reference
                .as_deref()
                .map(j)
                .unwrap_or_else(|| "null".into())
        )
    }
}
