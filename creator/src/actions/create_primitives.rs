use crate::actions::create_geometry_box::CreateGeometryBox;
use crate::editor::RUSTERIX;
use crate::prelude::*;

const WIDTH_ID: &str = "primitiveWidth";
const HEIGHT_ID: &str = "primitiveHeight";
const DEPTH_ID: &str = "primitiveDepth";
const RADIUS_ID: &str = "primitiveRadius";
const SEGMENTS_ID: &str = "primitiveSegments";
const SMOOTH_ID: &str = "primitiveSmooth";

pub struct CreateRoundedBox {
    id: TheId,
    nodeui: TheNodeUI,
}

pub struct CreateCylinder {
    id: TheId,
    nodeui: TheNodeUI,
}

fn dimension_slider(id: &str, label: &str, value: f32) -> TheNodeUIItem {
    TheNodeUIItem::FloatEditSlider(
        id.into(),
        label.into(),
        "".into(),
        value,
        0.05..=256.0,
        false,
    )
}

fn segments_slider(value: i32, max: i32) -> TheNodeUIItem {
    TheNodeUIItem::IntEditSlider(
        SEGMENTS_ID.into(),
        "Segments".into(),
        "Geometric resolution".into(),
        value,
        1..=max,
        false,
    )
}

fn finish_created_object(
    map: &mut Map,
    mut object: rusterix::GeometryObject,
    previous: Map,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
) -> Option<ProjectUndoAtom> {
    object.kind = rusterix::GeometryObjectKind::Generated;
    let object_id = object.id;
    map.geometry_objects.push(object);
    map.clear_selection();
    map.selected_geometry_objects.push(object_id);
    map.update_surfaces();
    server_ctx.curr_map_tool_type = MapToolType::Selection;
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Set Tool"),
        TheValue::Text("tool.geometry".into()),
    ));
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Map Selection Changed"),
        TheValue::Empty,
    ));
    RUSTERIX.write().unwrap().set_dirty();
    RUSTERIX.write().unwrap().set_overlay_dirty();
    Some(ProjectUndoAtom::MapEdit(
        server_ctx.pc,
        Box::new(previous),
        Box::new(map.clone()),
    ))
}

impl Action for CreateRoundedBox {
    fn new() -> Self {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            "Create a parametric box with geometric rounded corners.".into(),
        ));
        nodeui.add_item(dimension_slider(WIDTH_ID, "Width", 1.0));
        nodeui.add_item(dimension_slider(HEIGHT_ID, "Height", 1.0));
        nodeui.add_item(dimension_slider(DEPTH_ID, "Depth", 1.0));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            RADIUS_ID.into(),
            "Corner radius".into(),
            "Radius is clamped to half the smallest dimension.".into(),
            0.15,
            0.0..=128.0,
            false,
        ));
        nodeui.add_item(segments_slider(3, 8));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            SMOOTH_ID.into(),
            "Smooth corners".into(),
            "Share normals across the rounded surface.".into(),
            true,
        ));
        Self {
            id: TheId::named("Create Rounded Box"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Create an editable rounded box.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
    }

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let size = Vec3::new(
            self.nodeui.get_f32_value(WIDTH_ID).unwrap_or(1.0).max(0.05),
            self.nodeui
                .get_f32_value(HEIGHT_ID)
                .unwrap_or(1.0)
                .max(0.05),
            self.nodeui.get_f32_value(DEPTH_ID).unwrap_or(1.0).max(0.05),
        );
        let radius = self
            .nodeui
            .get_f32_value(RADIUS_ID)
            .unwrap_or(0.15)
            .max(0.0);
        let segments = self
            .nodeui
            .get_i32_value(SEGMENTS_ID)
            .unwrap_or(3)
            .clamp(1, 8) as usize;
        let smooth = self.nodeui.get_bool_value(SMOOTH_ID).unwrap_or(true);
        let (min, max) = CreateGeometryBox::bounds_for_primitive(map, ui, server_ctx, size)?;
        let previous = map.clone();
        let mut object = rusterix::GeometryObject::rounded_box_from_bounds(
            "Rounded Box",
            min,
            max,
            radius,
            segments,
            smooth,
        );
        object
            .properties
            .set("generator", Value::Str("rounded_box".to_string()));
        object
            .properties
            .set("primitive_radius", Value::Float(radius));
        object
            .properties
            .set("primitive_segments", Value::Int(segments as i32));
        object
            .properties
            .set("primitive_smooth", Value::Bool(smooth));
        finish_created_object(map, object, previous, ctx, server_ctx)
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

impl Action for CreateCylinder {
    fn new() -> Self {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            "Create a parametric cylinder. Resize it later to make an elliptical cylinder.".into(),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            RADIUS_ID.into(),
            "Radius".into(),
            "".into(),
            0.5,
            0.025..=128.0,
            false,
        ));
        nodeui.add_item(dimension_slider(HEIGHT_ID, "Height", 1.0));
        nodeui.add_item(segments_slider(16, 128));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            SMOOTH_ID.into(),
            "Smooth sides".into(),
            "Share normals around the cylinder while keeping caps flat.".into(),
            true,
        ));
        Self {
            id: TheId::named("Create Cylinder"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Create an editable cylinder.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
    }

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let radius = self
            .nodeui
            .get_f32_value(RADIUS_ID)
            .unwrap_or(0.5)
            .max(0.025);
        let height = self
            .nodeui
            .get_f32_value(HEIGHT_ID)
            .unwrap_or(1.0)
            .max(0.05);
        let segments = self
            .nodeui
            .get_i32_value(SEGMENTS_ID)
            .unwrap_or(16)
            .clamp(3, 128) as usize;
        let smooth = self.nodeui.get_bool_value(SMOOTH_ID).unwrap_or(true);
        let size = Vec3::new(radius * 2.0, height, radius * 2.0);
        let (min, max) = CreateGeometryBox::bounds_for_primitive(map, ui, server_ctx, size)?;
        let previous = map.clone();
        let mut object =
            rusterix::GeometryObject::cylinder_from_bounds("Cylinder", min, max, segments, smooth);
        object
            .properties
            .set("generator", Value::Str("cylinder".to_string()));
        object
            .properties
            .set("primitive_segments", Value::Int(segments as i32));
        object
            .properties
            .set("primitive_smooth", Value::Bool(smooth));
        finish_created_object(map, object, previous, ctx, server_ctx)
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
