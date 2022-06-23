
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "embedded/"]
#[include = "assets/*"]
#[include = "game/*"]
#[exclude = ".DS_Store"]
pub struct Embedded;