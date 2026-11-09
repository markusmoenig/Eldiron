//! Ordinary pre-rendered iso sprites. Importing them never creates an MZ recipe.
use crate::project::{Project, TileCollectionAsset};
use rusterix::isometric::SpriteLayout;
use rusterix::{Texture, Tile, TileRole};
use std::collections::BTreeMap;
use theframework::prelude::Uuid;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    #[default]
    Auto,
    Ground,
    Object,
    Character,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    Size,
    Rate,
    Empty,
    TooMany,
    Image(String),
    Duplicate(String),
    Views(String),
    Canvas(String),
    Frame {
        actor: String,
        clip: String,
        frame: usize,
        direction: usize,
    },
    Direction {
        actor: String,
        clip: String,
    },
}
impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Size => write!(
                f,
                "Size must be an even ground-cell width between 2 and 4096 pixels."
            ),
            Self::Rate => write!(
                f,
                "Animation speed must be between 0 and 120 frames per second."
            ),
            Self::Empty => write!(f, "Select at least one PNG image."),
            Self::TooMany => write!(f, "Use at most 2048 images at a time."),
            Self::Image(name) => write!(
                f,
                "{name}: image must be between 1 and 4096 pixels per side."
            ),
            Self::Duplicate(name) => write!(f, "Duplicate sprite view or frame: {name}"),
            Self::Views(name) => write!(
                f,
                "{name}: supply all N/E/S/W views, or disable direction grouping."
            ),
            Self::Canvas(name) => write!(
                f,
                "{name}: all views and frames must share the same canvas size."
            ),
            Self::Frame {
                actor,
                clip,
                frame,
                direction,
            } => write!(
                f,
                "{actor}: missing {clip} frame {frame} in direction {direction}"
            ),
            Self::Direction { actor, clip } => {
                write!(f, "{actor}: {clip} must include direction 0")
            }
        }
    }
}
impl std::error::Error for ImportError {}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub width: u32,
    pub kind: Kind,
    pub anchor: Option<[i32; 2]>,
    pub prelit: bool,
    pub fps: f32,
    pub collection: String,
    pub group_directions: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            width: 256,
            kind: Kind::Auto,
            anchor: None,
            prelit: true,
            fps: 12.,
            collection: "Imported iso sprites".into(),
            group_directions: true,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), ImportError> {
        if self.width < 2 || self.width > 4096 || self.width % 2 != 0 {
            return Err(ImportError::Size);
        }
        if !self.fps.is_finite() || self.fps <= 0. || self.fps > 120. {
            return Err(ImportError::Rate);
        }
        Ok(())
    }
    pub fn layout(&self, texture: &Texture, ground: bool) -> SpriteLayout {
        SpriteLayout {
            bitmap: true,
            prelit: self.prelit,
            anchor: self.anchor.unwrap_or([
                texture.width as i32 / 2,
                texture.height as i32 - self.width as i32 / 4,
            ]),
            pixels_per_unit: self.width as f32 / 2.,
            ground,
            directions: 1,
            frames_per_direction: 1,
            frames_per_second: self.fps,
            ..Default::default()
        }
    }
}
fn ground(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "ground"
            | "floor"
            | "dirt"
            | "dirttiles"
            | "stone"
            | "stonetile"
            | "stoneuneven"
            | "stonemissingtiles"
            | "planks"
            | "planksbroken"
            | "plankshole"
    )
}
fn blocking(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let opening = name.contains("archway")
        || ((name.contains("door") || name.contains("gate"))
            && !name.contains("closed")
            && !name.contains("bars"));
    (name.contains("wall") && !opening)
        || name.contains("crate")
        || name.contains("barrel")
        || name.contains("table")
        || name.contains("chest")
        || name.contains("column")
        || name == "fence"
}

fn character_name(name: &str) -> Option<(String, usize, String, usize)> {
    let mut parts = name.rsplitn(3, '_');
    let animation = parts.next()?;
    let direction = parts.next()?.parse::<usize>().ok()?;
    let actor = parts.next()?;
    if direction > 7 {
        return None;
    }
    let at = animation.find(|c: char| c.is_ascii_digit())?;
    Some((
        actor.to_string(),
        direction,
        animation[..at].to_ascii_lowercase(),
        animation[at..].parse().ok()?,
    ))
}
/// Names are optional conventions, never interpreted as code. Source pixels and
/// padding remain intact; a collection groups views of the same asset together.
pub fn import(
    project: &mut Project,
    images: Vec<(String, Texture)>,
    settings: &Settings,
) -> Result<Vec<Uuid>, ImportError> {
    settings.validate()?;
    if images.is_empty() {
        return Err(ImportError::Empty);
    }
    if images.len() > 2048 {
        return Err(ImportError::TooMany);
    }
    let mut props = BTreeMap::<String, BTreeMap<usize, Texture>>::new();
    let mut characters = BTreeMap::<String, BTreeMap<(usize, String, usize), Texture>>::new();
    for (name, texture) in images {
        if texture.width == 0
            || texture.height == 0
            || texture.width > 4096
            || texture.height > 4096
        {
            return Err(ImportError::Image(name));
        }
        if settings.group_directions
            && let Some((actor, direction, clip, frame)) = character_name(&name)
        {
            if characters
                .entry(actor)
                .or_default()
                .insert((direction, clip, frame), texture)
                .is_some()
            {
                return Err(ImportError::Duplicate(name));
            }
            continue;
        }
        let (base, direction) = settings
            .group_directions
            .then(|| name.rsplit_once('_'))
            .flatten()
            .and_then(|(base, s)| {
                Some((
                    base,
                    match s {
                        "E" => 0,
                        "N" => 1,
                        "W" => 2,
                        "S" => 3,
                        _ => return None,
                    },
                ))
            })
            .unwrap_or((&name, 0));
        if props
            .entry(base.into())
            .or_default()
            .insert(direction, texture)
            .is_some()
        {
            return Err(ImportError::Duplicate(name));
        }
    }
    let mut imported = Vec::new();
    for (name, mut views) in props {
        let directions = if views.len() == 4 && (0..4).all(|i| views.contains_key(&i)) {
            4
        } else {
            1
        };
        if directions == 1 && views.len() > 1 {
            return Err(ImportError::Views(name));
        }
        let textures = views
            .iter_mut()
            .map(|(_, t)| std::mem::take(t))
            .collect::<Vec<_>>();
        validate_canvas(&name, &textures)?;
        let is_ground = match settings.kind {
            Kind::Auto => ground(&name),
            Kind::Ground => true,
            _ => false,
        };
        let mut layout = settings.layout(&textures[0], is_ground);
        layout.directions = directions;
        let mut tile = Tile::from_textures(textures);
        tile.alias = name.clone();
        tile.blocking = !is_ground && blocking(&name);
        tile.role = if is_ground {
            TileRole::Dungeon
        } else {
            TileRole::ManMade
        };
        tile.isometric = Some(layout);
        imported.push(tile);
    }
    for (actor, frames) in characters {
        let order = [2, 1, 0, 7, 6, 5, 4, 3];
        let count = frames
            .keys()
            .filter(|(d, c, _)| *d == 0 && c == "run")
            .count();
        let mut textures = Vec::new();
        for direction in order {
            for frame in 0..count {
                textures.push(
                    frames
                        .get(&(direction, "run".into(), frame))
                        .ok_or_else(|| ImportError::Frame {
                            actor: actor.clone(),
                            clip: "run".into(),
                            frame,
                            direction,
                        })?
                        .clone(),
                );
            }
            textures.push(
                frames
                    .get(&(direction, "idle".into(), 0))
                    .ok_or_else(|| ImportError::Frame {
                        actor: actor.clone(),
                        clip: "idle".into(),
                        frame: 0,
                        direction,
                    })?
                    .clone(),
            );
        }
        validate_canvas(&actor, &textures)?;
        let mut layout = settings.layout(&textures[0], false);
        layout.directions = 8;
        layout.walk_frames = count;
        layout.frames_per_direction = count + 1;
        let mut tile = Tile::from_textures(textures);
        tile.alias = actor.clone();
        tile.role = TileRole::Character;
        tile.blocking = false;
        tile.isometric = Some(layout);
        imported.push(tile);
        // Keep additional clips as their own editable, direction-aware assets.
        let clips = frames
            .keys()
            .filter(|(_, c, _)| c != "run" && c != "idle")
            .map(|(_, c, _)| c.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for clip in clips {
            let count = frames
                .keys()
                .filter(|(d, c, _)| *d == 0 && *c == clip)
                .count();
            let mut textures = Vec::new();
            for direction in order {
                for frame in 0..count {
                    textures.push(
                        frames
                            .get(&(direction, clip.clone(), frame))
                            .ok_or_else(|| ImportError::Frame {
                                actor: actor.clone(),
                                clip: clip.clone(),
                                frame,
                                direction,
                            })?
                            .clone(),
                    );
                }
            }
            if textures.is_empty() {
                return Err(ImportError::Direction { actor, clip });
            }
            validate_canvas(&actor, &textures)?;
            let mut layout = settings.layout(&textures[0], false);
            layout.directions = 8;
            layout.frames_per_direction = count;
            let mut tile = Tile::from_textures(textures);
            tile.alias = format!("{actor} / {clip}");
            tile.role = TileRole::Character;
            tile.isometric = Some(layout);
            imported.push(tile);
        }
    }
    let mut collection = TileCollectionAsset::new(settings.collection.clone());
    collection.description="Original bitmap iso artwork, with editable ground-cell size, pivot and directions. No mesh recipe is required.".into();
    let id = collection.id;
    let ids = imported.iter().map(|t| t.id).collect::<Vec<_>>();
    for tile in imported {
        project.tiles.insert(tile.id, tile);
    }
    project.add_tile_collection(collection);
    for (i, tile) in ids.iter().enumerate() {
        let source = rusterix::TileSource::SingleTile(*tile);
        project.add_source_to_collection(&id, source);
        project.set_collection_tile_board_position(
            &id,
            source,
            theframework::prelude::Vec2::new((i % 12) as i32, (i / 12) as i32),
        );
    }
    Ok(ids)
}

fn validate_canvas(name: &str, textures: &[Texture]) -> Result<(), ImportError> {
    let first = &textures[0];
    if textures
        .iter()
        .any(|t| t.width != first.width || t.height != first.height)
    {
        return Err(ImportError::Canvas(name.into()));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image(value: u8) -> Texture {
        Texture::new(vec![value; 2 * 4 * 4], 2, 4)
    }
    #[test]
    fn cell_width_preserves_padding_pixels_and_direction_order() {
        let mut p = Project::default();
        let settings = Settings {
            width: 2,
            ..Default::default()
        };
        let ids = import(
            &mut p,
            [
                ("floor_S", 4),
                ("floor_W", 3),
                ("floor_N", 2),
                ("floor_E", 1),
            ]
            .into_iter()
            .map(|(n, v)| (n.into(), image(v)))
            .collect(),
            &settings,
        )
        .unwrap();
        let tile = &p.tiles[&ids[0]];
        let s = tile.isometric.as_ref().unwrap();
        assert_eq!(s.anchor, [1, 4]);
        assert_eq!(s.pixels_per_unit, 1.);
        assert_eq!(s.directions, 4);
        assert!(s.bitmap && s.prelit && s.ground);
        assert_eq!(
            tile.textures.iter().map(|t| t.data[0]).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(tile.textures[0].height, 4);
        assert!(tile.module.is_none());
    }
    #[test]
    fn invalid_views_do_not_partially_import() {
        let mut p = Project::default();
        let before = p.tiles.len();
        assert!(
            import(
                &mut p,
                vec![("wall_N".into(), image(1)), ("wall_E".into(), image(2))],
                &Settings::default()
            )
            .is_err()
        );
        assert_eq!(p.tiles.len(), before);
    }
    #[test]
    fn eight_directions_walk_idle_and_extra_clips_are_retained() {
        let mut p = Project::default();
        let mut images = Vec::new();
        for d in 0..8 {
            for clip in ["Idle0", "Run0", "Run1", "Pickup0"] {
                images.push((format!("Male_{d}_{clip}"), image(d)));
            }
        }
        let ids = import(&mut p, images, &Settings::default()).unwrap();
        assert_eq!(ids.len(), 2);
        let t = p.tiles.values().find(|t| t.alias == "Male").unwrap();
        let s = t.isometric.as_ref().unwrap();
        assert_eq!(s.walk_frames, 2);
        assert_eq!(s.frames_per_direction, 3);
        assert_eq!(
            t.textures
                .chunks(3)
                .map(|f| f[0].data[0])
                .collect::<Vec<_>>(),
            vec![2, 1, 0, 7, 6, 5, 4, 3]
        );
        assert!(
            p.tiles
                .values()
                .any(|t| t.alias == "Male / pickup" && t.textures.len() == 8)
        );
    }
}
