pub mod mapscript;

use crate::Texture;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub file_name: String,
    pub description: String,
    pub line: u32,
}

impl ParseError {
    pub fn new(file_name: String, description: String, line: u32) -> Self {
        Self {
            file_name,
            description,
            line,
        }
    }
}

/// Tries to load a texture from the current or the textures directory.
pub fn load_texture(texture: &str, path: String) -> Option<Texture> {
    let name = format!("{}.png", texture);

    if let Some(tex) = Texture::from_image_safe(std::path::Path::new(&name)) {
        return Some(tex);
    }

    let name = format!("textures/{}.png", texture);
    if let Some(tex) = Texture::from_image_safe(std::path::Path::new(&name)) {
        return Some(tex);
    }

    // Check in the provided path
    let name = format!("{}/{}.png", path, texture);
    if let Some(tex) = Texture::from_image_safe(std::path::Path::new(&name)) {
        return Some(tex);
    }

    // Check in the provided path / texture
    let name = format!("{}/textures/{}.png", path, texture);
    if let Some(tex) = Texture::from_image_safe(std::path::Path::new(&name)) {
        return Some(tex);
    }

    None
}
