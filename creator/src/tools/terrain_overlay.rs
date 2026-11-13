//! Terrain controls retain their original layout and controller parameters.
//! The viewport holds each typed frame through root presentation acknowledgement.
use super::*;
use crate::editor_overlay::{
    EditorOverlay,
    panel::{PanelFrame, PanelFrames},
};
use rusterix::chunkbuilder::terrain_sculpt::SculptMode;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Source {
    pub mode: SculptMode,
    pub radius: u32,
    pub strength: u32,
    pub view: EditorViewMode,
}
#[derive(Clone, Copy)]
pub enum Action {
    Mode(SculptMode),
    Radius(f32),
    Strength(f32),
    ClearHover,
    Key(char),
}
pub(crate) fn shortcut(key: char) -> Option<Action> {
    matches!(key, '1' | '2' | '3' | '[' | ']').then_some(Action::Key(key))
}
pub(crate) type Frame = PanelFrame<Source, Action>;
#[derive(Default)]
pub(crate) struct TerrainPanel {
    frames: PanelFrames<Source, Action>,
    pub native: bool,
}
impl TerrainPanel {
    pub fn visible(size: (usize, usize)) -> bool {
        size.0 >= 250 && size.1 >= 150
    }
    pub fn invalidate(&mut self) {
        self.frames.invalidate();
    }
    pub fn prepare(&mut self, target: ToolOverlayTarget, source: Source) -> Option<Frame> {
        if !Self::visible(target.size) {
            self.invalidate();
            return None;
        }
        let mut commands = EditorOverlay::new(target.size);
        commands.signed_rect(TheDim::rect(8, 28, 232, 104), [24, 29, 28, 245]);
        let mut label = |rect: TheDim, text: &str, selected| {
            commands.signed_rect(
                rect,
                if selected {
                    [82, 89, 71, 255]
                } else {
                    [46, 52, 49, 255]
                },
            );
            commands.text_rect_blend(
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                text,
                TheFontSettings {
                    size: 12.,
                    ..Default::default()
                },
                &[235, 235, 223, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        };
        let modes = [
            ("Raise", SculptMode::Raise),
            ("Lower", SculptMode::Lower),
            ("Smooth", SculptMode::Smooth),
        ];
        for (i, (name, mode)) in modes.into_iter().enumerate() {
            label(
                TheDim::rect(14 + i as i32 * 74, 36, 70, 26),
                name,
                source.mode == mode,
            );
        }
        for (y, text) in [
            (68, format!("Radius: {:.1}", f32::from_bits(source.radius))),
            (
                100,
                format!("Strength: {:.2}", f32::from_bits(source.strength)),
            ),
        ] {
            label(TheDim::rect(14, y, 144, 26), &text, false);
            label(TheDim::rect(164, y, 30, 26), "−", false);
            label(TheDim::rect(200, y, 30, 26), "+", false);
        }
        // Retain the reference's clickable row gaps, independently of the labels.
        let buttons = vec![
            (TheDim::rect(8, 36, 80, 26), Action::Mode(SculptMode::Raise)),
            (
                TheDim::rect(88, 36, 74, 26),
                Action::Mode(SculptMode::Lower),
            ),
            (
                TheDim::rect(162, 36, 78, 26),
                Action::Mode(SculptMode::Smooth),
            ),
            (TheDim::rect(164, 68, 36, 26), Action::Radius(-0.5)),
            (TheDim::rect(200, 68, 40, 26), Action::Radius(0.5)),
            (TheDim::rect(164, 100, 36, 26), Action::Strength(-0.25)),
            (TheDim::rect(200, 100, 40, 26), Action::Strength(0.25)),
        ];
        Some(self.frames.stage_commands(
            target.panel_scope(),
            source,
            commands,
            TheDim::rect(8, 28, 232, 104),
            buttons,
        ))
    }
    pub fn current(&self, frame: &Frame, target: ToolOverlayTarget, source: &Source) -> bool {
        self.frames.is_presented_frame(frame)
            && self
                .frames
                .input_current(target.panel_scope(), target.size, source)
    }
    pub fn command_current(
        &self,
        frame: &Frame,
        target: ToolOverlayTarget,
        source: &Source,
    ) -> bool {
        self.frames
            .command_source_current(frame, target.panel_scope(), target.size, source)
    }
    #[cfg(feature = "native-ui")]
    pub fn presented(
        &mut self,
        frame: &Frame,
        receipts: &[theframework::paint::FrameReceipt],
        target: ToolOverlayTarget,
        source: &Source,
    ) -> bool {
        self.frames
            .presented_paint(frame, receipts, target.panel_scope(), target.size, source)
    }
}
