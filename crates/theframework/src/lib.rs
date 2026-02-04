pub mod theapp;
pub mod thecolor;
pub mod thecontext;
pub mod thedim;
pub mod thedraw2d;
pub mod thenodecanvas;
pub mod thepalette;
pub mod thergbabuffer;
pub mod thetime;
pub mod thetrait;
#[cfg(feature = "winit_app")]
pub mod thewinitapp;

#[cfg(feature = "ui")]
pub mod theui;

#[cfg(feature = "log")]
pub mod thelogger;

#[cfg(feature = "i18n")]
pub mod thei18n;

pub use crate::theapp::TheApp;
pub use crate::thecontext::TheContext;
pub use crate::thetrait::TheTrait;

#[cfg(feature = "ui")]
pub use crate::theui::TheUI;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;
pub use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TheKeyCode {
    Escape,
    Return,
    Delete,
    Up,
    Right,
    Down,
    Left,
    Space,
    Tab,
}

use ::serde::de::{self, Deserializer};
use ::serde::ser::{self, Serializer};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

fn compress<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(ser::Error::custom)?;
    let compressed_data = encoder.finish().map_err(ser::Error::custom)?;

    serializer.serialize_bytes(&compressed_data)
}

fn decompress<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let data = Vec::<u8>::deserialize(deserializer)?;
    let mut decoder = ZlibDecoder::new(&data[..]);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .map_err(de::Error::custom)?;

    Ok(decompressed_data)
}

pub mod prelude {
    pub use serde::{Deserialize, Serialize};

    pub use crate::thedim::*;
    pub use crate::thergbabuffer::{
        TheRGBABuffer, TheRGBARegion, TheRGBARegionSequence, TheRGBATile,
    };

    pub use rustc_hash::*;
    pub use uuid::Uuid;
    pub use vek::*;

    pub use crate::theapp::TheApp;
    pub use crate::thecolor::TheColor;
    pub use crate::thecontext::TheContext;
    pub use crate::thecontext::TheCursorIcon;
    pub use crate::thedraw2d::{
        TheDraw2D, TheFontPreference, TheFontSettings, TheHorizontalAlign, TheVerticalAlign,
    };
    pub use crate::thenodecanvas::{TheNode, TheNodeCanvas, TheNodeTerminal};
    pub use crate::thepalette::ThePalette;
    pub use crate::thetime::TheTime;

    pub use crate::thetrait::TheTrait;
    pub use crate::TheKeyCode;

    //#[cfg(feature = "renderer")]
    //pub use therenderer::prelude::*;

    #[cfg(feature = "ui")]
    pub use crate::theui::prelude::*;

    //#[cfg(feature = "code")]
    //pub use crate::thecode::prelude::*;

    #[cfg(feature = "log")]
    pub use crate::thelogger::setup_logger;

    #[cfg(feature = "winit_app")]
    pub use crate::thewinitapp::run_winit_app;

    #[cfg(feature = "i18n")]
    pub use crate::thei18n::TheFontScript;
}
