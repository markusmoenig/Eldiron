
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "embedded/"]
#[include = "assets/*"]
#[exclude = ".DS_Store"]
pub struct Embedded;