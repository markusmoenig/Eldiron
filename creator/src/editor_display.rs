//! Shared full-viewport editing displays used by both actions and persistent tools.
//!
//! The implementation currently lives beside the original profile action code while the host is
//! being generalized. Consumers should import this neutral module so additional displays (walls,
//! paths, curves) do not depend on action ownership.

pub use crate::actions::action_edit::{
    EditorDisplay, EditorDisplayOwner, EditorDisplayResult, EditorDisplaySession, EditorProfile2D,
    EditorProfileDimensions, EditorProfilePreset,
};
