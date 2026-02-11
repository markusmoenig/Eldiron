use rusterix::Texture;
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
pub struct AvatarPerspective {
    pub direction: AvatarDirection,
    pub frames: Vec<Texture>,
}

impl Default for AvatarPerspective {
    fn default() -> Self {
        Self {
            direction: AvatarDirection::Front,
            frames: vec![],
        }
    }
}

/// A named animation with perspectives, each holding its own frames.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvatarAnimation {
    pub id: Uuid,
    pub name: String,
    pub perspectives: Vec<AvatarPerspective>,
}

impl Default for AvatarAnimation {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Unnamed".to_string(),
            perspectives: vec![],
        }
    }
}

/// Number of perspective directions supported by an avatar.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
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
                    *frame = frame.resized(size, size);
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
                        .map(|_| Texture::new(vec![0; size * size * 4], size, size))
                        .collect();
                    anim.perspectives.push(AvatarPerspective {
                        direction: *dir,
                        frames,
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
                            .push(Texture::new(vec![0; size * size * 4], size, size));
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
