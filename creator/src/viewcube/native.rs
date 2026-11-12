//! Camera-scoped immutable cube geometry and input activated by root receipts.
use super::*;
use crate::editor_overlay::{
    EditorOverlay,
    panel::{PanelFrame, PanelFrames, PanelHit, PanelScope},
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) struct CubeTarget {
    pub(crate) view: Uuid,
    pub(crate) scope: PanelScope,
    pub(crate) size: (usize, usize),
    pub(crate) visible: bool,
}
#[derive(Clone, PartialEq)]
struct CameraPose {
    basis: [[f32; 3]; 3],
    view: Uuid,
}
impl CameraPose {
    fn of(camera: &D3OrbitCamera, view: Uuid) -> Option<Self> {
        let (forward, right, up) = camera.basis_vectors();
        let basis = [forward, right, up].map(|v| [v.x, v.y, v.z]);
        basis
            .iter()
            .flatten()
            .all(|v| v.is_finite())
            .then_some(Self { basis, view })
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum CubePick {
    Unavailable,
    Outside,
    Face(ViewCubeFace),
}
#[derive(Default)]
pub(crate) struct ViewCubeFrames {
    frames: PanelFrames<CameraPose, ()>,
    presented: Option<Arc<[ProjectedFace]>>,
}
pub(crate) struct CubeFrame {
    panel: PanelFrame<CameraPose, ()>,
    faces: Arc<[ProjectedFace]>,
}
impl CubeTarget {
    fn bounds(self) -> Option<TheDim> {
        if !self.visible || self.size.0 > i32::MAX as usize || self.size.1 > i32::MAX as usize {
            return None;
        }
        let center = ViewCube::center(TheDim::sized(self.size.0 as i32, self.size.1 as i32))?;
        Some(TheDim::rect(
            center.x as i32 - 41,
            center.y as i32 - 41,
            83,
            83,
        ))
    }
}
impl CubeFrame {
    pub(crate) fn commands(&self) -> &EditorOverlay {
        &self.panel.commands
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn receipt(&self) -> theframework::paint::FrameReceipt {
        self.panel.receipt()
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn crate::editor_overlay::OverlayImages,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        self.panel.paint(text, images, density)
    }
}
impl ViewCubeFrames {
    pub(crate) fn invalidate(&mut self) {
        self.frames.invalidate();
        self.presented = None;
    }
    pub(crate) fn prepare(
        &mut self,
        target: CubeTarget,
        camera: &D3OrbitCamera,
    ) -> Option<CubeFrame> {
        let Some((bounds, pose)) = target.bounds().zip(CameraPose::of(camera, target.view)) else {
            self.invalidate();
            return None;
        };
        let faces: Arc<[ProjectedFace]> = ViewCube::projected_faces(
            TheDim::sized(target.size.0 as i32, target.size.1 as i32),
            camera,
        )
        .into();
        let commands = ViewCube::commands_for_faces(target.size, &faces);
        let panel = self
            .frames
            .stage_commands(target.scope, pose, commands, bounds, vec![]);
        Some(CubeFrame { panel, faces })
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented(
        &mut self,
        frame: &CubeFrame,
        receipts: &[theframework::paint::FrameReceipt],
        target: CubeTarget,
        camera: &D3OrbitCamera,
    ) -> bool {
        let Some(pose) = target
            .bounds()
            .and_then(|_| CameraPose::of(camera, target.view))
        else {
            self.invalidate();
            return false;
        };
        if !self
            .frames
            .presented_paint(&frame.panel, receipts, target.scope, target.size, &pose)
        {
            return false;
        }
        self.presented = Some(frame.faces.clone());
        true
    }
    #[cfg(all(test, not(feature = "native-ui")))]
    fn presented_reference(
        &mut self,
        frame: &CubeFrame,
        target: CubeTarget,
        camera: &D3OrbitCamera,
    ) -> bool {
        let Some(pose) = target
            .bounds()
            .and_then(|_| CameraPose::of(camera, target.view))
        else {
            self.invalidate();
            return false;
        };
        if !self
            .frames
            .acknowledge_reference(&frame.panel, target.scope, target.size, &pose)
        {
            return false;
        }
        self.presented = Some(frame.faces.clone());
        true
    }
    /// New picking uses frozen painted polygons and their depth order. Outside
    /// the cube footprint, unavailable cube state never blocks world input.
    pub(crate) fn pick(
        &self,
        target: CubeTarget,
        camera: &D3OrbitCamera,
        coord: Vec2<i32>,
    ) -> CubePick {
        let Some(bounds) = target.bounds() else {
            return CubePick::Outside;
        };
        if !bounds.contains(coord) {
            return CubePick::Outside;
        }
        let Some(pose) = CameraPose::of(camera, target.view) else {
            return CubePick::Unavailable;
        };
        match self
            .frames
            .hit_test(target.scope, target.size, &pose, coord)
        {
            PanelHit::Unavailable => CubePick::Unavailable,
            PanelHit::Outside => CubePick::Outside,
            PanelHit::Inside(_) => {
                let Some(faces) = &self.presented else {
                    return CubePick::Unavailable;
                };
                faces
                    .iter()
                    .rev()
                    .find(|face| ViewCube::point_in_face(coord.map(|v| v as f32), face))
                    .map_or(CubePick::Outside, |face| CubePick::Face(face.face))
            }
        }
    }
}
#[cfg(test)]
mod tests;
