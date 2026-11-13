//! Source-scoped Rect composition/rotation controls, acknowledged by the root.
use super::*;
use crate::editor_overlay::{
    EditorOverlay,
    panel::{PanelFrame, PanelFrames},
};
#[derive(Clone, PartialEq)]
pub(crate) struct Source {
    pub map: Uuid,
    pub camera: MapCamera,
    pub compose: bool,
    pub yaw: u32,
    pub height: u32,
    pub step: u32,
    pub pick_height: bool,
    pub view: EditorViewMode,
    pub composition: Option<shared::material_composition::MapMaterialBinding>,
}
impl Source {
    pub fn read(map: &Map, server: &ServerContext) -> Self {
        Self {
            map: map.id,
            camera: map.camera,
            compose: server.rect_compose_mode,
            yaw: map
                .properties
                .get_float_default("iso_brush_yaw", 0.)
                .to_bits(),
            height: map
                .properties
                .get_float_default("iso_edit_height", 0.)
                .to_bits(),
            step: map
                .properties
                .get_float_default("iso_step_height", 0.25)
                .to_bits(),
            pick_height: map.properties.get_bool_default("iso_pick_height", false),
            view: server.editor_view_mode,
            composition: server.rect_composition_target.clone(),
        }
    }
}
#[derive(Clone, Copy)]
pub enum Action {
    Compose(bool),
    Rotate { delta: f32, button: bool },
    Height(f32),
}
pub(crate) fn shortcut(key: char, camera: MapCamera) -> Option<Action> {
    if camera != MapCamera::TwoDIso {
        return None;
    }
    match key {
        ',' => Some(Action::Rotate {
            delta: -90.,
            button: false,
        }),
        '.' => Some(Action::Rotate {
            delta: 90.,
            button: false,
        }),
        '[' => Some(Action::Height(-1.)),
        ']' => Some(Action::Height(1.)),
        _ => None,
    }
}
pub(crate) type Frame = PanelFrame<Source, Action>;
#[derive(Default)]
pub(crate) struct RectPanel {
    frames: PanelFrames<Source, Action>,
}
impl RectPanel {
    pub fn visible(target: ToolOverlayTarget) -> bool {
        target.size.0 >= 188 && target.size.1 >= 54 && target.context.is_region()
    }
    pub fn invalidate(&mut self) {
        self.frames.invalidate();
    }
    pub fn prepare(
        &mut self,
        target: ToolOverlayTarget,
        source: Source,
        highlight: EditorOverlay,
    ) -> Option<Frame> {
        if !Self::visible(target) {
            self.invalidate();
            return None;
        }
        let mut commands = EditorOverlay::new(target.size);
        let mut buttons = Vec::new();
        let mut label = |rect: TheDim, text: &str, selected: bool, background: bool| {
            if background {
                commands.signed_rect(
                    rect,
                    if selected {
                        [68, 68, 74, 245]
                    } else {
                        [34, 34, 36, 240]
                    },
                );
            }
            if selected {
                commands.signed_rect(
                    TheDim::rect(rect.x + 3, 46, rect.width - 6, 2),
                    [210, 176, 92, 255],
                );
            }
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
                &[230, 230, 232, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        };
        for (compose, x, width, text) in [(false, 8, 70, "Paint"), (true, 80, 100, "Compose")] {
            let rect = TheDim::rect(x, 26, width, 22);
            label(rect, text, source.compose == compose, true);
            if !source.pick_height || source.camera != MapCamera::TwoDIso {
                buttons.push((rect, Action::Compose(compose)));
            }
        }
        if source.camera == MapCamera::TwoDIso {
            let y = if target.size.0 >= 490 { 26 } else { 52 };
            let left = if y == 26 { 188 } else { 8 };
            for (i, (text, delta)) in [("Rotate -90 (,)", -90.), ("Rotate +90 (.)", 90.)]
                .into_iter()
                .enumerate()
            {
                let rect = TheDim::rect(left + i as i32 * 116, y, 112, 22);
                label(rect, text, false, true);
                buttons.push((
                    rect,
                    Action::Rotate {
                        delta,
                        button: true,
                    },
                ));
            }
            label(
                TheDim::rect(left + 232, y, 64, 22),
                &format!("{:.0} deg", f32::from_bits(source.yaw)),
                false,
                false,
            );
        }
        commands.append(highlight);
        Some(self.frames.stage_commands(
            target.panel_scope(),
            source,
            commands,
            TheDim::sized(target.size.0 as i32, target.size.1 as i32),
            buttons,
        ))
    }
    pub fn current(
        &self,
        frame: &Frame,
        target: ToolOverlayTarget,
        source: &Source,
        command: bool,
    ) -> bool {
        if command {
            self.frames
                .command_source_current(frame, target.panel_scope(), target.size, source)
        } else {
            self.frames.is_presented_frame(frame)
                && self
                    .frames
                    .input_current(target.panel_scope(), target.size, source)
        }
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
