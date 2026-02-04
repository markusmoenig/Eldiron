use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata for a recent project, used in the project browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    /// Display name
    pub name: String,

    /// Full file path
    pub path: PathBuf,

    /// Last opened timestamp (Unix timestamp)
    pub last_opened: u64,

    /// Optional thumbnail as base64-encoded PNG
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_base64: Option<String>,

    /// Thumbnail width (if thumbnail exists)
    #[serde(default)]
    pub thumbnail_width: u32,

    /// Thumbnail height (if thumbnail exists)
    #[serde(default)]
    pub thumbnail_height: u32,
}

impl RecentProject {
    pub fn new(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            name: name.into(),
            path,
            last_opened: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            thumbnail_base64: None,
            thumbnail_width: 0,
            thumbnail_height: 0,
        }
    }

    pub fn with_thumbnail(mut self, thumbnail_rgba: &[u8], width: u32, height: u32) -> Self {
        if let Ok(base64) = encode_thumbnail_to_base64(thumbnail_rgba, width, height) {
            self.thumbnail_base64 = Some(base64);
            self.thumbnail_width = width;
            self.thumbnail_height = height;
        }
        self
    }

    pub fn update_last_opened(&mut self) {
        self.last_opened = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Recent projects list, persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProjects {
    pub projects: Vec<RecentProject>,

    /// Maximum number of recent projects to keep
    #[serde(default = "default_max_recent")]
    pub max_recent: usize,
}

fn default_max_recent() -> usize {
    20
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            max_recent: default_max_recent(),
        }
    }
}

impl RecentProjects {
    /// Load recent projects from JSON file
    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save recent projects to JSON file
    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Add or update a project in the recent list
    pub fn add_or_update(&mut self, project: RecentProject) {
        // Remove if already exists
        self.projects.retain(|p| p.path != project.path);

        // Insert at beginning (most recent)
        self.projects.insert(0, project);

        // Trim to max_recent
        if self.projects.len() > self.max_recent {
            self.projects.truncate(self.max_recent);
        }
    }

    /// Remove a project from the recent list
    pub fn remove(&mut self, path: &std::path::Path) {
        self.projects.retain(|p| p.path != path);
    }

    /// Get projects sorted by last opened (most recent first)
    pub fn sorted_by_recent(&self) -> Vec<&RecentProject> {
        let mut projects: Vec<&RecentProject> = self.projects.iter().collect();
        projects.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        projects
    }
}

/// Encode RGBA image data to base64 PNG
fn encode_thumbnail_to_base64(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
    use image::{ImageFormat, RgbaImage};
    use std::io::Cursor;

    if rgba.len() != (width * height * 4) as usize {
        return Err("Invalid RGBA data size".to_string());
    }

    let img = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or("Failed to create image")?;

    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(base64_encode(buffer.get_ref()))
}

/// Decode base64 PNG to RGBA data
#[allow(dead_code)]
pub fn decode_thumbnail_from_base64(base64: &str) -> Result<(Vec<u8>, u32, u32), String> {
    use image::ImageFormat;
    use std::io::Cursor;

    let png_data = base64_decode(base64)?;
    let img = image::load(Cursor::new(png_data), ImageFormat::Png)
        .map_err(|e| format!("Failed to decode PNG: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok((rgba.into_raw(), width, height))
}

// Simple base64 encoding/decoding without extra dependencies
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();

    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        result.push(CHARS[(b1 >> 2) as usize]);
        result.push(CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize]);
        result.push(if chunk.len() > 1 {
            CHARS[(((b2 & 0x0f) << 2) | (b3 >> 6)) as usize]
        } else {
            b'='
        });
        result.push(if chunk.len() > 2 {
            CHARS[(b3 & 0x3f) as usize]
        } else {
            b'='
        });
    }

    String::from_utf8(result).unwrap()
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let bytes = s.as_bytes();

    for chunk in bytes.chunks(4) {
        if chunk.len() != 4 {
            break;
        }

        let mut vals = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            vals[i] = match b {
                b'A'..=b'Z' => b - b'A',
                b'a'..=b'z' => b - b'a' + 26,
                b'0'..=b'9' => b - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => 0,
                _ => return Err(format!("Invalid base64 character: {}", b as char)),
            };
        }

        result.push((vals[0] << 2) | (vals[1] >> 4));
        if chunk[2] != b'=' {
            result.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if chunk[3] != b'=' {
            result.push((vals[2] << 6) | vals[3]);
        }
    }

    Ok(result)
}
