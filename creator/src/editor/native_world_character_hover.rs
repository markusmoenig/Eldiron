use super::*;
use eldiron_presentation::ViewPick;
use theframework::text::{TextLayout, TextSystem};

#[derive(Default)]
pub(super) struct CharacterHover {
    pub(super) point: Option<Vec2<i32>>,
    layout: Option<(String, (u64, u64), f32, TextLayout)>,
}

pub(super) fn name_at<'a>(
    binding: &Binding,
    map: &'a Map,
    scenes: &SceneViews,
    output: ViewImage,
    isometric: Option<&eldiron_presentation::isometric::IsometricWorldTargets>,
    point: Vec2<i32>,
) -> Option<&'a str> {
    let (width, height) = (binding.size.0 as f32, binding.size.1 as f32);
    let point = point.map(|axis| axis as f32);
    let uv = [point.x / width, point.y / height];
    if uv.iter().any(|axis| !(0. ..1.).contains(axis)) {
        return None;
    }
    let id = if binding.mode != EditorViewMode::D2 {
        match scenes.pick_game(output, uv) {
            ViewPick::Hit(hit) => match hit.geo {
                scenevm::GeoId::Character(id) => id,
                _ => return None,
            },
            _ => return None,
        }
    } else if binding.camera == MapCamera::TwoDIso {
        isometric?.editor_character(uv)?
    } else {
        let origin = Vec2::new(width, height) * 0.5 + Vec2::new(map.offset.x, -map.offset.y);
        let position = (point - origin) / map.grid_size;
        rusterix::world_interaction::CellTargets::capture_visible(map)
            .hit(position, false)?
            .entity?
    };
    map.entities
        .iter()
        .find(|entity| entity.id == id)?
        .attributes
        .get_str("name")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

impl CharacterHover {
    pub(super) fn paint(
        &mut self,
        name: &str,
        viewport: (usize, usize),
        text: &mut TextSystem,
        list: &mut PaintList,
    ) {
        let Some(point) = self.point else { return };
        let (width, height) = (viewport.0 as f32, viewport.1 as f32);
        if width < 20. || height < 28. {
            return;
        }
        let font = text.font_stamp();
        let wrap = (width - 20.).min(260.);
        if self
            .layout
            .as_ref()
            .is_none_or(|(old, stamp, limit, _)| old != name || *stamp != font || *limit != wrap)
        {
            self.layout = Some((
                name.into(),
                font,
                wrap,
                text.layout(name, "Roboto", 13., Some(wrap)),
            ));
        }
        let (_, _, _, layout) = self.layout.as_ref().unwrap();
        let w = layout.width().ceil() + 14.;
        let h = (layout.height().ceil() + 8.).min(height - 6.);
        let x = (point.x as f32 + 13.).min(width - w - 3.).max(3.);
        let y = (point.y as f32 + 17.).min(height - h - 3.).max(3.);
        let rect = Rect::new(x, y, w, h);
        list.rect(rect, 4., [0.085, 0.075, 0.063, 0.97]);
        list.ops.push(PaintOp::PushClip(rect));
        for run in &layout.runs {
            list.ops.push(PaintOp::GlyphRun {
                run: run.clone(),
                origin: [x + 7., y + 4.],
                color: [0.94, 0.88, 0.76, 1.],
            });
        }
        list.ops.push(PaintOp::PopClip);
    }
}
