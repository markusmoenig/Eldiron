//! Compatibility imports for action consumers. Display data, controller and
//! rendering now belong to the neutral editor_display module.
pub use crate::editor_display::{
    EditorDisplay, EditorDisplayOwner, EditorDisplayResult, EditorDisplaySession, EditorProfile2D,
    EditorProfileDimensions, EditorProfilePreset,
};
