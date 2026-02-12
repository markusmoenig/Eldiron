use crate::Texture;
use theframework::prelude::*;

/// Screen-relative direction for avatar perspectives.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AvatarDirection {
    Front,
    Back,
    Left,
    Right,
}

/// Frames for a single perspective direction.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvatarAnimationFrame {
    pub texture: Texture,
    #[serde(default)]
    pub weapon_main_anchor: Option<(i16, i16)>,
    #[serde(default)]
    pub weapon_off_anchor: Option<(i16, i16)>,
}

impl AvatarAnimationFrame {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            weapon_main_anchor: None,
            weapon_off_anchor: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AvatarFrameSerde {
    Texture(Texture),
    Frame(AvatarAnimationFrame),
}

fn deserialize_avatar_frames<'de, D>(deserializer: D) -> Result<Vec<AvatarAnimationFrame>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let frames = Vec::<AvatarFrameSerde>::deserialize(deserializer)?;
    Ok(frames
        .into_iter()
        .map(|f| match f {
            AvatarFrameSerde::Texture(texture) => AvatarAnimationFrame::new(texture),
            AvatarFrameSerde::Frame(frame) => frame,
        })
        .collect())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvatarPerspective {
    pub direction: AvatarDirection,
    #[serde(default, deserialize_with = "deserialize_avatar_frames")]
    pub frames: Vec<AvatarAnimationFrame>,
    #[serde(default)]
    pub weapon_main_anchor: Option<(i16, i16)>,
    #[serde(default)]
    pub weapon_off_anchor: Option<(i16, i16)>,
}

impl Default for AvatarPerspective {
    fn default() -> Self {
        Self {
            direction: AvatarDirection::Front,
            frames: vec![],
            weapon_main_anchor: None,
            weapon_off_anchor: None,
        }
    }
}

/// A named animation with perspectives, each holding its own frames.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvatarAnimation {
    pub id: Uuid,
    pub name: String,
    /// Playback time scale: 1.0 = normal, >1.0 = slower, <1.0 = faster.
    #[serde(default = "AvatarAnimation::default_speed")]
    pub speed: f32,
    pub perspectives: Vec<AvatarPerspective>,
}

impl Default for AvatarAnimation {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Unnamed".to_string(),
            speed: 1.0,
            perspectives: vec![],
        }
    }
}

impl AvatarAnimation {
    fn default_speed() -> f32 {
        1.0
    }
}

/// Number of perspective directions supported by an avatar.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum AvatarPerspectiveCount {
    /// Single direction (Front only)
    One,
    /// Four directions (Front, Back, Left, Right)
    Four,
}

impl Default for AvatarPerspectiveCount {
    fn default() -> Self {
        Self::One
    }
}

/// The data for a character instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Avatar {
    pub id: Uuid,
    pub name: String,
    pub resolution: u16,
    pub perspective_count: AvatarPerspectiveCount,
    pub animations: Vec<AvatarAnimation>,
}

impl Default for Avatar {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Unnamed".to_string(),
            resolution: 24,
            perspective_count: AvatarPerspectiveCount::One,
            animations: vec![],
        }
    }
}

impl Avatar {
    /// Sets the resolution and resizes all existing frame textures to the new size.
    pub fn set_resolution(&mut self, new_resolution: u16) {
        if new_resolution == self.resolution || new_resolution == 0 {
            return;
        }
        let size = new_resolution as usize;
        for animation in &mut self.animations {
            for perspective in &mut animation.perspectives {
                for frame in &mut perspective.frames {
                    frame.texture = frame.texture.resized(size, size);
                }
            }
        }
        self.resolution = new_resolution;
    }

    /// Sets the perspective count, adding or removing perspective entries in each animation.
    pub fn set_perspective_count(&mut self, count: AvatarPerspectiveCount) {
        if count == self.perspective_count {
            return;
        }

        let size = self.resolution as usize;

        let needed: &[AvatarDirection] = match count {
            AvatarPerspectiveCount::One => &[AvatarDirection::Front],
            AvatarPerspectiveCount::Four => &[
                AvatarDirection::Front,
                AvatarDirection::Back,
                AvatarDirection::Left,
                AvatarDirection::Right,
            ],
        };

        for anim in &mut self.animations {
            // Determine frame count from existing perspectives (use first, fallback 1)
            let frame_count = anim
                .perspectives
                .first()
                .map(|p| p.frames.len())
                .unwrap_or(1)
                .max(1);

            // Add missing perspectives with matching frame count
            for dir in needed {
                if !anim.perspectives.iter().any(|p| p.direction == *dir) {
                    let frames = (0..frame_count)
                        .map(|_| {
                            AvatarAnimationFrame::new(Texture::new(
                                vec![0; size * size * 4],
                                size,
                                size,
                            ))
                        })
                        .collect();
                    anim.perspectives.push(AvatarPerspective {
                        direction: *dir,
                        frames,
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    });
                }
            }

            // Remove perspectives not in the needed set
            anim.perspectives.retain(|p| needed.contains(&p.direction));
        }

        self.perspective_count = count;
    }

    /// Sets the frame count for the given animation, allocating or truncating textures.
    /// Frame count is clamped to a minimum of 1.
    pub fn set_animation_frame_count(&mut self, animation_id: &Uuid, count: usize) {
        let count = count.max(1);
        let size = self.resolution as usize;

        if let Some(anim) = self.animations.iter_mut().find(|a| a.id == *animation_id) {
            // Ensure perspectives exist based on perspective_count
            let needed: &[AvatarDirection] = match self.perspective_count {
                AvatarPerspectiveCount::One => &[AvatarDirection::Front],
                AvatarPerspectiveCount::Four => &[
                    AvatarDirection::Front,
                    AvatarDirection::Back,
                    AvatarDirection::Left,
                    AvatarDirection::Right,
                ],
            };

            // Add missing perspectives
            for dir in needed {
                if !anim.perspectives.iter().any(|p| p.direction == *dir) {
                    anim.perspectives.push(AvatarPerspective {
                        direction: *dir,
                        frames: vec![],
                        weapon_main_anchor: None,
                        weapon_off_anchor: None,
                    });
                }
            }

            // Resize frames in each perspective
            for perspective in &mut anim.perspectives {
                let current = perspective.frames.len();
                if count > current {
                    for _ in current..count {
                        perspective
                            .frames
                            .push(AvatarAnimationFrame::new(Texture::new(
                                vec![0; size * size * 4],
                                size,
                                size,
                            )));
                    }
                } else if count < current {
                    perspective.frames.truncate(count);
                }
            }
        }
    }

    /// Returns the frame count for the given animation (from the first perspective, or 0).
    pub fn get_animation_frame_count(&self, animation_id: &Uuid) -> usize {
        self.animations
            .iter()
            .find(|a| a.id == *animation_id)
            .and_then(|a| a.perspectives.first())
            .map(|p| p.frames.len())
            .unwrap_or(0)
    }
}

/// Marker recolor configuration used by avatar builds.
#[derive(Clone, Copy, Debug)]
pub struct AvatarMarkerColors {
    pub skin_light: [u8; 4],
    pub skin_dark: [u8; 4],
    pub torso: [u8; 4],
    pub legs: [u8; 4],
    pub hair: [u8; 4],
    pub eyes: [u8; 4],
    pub hands: [u8; 4],
    pub feet: [u8; 4],
}

impl Default for AvatarMarkerColors {
    fn default() -> Self {
        Self {
            skin_light: [255, 224, 189, 255],
            skin_dark: [205, 133, 63, 255],
            torso: [70, 90, 140, 255],
            legs: [50, 60, 90, 255],
            hair: [70, 50, 30, 255],
            eyes: [30, 80, 120, 255],
            hands: [255, 210, 170, 255],
            feet: [80, 70, 60, 255],
        }
    }
}

/// Output image data for an avatar frame.
#[derive(Clone, Debug)]
pub struct AvatarBuildOutput {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// Request for building a single avatar frame.
pub struct AvatarBuildRequest<'a> {
    pub avatar: &'a Avatar,
    pub animation_name: Option<&'a str>,
    pub direction: AvatarDirection,
    pub frame_index: usize,
    pub marker_colors: AvatarMarkerColors,
}

/// Stub avatar builder.
/// This currently recolors marker pixels on selected frame data.
pub struct AvatarBuilder;

impl AvatarBuilder {
    pub fn build_current_stub(req: AvatarBuildRequest<'_>) -> Option<AvatarBuildOutput> {
        let anim = req
            .animation_name
            .and_then(|name| {
                req.avatar
                    .animations
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(name))
            })
            .or_else(|| req.avatar.animations.first())?;

        let persp = anim
            .perspectives
            .iter()
            .find(|p| p.direction == req.direction)
            .or_else(|| {
                anim.perspectives
                    .iter()
                    .find(|p| p.direction == AvatarDirection::Front)
            })
            .or_else(|| anim.perspectives.first())?;

        if persp.frames.is_empty() {
            return None;
        }

        let frame = persp.frames.get(req.frame_index % persp.frames.len())?;

        // SceneVM avatar data is square; normalize here for the stub path.
        let target_size = frame.texture.width.max(frame.texture.height);
        let mut rgba = if frame.texture.width == frame.texture.height {
            frame.texture.data.clone()
        } else {
            frame.texture.resized(target_size, target_size).data
        };

        Self::recolor_markers(&mut rgba, req.marker_colors);

        Some(AvatarBuildOutput {
            size: target_size as u32,
            rgba,
        })
    }

    fn recolor_markers(rgba: &mut [u8], colors: AvatarMarkerColors) {
        const SKIN_LIGHT: [u8; 3] = [255, 0, 255];
        const SKIN_DARK: [u8; 3] = [200, 0, 200];
        const TORSO: [u8; 3] = [0, 0, 255];
        const LEGS: [u8; 3] = [0, 255, 0];
        const HAIR: [u8; 3] = [255, 255, 0];
        const EYES: [u8; 3] = [0, 255, 255];
        const HANDS: [u8; 3] = [255, 128, 0];
        const FEET: [u8; 3] = [255, 80, 0];

        for px in rgba.chunks_exact_mut(4) {
            if px[3] == 0 {
                continue;
            }
            let src = [px[0], px[1], px[2]];
            let dst = if src == SKIN_LIGHT {
                colors.skin_light
            } else if src == SKIN_DARK {
                colors.skin_dark
            } else if src == TORSO {
                colors.torso
            } else if src == LEGS {
                colors.legs
            } else if src == HAIR {
                colors.hair
            } else if src == EYES {
                colors.eyes
            } else if src == HANDS {
                colors.hands
            } else if src == FEET {
                colors.feet
            } else {
                continue;
            };
            px.copy_from_slice(&dst);
        }
    }
}
