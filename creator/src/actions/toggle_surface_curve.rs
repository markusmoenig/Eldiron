use crate::actions::geometry_face_ops::{
    selected_surface_curve_segment_ids, set_selected_surface_detail_curve,
};
use crate::editor::RUSTERIX;
use crate::prelude::*;

const SURFACE_CURVE_MODE_ID: &str = "actionSurfaceCurveMode";
const SURFACE_CURVE_AMOUNT_ID: &str = "actionSurfaceCurveAmount";

pub struct ToggleSurfaceCurve {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleSurfaceCurve {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_toggle_surface_curve_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            SURFACE_CURVE_MODE_ID.into(),
            "Mode".into(),
            "".into(),
            vec!["Line".into(), "Arc".into()],
            1,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SURFACE_CURVE_AMOUNT_ID.into(),
            "Amount".into(),
            "".into(),
            0.35,
            -2.0..=2.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_toggle_surface_curve")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_toggle_surface_curve_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(
            TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
            'c',
        ))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && !selected_surface_curve_segment_ids(map).is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let segment_ids = selected_surface_curve_segment_ids(map);
        if segment_ids.is_empty() {
            return;
        }

        let mut amount_sum = 0.0;
        let mut amount_count = 0;
        for (object_id, face_index, segment_index) in segment_ids {
            let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == object_id)
            else {
                continue;
            };
            let Some(face) = object.faces.get(face_index) else {
                continue;
            };
            let Some(segment) = face.surface_segments.get(segment_index) else {
                continue;
            };
            if segment.mode == rusterix::GeometrySurfaceSegmentMode::Arc {
                amount_sum += segment.curve_amount;
                amount_count += 1;
            }
        }

        self.nodeui.set_i32_value(SURFACE_CURVE_MODE_ID, 1);
        if amount_count > 0 {
            self.nodeui
                .set_f32_value(SURFACE_CURVE_AMOUNT_ID, amount_sum / amount_count as f32);
        } else {
            self.nodeui.set_f32_value(SURFACE_CURVE_AMOUNT_ID, 0.35);
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mode = if self
            .nodeui
            .get_i32_value(SURFACE_CURVE_MODE_ID)
            .unwrap_or(1)
            == 0
        {
            rusterix::GeometrySurfaceSegmentMode::Line
        } else {
            rusterix::GeometrySurfaceSegmentMode::Arc
        };
        let curve_amount = self
            .nodeui
            .get_f32_value(SURFACE_CURVE_AMOUNT_ID)
            .unwrap_or(0.35)
            .clamp(-2.0, 2.0);
        let prev = map.clone();
        if !set_selected_surface_detail_curve(map, mode, curve_amount) {
            return None;
        }

        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
