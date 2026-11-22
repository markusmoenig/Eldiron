//! Frozen HUD paint/input metadata, independent of a destination framebuffer.
use super::*;
use crate::editor_overlay::EditorOverlay;
use std::sync::Arc;

/// Native controllers supply their real document/tool identity and revision.
/// The immediate CPU reference adapter uses its map identity for compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HudScope {
    pub(crate) document: Uuid,
    pub(crate) tool: Uuid,
    pub(crate) revision: u64,
}
#[derive(Default, PartialEq)]
pub(super) struct HitLayout {
    pub(super) header: bool,
    pub(super) icons: Vec<TheDim>,
    pub(super) subdivisions: Vec<TheDim>,
    pub(super) edit_modes: Vec<TheDim>,
    pub(super) selected: i32,
    pub(super) selected_server: Option<i32>,
}
struct IconSelection {
    requested: i32,
    displayed: i32,
}
impl PartialEq for IconSelection {
    fn eq(&self, other: &Self) -> bool {
        self.displayed == other.displayed
    }
}
#[derive(PartialEq)]
pub(super) struct Source {
    scope: HudScope,
    size: (usize, usize),
    map: Uuid,
    changed: u32,
    camera: MapCamera,
    subdivisions: u32,
    gizmo: GeometryGizmoOp,
    icon: IconSelection,
    context: ProjectContext,
    view: EditorViewMode,
    action: Option<Uuid>,
    palette: bool,
    tool: MapToolType,
    vertices: Vec<u32>,
    linedefs: Vec<u32>,
    sectors: Vec<u32>,
    objects: Vec<Uuid>,
    props: Vec<Uuid>,
    geometry_vertices: Vec<(Uuid, usize)>,
    faces: Vec<(Uuid, usize)>,
    points: Vec<(Uuid, usize, usize)>,
    segments: Vec<(Uuid, usize, usize)>,
    entity_item: Option<Uuid>,
    layout: HitLayout,
}
impl Source {
    pub(super) fn new(
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        server: &CreatorContext,
        layout: HitLayout,
    ) -> Self {
        Self {
            scope,
            size,
            map: map.id,
            changed: map.changed,
            camera: map.camera,
            subdivisions: map.subdivisions.to_bits(),
            gizmo: server.shared.geometry_gizmo_op,
            icon: IconSelection {
                requested: server.shared.selected_hud_icon_index,
                displayed: layout
                    .selected_server
                    .unwrap_or(server.shared.selected_hud_icon_index),
            },
            context: server.shared.pc,
            view: server.shared.editor_view_mode,
            action: server.shared.curr_action_id,
            palette: server.shared.palette_tool_active,
            tool: server.shared.curr_map_tool_type,
            vertices: map.selected_vertices.clone(),
            linedefs: map.selected_linedefs.clone(),
            sectors: map.selected_sectors.clone(),
            objects: map.selected_geometry_objects.clone(),
            props: map.selected_block_prop_instances.clone(),
            geometry_vertices: map.selected_geometry_vertices.clone(),
            faces: map.selected_geometry_faces.clone(),
            points: map.selected_geometry_surface_points.clone(),
            segments: map.selected_geometry_surface_segments.clone(),
            entity_item: map.selected_entity_item,
            layout,
        }
    }
    fn matches(&self, map: &Map, server: &CreatorContext) -> bool {
        self.map == map.id
            && self.camera == map.camera
            && self.subdivisions == map.subdivisions.to_bits()
            && self.gizmo == server.shared.geometry_gizmo_op
            && (server.shared.selected_hud_icon_index == self.icon.requested
                || server.shared.selected_hud_icon_index == self.icon.displayed)
            && self.changed == map.changed
            && self.context == server.shared.pc
            && self.view == server.shared.editor_view_mode
            && self.action == server.shared.curr_action_id
            && self.palette == server.shared.palette_tool_active
            && self.tool == server.shared.curr_map_tool_type
            && self.vertices == map.selected_vertices
            && self.linedefs == map.selected_linedefs
            && self.sectors == map.selected_sectors
            && self.objects == map.selected_geometry_objects
            && self.props == map.selected_block_prop_instances
            && self.geometry_vertices == map.selected_geometry_vertices
            && self.faces == map.selected_geometry_faces
            && self.points == map.selected_geometry_surface_points
            && self.segments == map.selected_geometry_surface_segments
            && self.entity_item == map.selected_entity_item
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HudToken {
    owner: Uuid,
    serial: u64,
    #[cfg(feature = "native-ui")]
    receipt_owner: u64,
}
impl HudToken {
    #[cfg(feature = "native-ui")]
    pub(crate) fn receipt(self) -> theframework::paint::FrameReceipt {
        theframework::paint::FrameReceipt {
            owner: self.receipt_owner,
            serial: self.serial,
        }
    }
}
pub(crate) struct HudFrame {
    pub(super) commands: EditorOverlay,
    source: Arc<Source>,
    pub(crate) token: HudToken,
}
impl HudFrame {
    pub(crate) fn covers_control(&self, at: Vec2<i32>) -> bool {
        let layout = &self.source.layout;
        layout.icons.iter().chain(&layout.subdivisions).chain(&layout.edit_modes)
            .any(|rect| rect.contains(at))
            || (layout.header && self.source.camera == MapCamera::TwoD && at.y < 20)
    }

    #[cfg(test)]
    pub(crate) fn commands(&self) -> &EditorOverlay {
        &self.commands
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn crate::editor_overlay::OverlayImages,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        let mut list = self.commands.paint(text, images, density)?;
        list.ops
            .push(theframework::paint::PaintOp::Receipt(self.token.receipt()));
        Ok(list)
    }
}
impl Hud {
    pub(super) fn stage(&mut self, commands: EditorOverlay, source: Source) -> HudFrame {
        let source = Arc::new(source);
        if self
            .prepared_source
            .as_ref()
            .is_some_and(|previous| previous.as_ref() != source.as_ref())
        {
            self.invalidate_frame();
        }
        self.serial = self
            .serial
            .checked_add(1)
            .expect("HUD frame serial exhausted");
        self.prepared_source = Some(source.clone());
        HudFrame {
            commands,
            source,
            token: HudToken {
                owner: self.frame_owner,
                serial: self.serial,
                #[cfg(feature = "native-ui")]
                receipt_owner: self.receipt_owner,
            },
        }
    }
    /// Revoke old/pending native input on hide, resize, tool/document change,
    /// suspend or GPU reconstruction. Retained editable source images survive.
    pub(crate) fn invalidate_frame(&mut self) {
        self.invalidated_after = self.serial;
        self.prepared_source = None;
        self.displayed_source = None;
        self.icon_rects.clear();
        self.subdiv_rects.clear();
        self.edit_mode_rects.clear();
    }
    #[cfg(feature = "native-ui")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn presented_paint(
        &mut self,
        frame: &HudFrame,
        painted: &[theframework::paint::FrameReceipt],
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        server: &mut CreatorContext,
    ) -> bool {
        if frame.token.receipt_owner != self.receipt_owner
            || !painted.contains(&frame.token.receipt())
        {
            return false;
        }
        self.presented_frame(frame, scope, size, map, server)
    }
    /// Acknowledge only a receipt present in the successfully presented root list.
    /// Preparing a newer frame never makes its hit rectangles active by itself.
    pub(super) fn presented_frame(
        &mut self,
        frame: &HudFrame,
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        server: &mut CreatorContext,
    ) -> bool {
        if frame.commands.is_empty()
            || frame.token.owner != self.frame_owner
            || frame.token.serial <= self.invalidated_after
            || frame.token.serial <= self.last_presented
            || frame.token.serial > self.serial
            || frame.source.scope != scope
            || frame.source.size != size
            || !frame.source.matches(map, server)
            || self
                .prepared_source
                .as_ref()
                .is_none_or(|current| current.as_ref() != frame.source.as_ref())
        {
            return false;
        }
        self.icon_rects = frame.source.layout.icons.clone();
        self.subdiv_rects = frame.source.layout.subdivisions.clone();
        self.edit_mode_rects = frame.source.layout.edit_modes.clone();
        self.selected_icon_index = frame.source.layout.selected;
        if let Some(selected) = frame.source.layout.selected_server {
            server.shared.selected_hud_icon_index = selected;
        }
        self.last_presented = frame.token.serial;
        self.displayed_source = Some(frame.source.clone());
        true
    }
    pub(crate) fn owns_frame(&self, frame: &HudFrame) -> bool {
        frame.token.owner == self.frame_owner && frame.token.serial <= self.serial
    }
    pub(crate) fn command_source_current(
        &self,
        frame: &HudFrame,
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        server: &CreatorContext,
    ) -> bool {
        self.owns_frame(frame)
            && frame.token.serial > self.invalidated_after
            && frame.source.scope == scope
            && frame.source.size == size
            && frame.source.matches(map, server)
            && self
                .prepared_source
                .as_ref()
                .is_some_and(|source| source.as_ref() == frame.source.as_ref())
    }
    pub(crate) fn is_presented_frame(&self, frame: &HudFrame) -> bool {
        frame.token.owner == self.frame_owner
            && frame.token.serial == self.last_presented
            && self
                .displayed_source
                .as_ref()
                .is_some_and(|source| Arc::ptr_eq(source, &frame.source))
    }
    pub(crate) fn frame_input_current(
        &self,
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        server: &CreatorContext,
    ) -> bool {
        self.displayed_source.as_ref().is_some_and(|source| {
            source.scope == scope && source.size == size && source.matches(map, server)
        })
    }
    /// Unavailable is distinct from a transparent/outside hit: a native owner
    /// must not fall through stale/unpainted HUD input into world authoring.
    pub(crate) fn hit_test(
        &self,
        scope: HudScope,
        size: (usize, usize),
        point: Vec2<i32>,
        map: &Map,
        server: &CreatorContext,
    ) -> HudHit {
        if !self.frame_input_current(scope, size, map, server) {
            return HudHit::Unavailable;
        }
        self.source_pointer_action(point, map)
            .map_or(HudHit::Outside, HudHit::Action)
    }
    pub(super) fn source_pointer_action(&self, point: Vec2<i32>, _map: &Map) -> Option<HudAction> {
        let source = self.displayed_source.as_ref()?;
        if point.x < 0
            || point.y < 0
            || point.x as usize >= source.size.0
            || point.y as usize >= source.size.1
        {
            return None;
        }
        for (index, rect) in self.edit_mode_rects.iter().enumerate() {
            if rect.contains(point) {
                return Some(HudAction::GeometryMode { resize: index == 1 });
            }
        }
        for (index, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(point) {
                return Some(HudAction::SelectIcon {
                    index: index as i32,
                    wall_row: self.mode == HudMode::Linedef,
                });
            }
        }
        if self.mode != HudMode::Rect {
            for (index, rect) in self.subdiv_rects.iter().enumerate() {
                if rect.contains(point) {
                    return Self::GRID_SUBDIVISIONS
                        .get(index)
                        .copied()
                        .map(HudAction::Subdivision);
                }
            }
        }
        (source.layout.header && source.camera == MapCamera::TwoD && point.y < 20)
            .then_some(HudAction::BlockHeader)
    }
    pub(super) fn source_input_current(&self, map: &Map, server: &CreatorContext) -> bool {
        self.displayed_source
            .as_ref()
            .is_some_and(|source| source.matches(map, server))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HudHit {
    Unavailable,
    Outside,
    Action(HudAction),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HudAction {
    GeometryMode { resize: bool },
    SelectIcon { index: i32, wall_row: bool },
    Subdivision(usize),
    BlockHeader,
}
#[derive(Default)]
pub(crate) struct HudEffects {
    pub(crate) scene_dirty: bool,
    pub(crate) overlay_dirty: bool,
    pub(crate) redraw: bool,
    pub(crate) events: Vec<TheEvent>,
}
impl HudEffects {
    /// Temporary reference controller adapter. Native owners consume the typed
    /// effects to invalidate their own scene/view state and forward UI events.
    pub(super) fn apply_reference(self, ctx: &mut TheContext, reference: &CreatorReference) {
        if self.scene_dirty || self.overlay_dirty {
            let mut renderer = reference.write();
            if self.scene_dirty {
                renderer.set_dirty();
            }
            if self.overlay_dirty {
                renderer.set_overlay_dirty();
            }
        }
        for event in self.events {
            ctx.ui.send(event);
        }
        ctx.ui.invalidate_paint_if(self.redraw);
    }
}
impl Hud {
    pub(crate) fn apply_action(
        &mut self,
        action: HudAction,
        map: &mut Map,
        server: &mut CreatorContext,
    ) -> HudEffects {
        let mut effects = HudEffects::default();
        match action {
            HudAction::GeometryMode { resize } => {
                server.shared.geometry_gizmo_op = if resize {
                    GeometryGizmoOp::Resize
                } else {
                    GeometryGizmoOp::Move
                };
                effects.overlay_dirty = true;
            }
            HudAction::SelectIcon { index, wall_row } => {
                self.selected_icon_index = index;
                server.shared.selected_hud_icon_index = index;
                effects.redraw = true;
                effects.events.push(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
                if wall_row {
                    server.shared.selected_wall_row = Some(index);
                    effects.events.push(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
            HudAction::Subdivision(value) => {
                map.subdivisions = value as f32;
                effects.scene_dirty = true;
                effects.overlay_dirty = true;
                effects.redraw = true;
                effects.events.push(TheEvent::Custom(
                    TheId::named("Tool Changed"),
                    TheValue::Empty,
                ));
            }
            HudAction::BlockHeader => {}
        }
        effects
    }
}
