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

    /// Sets the perspective count. Will eventually add/remove perspectives in animations.
    pub fn set_perspective_count(&mut self, count: AvatarPerspectiveCount) {
        if count == self.perspective_count {
            return;
        }
        // TODO: add/remove perspective entries in each animation
        self.perspective_count = count;
    }
}
