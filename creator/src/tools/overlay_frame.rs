//! Ordered tool overlay descriptions. Preparation has CPU source data only;
//! the application retains the typed frame through successful root presentation.
use super::*;
mod input;
use crate::{
    editor_overlay::{
        EditorOverlay,
        panel::{PanelFrame, PanelScope},
    },
    hud::{Hud, HudFrame, HudInputs, HudScope, HudStyle},
};
pub use input::{
    ScopedToolOverlayEvent, ToolOverlayCommand, ToolOverlayEffects, ToolOverlayInputState,
    ToolOverlayRoute,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolOverlayTarget {
    pub document: Uuid,
    pub tool: Uuid,
    pub view: Uuid,
    /// Binding/source epoch, including parent-resolved action-slot and asset
    /// revisions. This is not a frame serial or a local draft edit counter.
    pub revision: u64,
    pub context: ProjectContext,
    pub size: (usize, usize),
    pub visible: bool,
}
impl ToolOverlayTarget {
    pub(crate) fn hud_scope(self) -> HudScope {
        HudScope {
            document: self.document,
            tool: self.tool,
            revision: self.revision,
        }
    }
    pub(crate) fn panel_scope(self) -> PanelScope {
        PanelScope {
            document: self.document,
            tool: self.tool,
            revision: self.revision,
        }
    }
    fn valid(self, server: &ServerContext) -> bool {
        self.visible
            && self.size.0 > 0
            && self.size.1 > 0
            && self.size.0 <= i32::MAX as usize
            && self.size.1 <= i32::MAX as usize
            && self.context == server.pc
    }
}
/// Resolved source inputs. Theme, action slots and source images are supplied by
/// the parent before preparation; no UI context, renderer or output is borrowed.
#[derive(Clone, Copy)]
pub struct ToolOverlayInputs<'a> {
    pub(crate) target: ToolOverlayTarget,
    pub(crate) hud: HudInputs<'a>,
    pub(crate) style: &'a HudStyle,
}
enum Part {
    Commands(EditorOverlay),
    Hud(HudFrame),
    Terrain(terrain_overlay::Frame),
    Props(PanelFrame<props_overlay::PropsSource, props_overlay::Action>),
}
/// Opaque typed aggregate; its source identities and frozen hit metadata remain
/// with the parent rather than being erased or reconstructed from current state.
pub struct ToolOverlayFrame {
    target: ToolOverlayTarget,
    selection: SelectionSource,
    parts: Vec<Part>,
}
// Detail profiles have independent selections/revisions. The common HUD tracks
// the base map; capture this additional source identity without cloning a map.
struct ProfileSelection {
    id: Uuid,
    changed: u32,
    vertices: Vec<u32>,
    linedefs: Vec<u32>,
    sectors: Vec<u32>,
}
struct SelectionSource {
    id: Option<u32>,
    mode: GeometryEditMode,
    detail: Option<(Uuid, u32, Option<Uuid>)>,
    profile: Option<ProfileSelection>,
    rectangle: Option<(Vec2<f32>, Vec2<f32>)>,
}
impl SelectionSource {
    fn new<T: Tool + ?Sized>(tool: &T, map: &Map, server: &ServerContext) -> Self {
        Self {
            id: tool.hud_selected_id(map, server),
            mode: server.geometry_edit_mode,
            detail: server
                .active_detail_surface
                .as_ref()
                .map(|s| (s.id, s.sector_id, s.profile)),
            profile: server
                .active_detail_surface
                .as_ref()
                .and_then(|s| s.profile)
                .and_then(|id| map.profiles.get(&id))
                .map(|p| ProfileSelection {
                    id: p.id,
                    changed: p.changed,
                    vertices: p.selected_vertices.clone(),
                    linedefs: p.selected_linedefs.clone(),
                    sectors: p.selected_sectors.clone(),
                }),
            rectangle: tool.overlay_screen_rectangle(map, server),
        }
    }
    fn matches<T: Tool + ?Sized>(&self, tool: &T, map: &Map, server: &ServerContext) -> bool {
        if self.id != tool.hud_selected_id(map, server)
            || self.mode != server.geometry_edit_mode
            || self.detail
                != server
                    .active_detail_surface
                    .as_ref()
                    .map(|s| (s.id, s.sector_id, s.profile))
            || self.rectangle != tool.overlay_screen_rectangle(map, server)
        {
            return false;
        }
        let current = server
            .active_detail_surface
            .as_ref()
            .and_then(|s| s.profile)
            .and_then(|id| map.profiles.get(&id));
        match (&self.profile, current) {
            (None, None) => true,
            (Some(old), Some(current)) => {
                old.id == current.id
                    && old.changed == current.changed
                    && old.vertices == current.selected_vertices
                    && old.linedefs == current.selected_linedefs
                    && old.sectors == current.selected_sectors
            }
            _ => false,
        }
    }
}
impl ToolOverlayFrame {
    pub(crate) fn prepare_standard<T: Tool + ?Sized>(
        tool: &mut T,
        input: ToolOverlayInputs<'_>,
    ) -> Option<Self> {
        if tool.id().uuid != input.target.tool || !input.target.valid(input.hud.server) {
            tool.invalidate_overlay();
            return None;
        }
        let selection = SelectionSource::new(&*tool, input.hud.map, input.hud.server);
        let selected_id = selection.id;
        let rectangle = selection.rectangle;
        let hud = tool.overlay_hud()?;
        let frame = hud.prepare_frame(
            input.target.hud_scope(),
            input.target.size,
            HudInputs {
                selected_id,
                ..input.hud
            },
            input.style,
        );
        let mut parts = Vec::new();
        if let Some(rectangle) = rectangle {
            parts.push(Part::Commands(super::prepare_screen_rectangle_preview(
                input.target.size,
                rectangle,
            )));
        }
        parts.push(Part::Hud(frame));
        Some(Self {
            target: input.target,
            selection,
            parts,
        })
    }
    pub(crate) fn passive<T: Tool + ?Sized>(
        tool: &T,
        input: ToolOverlayInputs<'_>,
        commands: EditorOverlay,
    ) -> Option<Self> {
        if tool.id().uuid != input.target.tool || !input.target.valid(input.hud.server) {
            return None;
        }
        Some(Self {
            target: input.target,
            selection: SelectionSource::new(tool, input.hud.map, input.hud.server),
            parts: vec![Part::Commands(commands)],
        })
    }
    pub(crate) fn push_terrain(&mut self, frame: Option<terrain_overlay::Frame>) {
        if let Some(frame) = frame {
            self.parts.push(Part::Terrain(frame));
        }
    }
    fn has_terrain(&self) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, Part::Terrain(_)))
    }
    fn has_hud(&self) -> bool {
        self.parts.iter().any(|part| matches!(part, Part::Hud(_)))
    }
    pub(crate) fn terrain_current(
        &self,
        panel: &terrain_overlay::TerrainPanel,
        source: &terrain_overlay::Source,
        target: ToolOverlayTarget,
        command: bool,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Terrain(frame) = part {
                    Some(if command {
                        panel.command_current(frame, target, source)
                    } else {
                        panel.current(frame, target, source)
                    })
                } else {
                    None
                }
            })
            .unwrap_or(!terrain_overlay::TerrainPanel::visible(target.size))
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented_terrain(
        &self,
        panel: &mut terrain_overlay::TerrainPanel,
        source: &terrain_overlay::Source,
        receipts: &[theframework::paint::FrameReceipt],
        target: ToolOverlayTarget,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Terrain(frame) = part {
                    Some(panel.presented(frame, receipts, target, source))
                } else {
                    None
                }
            })
            .unwrap_or(!terrain_overlay::TerrainPanel::visible(target.size))
    }
    pub(crate) fn push_props(
        &mut self,
        frame: PanelFrame<props_overlay::PropsSource, props_overlay::Action>,
    ) {
        if !frame.commands.is_empty() {
            self.parts.push(Part::Props(frame));
        }
    }
    pub(crate) fn commands(&self) -> impl Iterator<Item = &EditorOverlay> {
        self.parts.iter().map(|part| match part {
            Part::Commands(commands) => commands,
            Part::Hud(frame) => frame.commands(),
            Part::Terrain(frame) => &frame.commands,
            Part::Props(frame) => &frame.commands,
        })
    }
    pub(crate) fn draw_reference(&self, buffer: &mut TheRGBABuffer, ctx: &TheContext) {
        for commands in self.commands() {
            commands.draw_reference(buffer, ctx);
        }
    }
    fn matches(&self, target: ToolOverlayTarget, server: &ServerContext) -> bool {
        self.target == target && target.valid(server)
    }
    fn source_matches<T: Tool + ?Sized>(
        &self,
        tool: &T,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.matches(target, server)
            && tool.id().uuid == target.tool
            && self.selection.matches(tool, map, server)
    }
    pub(crate) fn hud_input_current(
        &self,
        hud: &Hud,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Hud(frame) = part {
                    Some(
                        hud.is_presented_frame(frame)
                            && hud.frame_input_current(
                                target.hud_scope(),
                                target.size,
                                map,
                                server,
                            ),
                    )
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
    pub(crate) fn props_input_current(
        &self,
        overlay: &props_overlay::PropsOverlay,
        target: ToolOverlayTarget,
        server: &ServerContext,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Props(frame) = part {
                    Some(overlay.frame_input_current(
                        frame,
                        target.panel_scope(),
                        target.size,
                        server,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(true)
    }
    pub(crate) fn command_source_current<T: Tool + ?Sized>(
        &self,
        tool: &mut T,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.source_matches(&*tool, target, map, server)
            && tool.overlay_command_source_current(self, target, map, server)
    }
    pub(crate) fn hud_command_source_current(
        &self,
        hud: &Hud,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Hud(frame) = part {
                    Some(hud.command_source_current(
                        frame,
                        target.hud_scope(),
                        target.size,
                        map,
                        server,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
    pub(crate) fn hud_owner_matches(&self, hud: &Hud) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Hud(frame) = part {
                    Some(hud.owns_frame(frame))
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
    pub(crate) fn props_command_source_current(
        &self,
        overlay: &props_overlay::PropsOverlay,
        target: ToolOverlayTarget,
        server: &ServerContext,
    ) -> bool {
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Props(frame) = part {
                    Some(overlay.command_source_current(
                        frame,
                        target.panel_scope(),
                        target.size,
                        server,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(true)
    }
    pub(crate) fn input_current<T: Tool + ?Sized>(
        &self,
        tool: &mut T,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> bool {
        self.source_matches(&*tool, target, map, server)
            && tool.overlay_input_current(self, target, map, server)
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn crate::editor_overlay::OverlayImages,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        let mut list = theframework::paint::PaintList::default();
        for part in &self.parts {
            let next = match part {
                Part::Commands(commands) => commands.paint(text, images, density)?,
                Part::Hud(frame) => frame.paint(text, images, density)?,
                Part::Terrain(frame) => frame.paint(text, images, density)?,
                Part::Props(frame) => frame.paint(text, images, density)?,
            };
            list.ops.extend(next.ops);
        }
        Ok(list)
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented_hud<T: Tool + ?Sized>(
        &self,
        tool: &mut T,
        receipts: &[theframework::paint::FrameReceipt],
        target: ToolOverlayTarget,
        map: &Map,
        server: &mut ServerContext,
    ) -> bool {
        if !self.source_matches(&*tool, target, map, server) {
            return false;
        }
        let Some(hud) = tool.overlay_hud() else {
            return false;
        };
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Hud(frame) = part {
                    Some(hud.presented_paint(
                        frame,
                        receipts,
                        target.hud_scope(),
                        target.size,
                        map,
                        server,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented_props(
        &self,
        overlay: &mut props_overlay::PropsOverlay,
        receipts: &[theframework::paint::FrameReceipt],
        target: ToolOverlayTarget,
        server: &ServerContext,
    ) -> bool {
        if !self.matches(target, server) {
            return false;
        }
        self.parts
            .iter()
            .find_map(|part| {
                if let Part::Props(frame) = part {
                    Some(overlay.presented_paint(
                        frame,
                        receipts,
                        target.panel_scope(),
                        target.size,
                        server,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(true)
    }
}
#[cfg(test)]
mod tests;
