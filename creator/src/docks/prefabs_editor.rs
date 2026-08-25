use crate::docks::iso_paint::IsoPaintDock;
use crate::docks::palette::PaletteDock;
use crate::docks::tiles::TilesDock;
use crate::editor::{RUSTERIX, SCENEMANAGER, TOOLLIST, UNDOMANAGER};
use crate::prelude::*;

const PREFAB_VIEW: &str = "PrefabView";
const MAP_VIEW: &str = "PolyView";
const MODE_STACK: &str = "Prefab Editor Mode Stack";
const PART_TREE: &str = "Prefab Editor Part Tree";
const PART_OBJECT_ITEM: &str = "Prefab Editor Geometry Object";
const SUPPORT_SURFACE_ITEM: &str = "Prefab Editor Support Surface";
const PREFAB_NAME: &str = "Prefab Editor Prefab Name";
const PART_NAME: &str = "Prefab Editor Part Name";
const PART_PARENT: &str = "Prefab Editor Part Parent";
const PART_ASSIGNMENT: &str = "Prefab Editor Object Assignment";
const PART_PIVOT: &str = "Prefab Editor Part Pivot";
const PART_DOOR_LAYOUT: &str = "Prefab Editor Door Layout";
const PART_DOOR_MOTION: &str = "Prefab Editor Door Motion";
const PART_DOOR_ANGLE: &str = "Prefab Editor Door Angle";
const PART_DOOR_SLIDE_DISTANCE: &str = "Prefab Editor Door Slide Distance";
const PART_DOOR_USAGE_DISTANCE: &str = "Prefab Editor Door Usage Distance";
const SUPPORT_SURFACE_NAME: &str = "Prefab Editor Support Surface Name";
const SUPPORT_SURFACE_SNAP: &str = "Prefab Editor Support Surface Snap";
const SUPPORT_SURFACE_TAGS: &str = "Prefab Editor Support Surface Tags";
const SUPPORT_SURFACE_CAPACITY: &str = "Prefab Editor Support Surface Capacity";
const SUPPORT_SURFACE_POLICY: &str = "Prefab Editor Support Surface Policy";
const PART_CREATE: &str = "Prefab Editor Create Part";
const PART_SET_PIVOT: &str = "Prefab Editor Set Pivot";
const PART_REMOVE: &str = "Prefab Editor Remove Part";
const PART_CONFIGURE_DOOR: &str = "Prefab Editor Configure Door";
const PART_PREVIEW_DOOR: &str = "Prefab Editor Preview Door";
const SUPPORT_SURFACE_CREATE: &str = "Prefab Editor Create Support Surface";
const SUPPORT_SURFACE_EDIT: &str = "Prefab Editor Edit Support Surface";
const SUPPORT_SURFACE_REMOVE: &str = "Prefab Editor Remove Support Surface";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PrefabEditorMode {
    #[default]
    Parts,
    Paint,
    Tiles,
    Palette,
}

impl PrefabEditorMode {
    fn index(self) -> i32 {
        match self {
            Self::Parts => 0,
            Self::Paint => 1,
            Self::Tiles => 2,
            Self::Palette => 3,
        }
    }
}

/// Full-screen editor shell for authored Prefabs.
///
/// Geometry tools still consume the established PolyView event contract. The
/// dedicated canvas translates its input at the dock boundary, keeping the
/// region canvas and its visual state completely separate. Its lower controls
/// are owned by the Prefab editor and therefore remain available in maximized mode.
pub struct PrefabsEditorDock {
    mode: PrefabEditorMode,
    selected_part_id: Option<Uuid>,
    selected_support_surface_id: Option<Uuid>,
    parent_options: Vec<Option<Uuid>>,
    assignment_options: Vec<Uuid>,
    door_preview_open: bool,
    paint_dock: IsoPaintDock,
    tiles_dock: TilesDock,
    palette_dock: PaletteDock,
}

impl PrefabsEditorDock {
    fn translated_view_event(event: &TheEvent) -> Option<TheEvent> {
        let map_id = || TheId::named(MAP_VIEW);
        match event {
            TheEvent::RenderViewClicked(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewClicked(map_id(), *coord))
            }
            TheEvent::RenderViewDragged(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewDragged(map_id(), *coord))
            }
            TheEvent::RenderViewHoverChanged(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewHoverChanged(map_id(), *coord))
            }
            TheEvent::RenderViewLostHover(id) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewLostHover(map_id()))
            }
            TheEvent::RenderViewScrollBy(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewScrollBy(map_id(), *coord))
            }
            TheEvent::RenderViewPreciseScrollBy(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewPreciseScrollBy(map_id(), *coord))
            }
            TheEvent::RenderViewZoomBy(id, delta) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewZoomBy(map_id(), *delta))
            }
            TheEvent::RenderViewUp(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewUp(map_id(), *coord))
            }
            TheEvent::RenderViewContext(id, coord) if id.name == PREFAB_VIEW => {
                Some(TheEvent::RenderViewContext(map_id(), *coord))
            }
            _ => None,
        }
    }

    fn part_actions_toolbar() -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut layout = TheHLayout::new(TheId::named("Prefab Part Actions"));
        layout.set_background_color(None);
        layout.set_margin(Vec4::new(6, 2, 6, 2));
        layout.set_padding(5);
        for (id, text, status) in [
            (
                PART_CREATE,
                fl!("prefab_editor_create_part"),
                fl!("status_prefab_editor_create_part"),
            ),
            (
                PART_REMOVE,
                fl!("prefab_editor_remove_part"),
                fl!("status_prefab_editor_remove_part"),
            ),
            (
                PART_SET_PIVOT,
                fl!("prefab_editor_set_pivot"),
                fl!("status_prefab_editor_set_pivot"),
            ),
            (
                SUPPORT_SURFACE_CREATE,
                fl!("prefab_editor_create_support_surface"),
                fl!("status_prefab_editor_create_support_surface"),
            ),
            (
                SUPPORT_SURFACE_REMOVE,
                fl!("prefab_editor_remove_support_surface"),
                fl!("status_prefab_editor_remove_support_surface"),
            ),
            (
                PART_CONFIGURE_DOOR,
                fl!("prefab_editor_configure_door"),
                fl!("status_prefab_editor_configure_door"),
            ),
            (
                PART_PREVIEW_DOOR,
                fl!("prefab_editor_preview_door"),
                fl!("status_prefab_editor_preview_door"),
            ),
        ] {
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(text);
            button.set_status_text(&status);
            button.set_fixed_size(false);
            layout.add_widget(Box::new(button));
        }
        layout.set_reverse_index(Some(2));
        canvas.set_layout(layout);
        canvas
    }

    fn parts_canvas() -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut tree_canvas = TheCanvas::new();
        tree_canvas.set_layout(TheTreeLayout::new(TheId::named(PART_TREE)));

        let mut inspector_canvas = TheCanvas::new();
        let mut inspector = TheTextLayout::new(TheId::named("Prefab Part Inspector"));
        inspector.set_margin(Vec4::new(10, 8, 10, 8));
        inspector.set_padding(7);
        inspector.set_text_margin(8);
        inspector.set_fixed_text_width(120);
        inspector.set_text_align(TheHorizontalAlign::Right);

        let mut prefab_name = TheTextLineEdit::new(TheId::named(PREFAB_NAME));
        prefab_name.limiter_mut().set_max_width(i32::MAX);
        prefab_name.set_status_text(&fl!("status_prefab_editor_prefab_name"));
        inspector.add_pair(fl!("prefab_editor_prefab_name"), Box::new(prefab_name));

        let mut name = TheTextLineEdit::new(TheId::named(PART_NAME));
        name.limiter_mut().set_max_width(i32::MAX);
        name.set_status_text(&fl!("status_prefab_editor_part_name"));
        inspector.add_pair(fl!("prefab_editor_part_name"), Box::new(name));

        let mut parent = TheDropdownMenu::new(TheId::named(PART_PARENT));
        parent.limiter_mut().set_max_width(i32::MAX);
        parent.set_status_text(&fl!("status_prefab_editor_part_parent"));
        inspector.add_pair(fl!("prefab_editor_part_parent"), Box::new(parent));

        let mut assignment = TheDropdownMenu::new(TheId::named(PART_ASSIGNMENT));
        assignment.limiter_mut().set_max_width(i32::MAX);
        assignment.set_status_text(&fl!("status_prefab_editor_part_assignment"));
        inspector.add_pair(fl!("prefab_editor_part_assignment"), Box::new(assignment));

        let mut pivot = TheTextLineEdit::new(TheId::named(PART_PIVOT));
        pivot.limiter_mut().set_max_width(i32::MAX);
        pivot.set_disabled(true);
        pivot.set_status_text(&fl!("status_prefab_editor_part_pivot"));
        inspector.add_pair(fl!("prefab_editor_part_pivot"), Box::new(pivot));

        let mut door_angle = TheTextLineEdit::new(TheId::named(PART_DOOR_ANGLE));
        door_angle.limiter_mut().set_max_width(i32::MAX);
        door_angle.set_value(TheValue::Text("90".to_string()));
        door_angle.set_status_text(&fl!("status_prefab_editor_door_angle"));
        inspector.add_pair(fl!("prefab_editor_door_angle"), Box::new(door_angle));

        let mut door_layout = TheDropdownMenu::new(TheId::named(PART_DOOR_LAYOUT));
        door_layout.add_option(fl!("prefab_editor_door_layout_single"));
        door_layout.add_option(fl!("prefab_editor_door_layout_split"));
        door_layout.limiter_mut().set_max_width(i32::MAX);
        door_layout.set_status_text(&fl!("status_prefab_editor_door_layout"));
        inspector.add_pair(fl!("prefab_editor_door_layout"), Box::new(door_layout));

        let mut door_motion = TheDropdownMenu::new(TheId::named(PART_DOOR_MOTION));
        door_motion.add_option(fl!("prefab_editor_door_motion_swing"));
        door_motion.add_option(fl!("prefab_editor_door_motion_slide"));
        door_motion.limiter_mut().set_max_width(i32::MAX);
        door_motion.set_status_text(&fl!("status_prefab_editor_door_motion"));
        inspector.add_pair(fl!("prefab_editor_door_motion"), Box::new(door_motion));

        let mut slide_distance = TheTextLineEdit::new(TheId::named(PART_DOOR_SLIDE_DISTANCE));
        slide_distance.limiter_mut().set_max_width(i32::MAX);
        slide_distance.set_value(TheValue::Text("1".to_string()));
        slide_distance.set_status_text(&fl!("status_prefab_editor_door_slide_distance"));
        inspector.add_pair(
            fl!("prefab_editor_door_slide_distance"),
            Box::new(slide_distance),
        );

        let mut usage_distance = TheTextLineEdit::new(TheId::named(PART_DOOR_USAGE_DISTANCE));
        usage_distance.limiter_mut().set_max_width(i32::MAX);
        usage_distance.set_value(TheValue::Text("3".to_string()));
        usage_distance.set_status_text(&fl!("status_prefab_editor_door_usage_distance"));
        inspector.add_pair(
            fl!("prefab_editor_door_usage_distance"),
            Box::new(usage_distance),
        );

        let mut surface_settings = TheTraybarButton::new(TheId::named(SUPPORT_SURFACE_EDIT));
        surface_settings.set_text(fl!("prefab_editor_edit_support_surface"));
        surface_settings.set_status_text(&fl!("status_prefab_editor_edit_support_surface"));
        surface_settings.set_fixed_size(false);
        surface_settings.limiter_mut().set_max_width(i32::MAX);
        inspector.add_pair(
            fl!("prefab_editor_support_surface"),
            Box::new(surface_settings),
        );

        inspector_canvas.set_layout(inspector);

        let mut split = TheSharedHLayout::new(TheId::named("Prefab Parts Shared HLayout"));
        split.set_shared_ratio(0.52);
        split.set_mode(TheSharedHLayoutMode::Shared);
        split.add_canvas(tree_canvas);
        split.add_canvas(inspector_canvas);
        let mut content = TheCanvas::new();
        content.set_layout(split);

        canvas.set_center(content);
        canvas.set_top(Self::part_actions_toolbar());
        canvas
    }

    fn support_surface_popup_canvas() -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(Vec2::new(420, 154));

        let mut inspector = TheTextLayout::new(TheId::named("Support Surface Popup Inspector"));
        inspector.set_background_color(None);
        inspector.set_margin(Vec4::new(10, 10, 10, 8));
        inspector.set_padding(6);
        inspector.set_text_margin(8);
        inspector.set_fixed_text_width(115);
        inspector.set_text_align(TheHorizontalAlign::Right);

        let mut surface_name = TheTextLineEdit::new(TheId::named(SUPPORT_SURFACE_NAME));
        surface_name.limiter_mut().set_max_width(i32::MAX);
        surface_name.set_status_text(&fl!("status_prefab_editor_surface_name"));
        inspector.add_pair(fl!("prefab_editor_surface_name"), Box::new(surface_name));

        let mut surface_snap = TheTextLineEdit::new(TheId::named(SUPPORT_SURFACE_SNAP));
        surface_snap.limiter_mut().set_max_width(i32::MAX);
        surface_snap.set_status_text(&fl!("status_prefab_editor_surface_snap"));
        inspector.add_pair(fl!("prefab_editor_surface_snap"), Box::new(surface_snap));

        let mut surface_tags = TheTextLineEdit::new(TheId::named(SUPPORT_SURFACE_TAGS));
        surface_tags.limiter_mut().set_max_width(i32::MAX);
        surface_tags.set_status_text(&fl!("status_prefab_editor_surface_tags"));
        inspector.add_pair(fl!("prefab_editor_surface_tags"), Box::new(surface_tags));

        let mut surface_capacity = TheTextLineEdit::new(TheId::named(SUPPORT_SURFACE_CAPACITY));
        surface_capacity.limiter_mut().set_max_width(i32::MAX);
        surface_capacity.set_status_text(&fl!("status_prefab_editor_surface_capacity"));
        inspector.add_pair(
            fl!("prefab_editor_surface_capacity"),
            Box::new(surface_capacity),
        );

        let mut surface_policy = TheDropdownMenu::new(TheId::named(SUPPORT_SURFACE_POLICY));
        surface_policy.add_option(fl!("prefab_editor_surface_policy_reject"));
        surface_policy.add_option(fl!("prefab_editor_surface_policy_allow"));
        surface_policy.add_option(fl!("prefab_editor_surface_policy_single"));
        surface_policy.limiter_mut().set_max_width(i32::MAX);
        surface_policy.set_status_text(&fl!("status_prefab_editor_surface_policy"));
        inspector.add_pair(
            fl!("prefab_editor_surface_policy"),
            Box::new(surface_policy),
        );

        canvas.set_layout(inspector);
        canvas
    }

    fn open_support_surface_popover(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        asset_id: Uuid,
        anchor_name: &str,
    ) -> bool {
        let Some((anchor_id, anchor)) = ui
            .get_widget(anchor_name)
            .map(|widget| (widget.id().clone(), *widget.dim()))
        else {
            return false;
        };
        ui.show_popover(anchor_id, anchor, Self::support_surface_popup_canvas(), ctx);
        self.sync_part_inspector(ui, ctx, project, asset_id);
        true
    }

    fn active_asset_id(server_ctx: &ServerContext) -> Option<Uuid> {
        match server_ctx.pc {
            ProjectContext::Prefab(asset_id) => Some(asset_id),
            _ => None,
        }
    }

    fn support_surface_matches_selection(
        project: &Project,
        asset_id: Uuid,
        surface_id: Uuid,
    ) -> bool {
        let Some(map) = project.prefab_editor_map.as_ref() else {
            return false;
        };
        let Some(surface) = project
            .block_props
            .get(&asset_id)
            .and_then(|asset| asset.find_support_surface(surface_id))
        else {
            return false;
        };
        let rusterix::BlockPropSemanticShape::Faces(face_refs) = &surface.shape else {
            return false;
        };
        if face_refs.len() != map.selected_geometry_faces.len() {
            return false;
        }
        map.selected_geometry_faces
            .iter()
            .all(|(object_id, face_index)| {
                map.geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                    .and_then(|object| object.faces.get(*face_index))
                    .is_some_and(|face| {
                        face_refs.iter().any(|face_ref| {
                            face_ref.object_id == *object_id && face_ref.face_id == face.id
                        })
                    })
            })
    }

    fn build_part_node(
        asset: &rusterix::BlockPropAsset,
        project: &Project,
        part_id: Uuid,
        visited: &mut FxHashSet<Uuid>,
    ) -> Option<TheTreeNode> {
        if !visited.insert(part_id) {
            return None;
        }
        let part = asset.find_part(part_id)?;
        let mut node = TheTreeNode::new(TheId::named_with_id(&part.name, part.id));
        node.set_open(true);

        if let Some(map) = project.prefab_editor_map.as_ref() {
            for object in map.geometry_objects.iter().filter(|object| {
                project.prefab_editor_part_by_object.get(&object.id) == Some(&part_id)
            }) {
                let mut item = TheTreeItem::new(TheId::named_with_id(PART_OBJECT_ITEM, object.id));
                item.set_text(object.name.clone());
                node.add_widget(Box::new(item));
            }
        }

        for surface in asset
            .support_surfaces
            .iter()
            .filter(|surface| surface.part_id == part_id)
        {
            let mut item = TheTreeItem::new(TheId::named_with_id(SUPPORT_SURFACE_ITEM, surface.id));
            item.set_text(fl!(
                "prefab_editor_surface_tree_item",
                name = surface.name.clone()
            ));
            node.add_widget(Box::new(item));
        }

        for child in asset
            .parts
            .iter()
            .filter(|candidate| candidate.parent_part_id == Some(part_id))
        {
            if let Some(child_node) = Self::build_part_node(asset, project, child.id, visited) {
                node.add_child(child_node);
            }
        }
        Some(node)
    }

    fn sync_part_tree(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        asset_id: Uuid,
    ) {
        let Some(asset) = project.block_props.get(&asset_id) else {
            return;
        };
        if self.selected_support_surface_id.is_none()
            && let Some(object_id) = project
                .prefab_editor_map
                .as_ref()
                .and_then(|map| map.selected_geometry_objects.first())
            && let Some(part_id) = project.prefab_editor_part_by_object.get(object_id)
        {
            self.selected_part_id = Some(*part_id);
        }
        if self.selected_support_surface_id.is_some_and(|surface_id| {
            asset
                .support_surfaces
                .iter()
                .all(|surface| surface.id != surface_id)
        }) {
            self.selected_support_surface_id = None;
        }
        if self
            .selected_part_id
            .is_none_or(|id| asset.parts.iter().all(|part| part.id != id))
        {
            self.selected_part_id = asset.parts.first().map(|part| part.id);
        }
        if let Some(tree) = ui.get_tree_layout(PART_TREE) {
            let root = tree.get_root();
            root.childs.clear();
            root.widgets.clear();

            let mut asset_node = TheTreeNode::new(TheId::named_with_id(&asset.name, asset.id));
            asset_node.set_open(true);
            let valid_ids = asset
                .parts
                .iter()
                .map(|part| part.id)
                .collect::<FxHashSet<_>>();
            let mut visited = FxHashSet::default();
            for part in asset.parts.iter().filter(|part| {
                part.parent_part_id
                    .is_none_or(|parent_id| !valid_ids.contains(&parent_id))
            }) {
                if let Some(node) = Self::build_part_node(asset, project, part.id, &mut visited) {
                    asset_node.add_child(node);
                }
            }
            for part in &asset.parts {
                if let Some(node) = Self::build_part_node(asset, project, part.id, &mut visited) {
                    asset_node.add_child(node);
                }
            }
            root.add_child(asset_node);

            if let Some(surface_id) = self.selected_support_surface_id {
                tree.new_item_selected(TheId::named_with_id(SUPPORT_SURFACE_ITEM, surface_id));
            } else if let Some(object_id) = project
                .prefab_editor_map
                .as_ref()
                .and_then(|map| map.selected_geometry_objects.first())
            {
                tree.new_item_selected(TheId::named_with_id(PART_OBJECT_ITEM, *object_id));
            }
            ctx.ui.relayout = true;
        }
        self.sync_part_inspector(ui, ctx, project, asset_id);
    }

    fn sync_part_inspector(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        asset_id: Uuid,
    ) {
        let asset = project.block_props.get(&asset_id);
        let part = self
            .selected_part_id
            .and_then(|part_id| asset.and_then(|asset| asset.find_part(part_id)));
        let surface = self
            .selected_support_surface_id
            .and_then(|surface_id| asset.and_then(|asset| asset.find_support_surface(surface_id)));
        ui.set_widget_value(
            PREFAB_NAME,
            ctx,
            TheValue::Text(asset.map(|asset| asset.name.clone()).unwrap_or_default()),
        );
        ui.set_widget_value(
            PART_NAME,
            ctx,
            TheValue::Text(part.map(|part| part.name.clone()).unwrap_or_default()),
        );
        ui.set_widget_value(
            PART_PIVOT,
            ctx,
            TheValue::Text(
                part.map(|part| {
                    format!(
                        "{:.3}, {:.3}, {:.3}",
                        part.pivot[0], part.pivot[1], part.pivot[2]
                    )
                })
                .unwrap_or_default(),
            ),
        );
        self.parent_options.clear();
        if let Some(dropdown) = ui.get_drop_down_menu(PART_PARENT) {
            dropdown.clear_options();
            dropdown.add_option(fl!("prefab_editor_root_part"));
            self.parent_options.push(None);
            if let Some(asset) = asset {
                for candidate in &asset.parts {
                    if Some(candidate.id) != self.selected_part_id {
                        dropdown.add_option(candidate.name.clone());
                        self.parent_options.push(Some(candidate.id));
                    }
                }
            }
            let selected = self
                .parent_options
                .iter()
                .position(|candidate| *candidate == part.and_then(|part| part.parent_part_id))
                .unwrap_or(0);
            dropdown.set_selected_index(selected as i32);
        }

        self.assignment_options.clear();
        if let Some(dropdown) = ui.get_drop_down_menu(PART_ASSIGNMENT) {
            dropdown.clear_options();
            if let Some(asset) = asset {
                for candidate in &asset.parts {
                    dropdown.add_option(candidate.name.clone());
                    self.assignment_options.push(candidate.id);
                }
            }
            let selected_owner = project
                .prefab_editor_map
                .as_ref()
                .and_then(|map| map.selected_geometry_objects.first())
                .and_then(|object_id| project.prefab_editor_part_by_object.get(object_id));
            let selected = self
                .assignment_options
                .iter()
                .position(|candidate| Some(candidate) == selected_owner)
                .unwrap_or(0);
            dropdown.set_selected_index(selected as i32);
        }

        let door_component = asset.and_then(|asset| {
            asset.components.iter().find(|component| {
                self.selected_part_id.is_some_and(|part_id| {
                    rusterix::block_prop_door_controls_part(component, part_id)
                })
            })
        });
        let door_angle = door_component
            .map(|component| {
                component
                    .properties
                    .get_float_default("angle_degrees", 90.0)
            })
            .unwrap_or(90.0);
        ui.set_widget_value(
            PART_DOOR_ANGLE,
            ctx,
            TheValue::Text(format!("{door_angle:.1}")),
        );
        ui.set_widget_value(
            PART_DOOR_LAYOUT,
            ctx,
            TheValue::Int(
                if door_component.is_some_and(|component| {
                    component.properties.get_id("secondary_part_id").is_some()
                }) {
                    1
                } else {
                    0
                },
            ),
        );
        ui.set_widget_value(
            PART_DOOR_MOTION,
            ctx,
            TheValue::Int(
                if door_component.is_some_and(|component| {
                    component
                        .properties
                        .get_str("motion")
                        .is_some_and(|motion| motion.eq_ignore_ascii_case("Slide"))
                }) {
                    1
                } else {
                    0
                },
            ),
        );
        ui.set_widget_value(
            PART_DOOR_SLIDE_DISTANCE,
            ctx,
            TheValue::Text(format!(
                "{:.3}",
                door_component
                    .map(|component| component
                        .properties
                        .get_float_default("slide_distance", 1.0))
                    .or_else(|| {
                        project.prefab_editor_map.as_ref().and_then(|map| {
                            map.geometry_objects
                                .iter()
                                .find(|object| {
                                    project.prefab_editor_part_by_object.get(&object.id)
                                        == self.selected_part_id.as_ref()
                                })
                                .and_then(|object| {
                                    object.properties.get_float("fitted_slide_distance")
                                })
                        })
                    })
                    .unwrap_or(1.0)
            )),
        );
        ui.set_widget_value(
            PART_DOOR_USAGE_DISTANCE,
            ctx,
            TheValue::Text(format!(
                "{:.3}",
                door_component
                    .map(|component| {
                        component
                            .properties
                            .get_float_default("interaction_range", 3.0)
                    })
                    .unwrap_or(3.0)
            )),
        );
        ui.set_widget_value(
            SUPPORT_SURFACE_NAME,
            ctx,
            TheValue::Text(
                surface
                    .map(|surface| surface.name.clone())
                    .unwrap_or_default(),
            ),
        );
        ui.set_widget_value(
            SUPPORT_SURFACE_SNAP,
            ctx,
            TheValue::Text(
                surface
                    .map(|surface| format!("{:.3}", surface.snap_spacing))
                    .unwrap_or_default(),
            ),
        );
        ui.set_widget_value(
            SUPPORT_SURFACE_TAGS,
            ctx,
            TheValue::Text(
                surface
                    .map(|surface| surface.allowed_item_tags.join(", "))
                    .unwrap_or_default(),
            ),
        );
        ui.set_widget_value(
            SUPPORT_SURFACE_CAPACITY,
            ctx,
            TheValue::Text(
                surface
                    .and_then(|surface| surface.capacity)
                    .map(|capacity| capacity.to_string())
                    .unwrap_or_default(),
            ),
        );
        ui.set_widget_value(
            SUPPORT_SURFACE_POLICY,
            ctx,
            TheValue::Int(
                surface
                    .map(|surface| match &surface.occupancy_policy {
                        rusterix::BlockPropOccupancyPolicy::RejectOverlap => 0,
                        rusterix::BlockPropOccupancyPolicy::AllowOverlap => 1,
                        rusterix::BlockPropOccupancyPolicy::SingleOccupant => 2,
                    })
                    .unwrap_or(0),
            ),
        );
        let editing_parts =
            part.is_some() && surface.is_none() && self.mode == PrefabEditorMode::Parts;
        let editing_surface = surface.is_some() && self.mode == PrefabEditorMode::Parts;
        let has_selected_faces = project
            .prefab_editor_map
            .as_ref()
            .is_some_and(|map| !map.selected_geometry_faces.is_empty());
        if editing_parts {
            ui.set_enabled(PART_CREATE, ctx);
            ui.set_enabled(PART_NAME, ctx);
            ui.set_enabled(PART_PARENT, ctx);
            ui.set_enabled(PART_ASSIGNMENT, ctx);
            ui.set_enabled(PART_SET_PIVOT, ctx);
            ui.set_enabled(PART_REMOVE, ctx);
            ui.set_enabled(PART_DOOR_LAYOUT, ctx);
            ui.set_enabled(PART_DOOR_MOTION, ctx);
            ui.set_enabled(PART_DOOR_ANGLE, ctx);
            ui.set_enabled(PART_DOOR_SLIDE_DISTANCE, ctx);
            ui.set_enabled(PART_DOOR_USAGE_DISTANCE, ctx);
            ui.set_enabled(PART_CONFIGURE_DOOR, ctx);
            ui.set_enabled(PART_PREVIEW_DOOR, ctx);
        } else {
            ui.set_disabled(PART_CREATE, ctx);
            ui.set_disabled(PART_NAME, ctx);
            ui.set_disabled(PART_PARENT, ctx);
            ui.set_disabled(PART_ASSIGNMENT, ctx);
            ui.set_disabled(PART_SET_PIVOT, ctx);
            ui.set_disabled(PART_REMOVE, ctx);
            ui.set_disabled(PART_DOOR_LAYOUT, ctx);
            ui.set_disabled(PART_DOOR_MOTION, ctx);
            ui.set_disabled(PART_DOOR_ANGLE, ctx);
            ui.set_disabled(PART_DOOR_SLIDE_DISTANCE, ctx);
            ui.set_disabled(PART_DOOR_USAGE_DISTANCE, ctx);
            ui.set_disabled(PART_CONFIGURE_DOOR, ctx);
            ui.set_disabled(PART_PREVIEW_DOOR, ctx);
        }
        for id in [
            SUPPORT_SURFACE_NAME,
            SUPPORT_SURFACE_SNAP,
            SUPPORT_SURFACE_TAGS,
            SUPPORT_SURFACE_CAPACITY,
            SUPPORT_SURFACE_POLICY,
        ] {
            if editing_surface {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
        if editing_surface {
            ui.set_enabled(SUPPORT_SURFACE_EDIT, ctx);
            ui.set_enabled(SUPPORT_SURFACE_REMOVE, ctx);
        } else {
            ui.set_disabled(SUPPORT_SURFACE_EDIT, ctx);
            ui.set_disabled(SUPPORT_SURFACE_REMOVE, ctx);
        }
        if self.mode == PrefabEditorMode::Parts
            && self.selected_support_surface_id.is_none()
            && has_selected_faces
        {
            ui.set_enabled(SUPPORT_SURFACE_CREATE, ctx);
        } else {
            ui.set_disabled(SUPPORT_SURFACE_CREATE, ctx);
        }
    }

    fn sync_mode(&self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(stack) = ui.get_stack_layout(MODE_STACK) {
            stack.set_index(self.mode.index() as usize);
        }
        let parts = self.mode == PrefabEditorMode::Parts;
        for id in [
            PART_CREATE,
            PART_SET_PIVOT,
            PART_REMOVE,
            SUPPORT_SURFACE_CREATE,
            SUPPORT_SURFACE_EDIT,
            SUPPORT_SURFACE_REMOVE,
            PART_CONFIGURE_DOOR,
            PART_PREVIEW_DOOR,
        ] {
            let enabled = parts
                && match id {
                    SUPPORT_SURFACE_CREATE => {
                        self.selected_support_surface_id.is_none()
                            && project
                                .prefab_editor_map
                                .as_ref()
                                .is_some_and(|map| !map.selected_geometry_faces.is_empty())
                    }
                    SUPPORT_SURFACE_EDIT | SUPPORT_SURFACE_REMOVE => {
                        self.selected_support_surface_id.is_some()
                    }
                    _ => self.selected_support_surface_id.is_none(),
                };
            if enabled {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
    }

    fn active_tool_mode() -> PrefabEditorMode {
        let tools = TOOLLIST.read().unwrap();
        if tools.palette_mode_active() {
            return PrefabEditorMode::Palette;
        }
        match tools.current_game_tool_command_id() {
            Some("tool.iso_paint") => PrefabEditorMode::Paint,
            Some("tool.tile_picker") => PrefabEditorMode::Tiles,
            _ => PrefabEditorMode::Parts,
        }
    }

    fn push_project_undo(before: Project, project: &Project, ctx: &mut TheContext) {
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                fl!("undo_prefab_parts_edit"),
                Box::new(before),
                Box::new(project.clone()),
            ),
            ctx,
        );
    }

    fn selected_door_component_id(&self, project: &Project, asset_id: Uuid) -> Option<Uuid> {
        let part_id = self.selected_part_id?;
        project.block_props.get(&asset_id).and_then(|asset| {
            asset
                .components
                .iter()
                .find(|component| rusterix::block_prop_door_controls_part(component, part_id))
                .map(|component| component.id)
        })
    }

    fn close_door_preview(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
        server_ctx: &ServerContext,
    ) -> bool {
        if !self.door_preview_open {
            return false;
        }
        let before = project.prefab_editor_map.clone();
        if crate::block_props::begin_prefab_editor(project, asset_id).is_ok()
            && let Some(part_id) = self.selected_part_id
        {
            crate::block_props::select_prefab_part(project, part_id);
        }
        self.door_preview_open = false;
        let after = project.prefab_editor_map.clone();
        if let (Some(before), Some(after)) = (before, after) {
            crate::utils::editor_scene_apply_map_edit(project, server_ctx, &before, &after);
        }
        true
    }

    fn open_door_preview(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
        server_ctx: &ServerContext,
    ) -> Result<(), String> {
        let before = project
            .prefab_editor_map
            .clone()
            .ok_or_else(|| fl!("error_prefab_editor_not_open"))?;
        let component_id = self
            .selected_door_component_id(project, asset_id)
            .ok_or_else(|| fl!("status_prefab_door_required"))?;
        let asset = project
            .block_props
            .get(&asset_id)
            .ok_or_else(|| fl!("error_prefab_editor_project_asset"))?;
        let mut instance = rusterix::BlockPropInstance::new(asset_id);
        rusterix::set_block_prop_door_open(&mut instance, component_id, true);
        let resolution =
            rusterix::resolve_block_prop_preview_geometry(asset, instance.runtime_state);
        let map = project
            .prefab_editor_map
            .as_mut()
            .ok_or_else(|| fl!("error_prefab_editor_not_open"))?;
        map.geometry_objects = resolution.geometry_objects;
        map.update_surfaces();
        self.door_preview_open = true;
        let after = project
            .prefab_editor_map
            .clone()
            .ok_or_else(|| fl!("error_prefab_editor_not_open"))?;
        crate::utils::editor_scene_apply_map_edit(project, server_ctx, &before, &after);
        Ok(())
    }

    fn sync_prefab_runtime(project: &mut Project) {
        let block_props = &project.block_props;
        for region in &mut project.regions {
            rusterix::sync_block_prop_surface_item_positions(
                &region.map.block_prop_instances,
                &region.map.block_prop_surface_placements,
                &mut region.map.items,
                block_props,
            );
            for item in region.items.values_mut() {
                if let Some(runtime_item) = region
                    .map
                    .items
                    .iter()
                    .find(|runtime_item| runtime_item.creator_id == item.id)
                {
                    item.position = runtime_item.position;
                }
            }
        }
        let prefabs = project.block_props.clone();
        RUSTERIX.write().unwrap().set_block_props(prefabs.clone());
        SCENEMANAGER.write().unwrap().set_block_props(prefabs);
    }
}

impl Dock for PrefabsEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            mode: PrefabEditorMode::Parts,
            selected_part_id: None,
            selected_support_surface_id: None,
            parent_options: Vec::new(),
            assignment_options: Vec::new(),
            door_preview_open: false,
            paint_dock: IsoPaintDock::new_prefab(),
            tiles_dock: TilesDock::new_prefab(),
            palette_dock: PaletteDock::new_prefab(),
        }
    }

    fn setup(&mut self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut split = TheSharedVLayout::new(TheId::named("Prefab Editor Shared VLayout"));
        split.set_shared_ratio(0.68);
        split.set_mode(TheSharedVLayoutMode::Shared);

        let mut view_canvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named(PREFAB_VIEW));
        render_view.set_auto_focus(true);
        view_canvas.set_widget(render_view);
        split.add_canvas(view_canvas);

        let mut lower_content = TheCanvas::new();
        let mut stack = TheStackLayout::new(TheId::named(MODE_STACK));
        stack.add_canvas(Self::parts_canvas());
        stack.add_canvas(self.paint_dock.setup(ctx));
        stack.add_canvas(self.tiles_dock.setup(ctx));
        stack.add_canvas(self.palette_dock.setup(ctx));
        lower_content.set_layout(stack);

        // Actions live in the global sidebar. Keeping another action list here
        // duplicated controls and unnecessarily narrowed the Prefab inspector.
        split.add_canvas(lower_content);

        canvas.set_layout(split);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        let Some(asset_id) = Self::active_asset_id(server_ctx) else {
            return;
        };
        self.mode = PrefabEditorMode::Parts;
        self.selected_support_surface_id = None;
        self.door_preview_open = false;
        self.sync_mode(ui, ctx, project);
        self.sync_part_tree(ui, ctx, project, asset_id);
        self.paint_dock.activate(ui, ctx, project, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
    }

    fn minimized(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext) {
        // The preview only replaces geometry in the isolated editor map. Mark
        // it closed as soon as that editor goes away so no preview lifecycle
        // state survives into a later maximize session.
        self.door_preview_open = false;
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(asset_id) = Self::active_asset_id(server_ctx) else {
            return false;
        };
        if let Some(event) = Self::translated_view_event(event) {
            if self.close_door_preview(project, asset_id, server_ctx) {
                self.sync_part_tree(ui, ctx, project, asset_id);
            }
            ctx.ui.send(event);
            return true;
        }
        if self.mode == PrefabEditorMode::Paint
            && self
                .paint_dock
                .handle_event(event, ui, ctx, project, server_ctx)
        {
            return true;
        }
        if self.mode == PrefabEditorMode::Tiles {
            let edits_prefab = self.tiles_dock.edits_map_for_event(event);
            let redraw = self
                .tiles_dock
                .handle_event(event, ui, ctx, project, server_ctx);
            if edits_prefab {
                match crate::block_props::sync_prefab_editor(project, asset_id) {
                    Ok(()) => Self::sync_prefab_runtime(project),
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
            }
            if redraw || edits_prefab {
                return true;
            }
        }
        if self.mode == PrefabEditorMode::Palette
            && self
                .palette_dock
                .handle_event(event, ui, ctx, project, server_ctx)
        {
            return true;
        }

        match event {
            TheEvent::Custom(id, _) if id.name == "Tool Changed" => {
                self.close_door_preview(project, asset_id, server_ctx);
                self.mode = Self::active_tool_mode();
                self.sync_mode(ui, ctx, project);
                if self.mode == PrefabEditorMode::Parts {
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                }
                if self.mode == PrefabEditorMode::Paint {
                    self.paint_dock.activate(ui, ctx, project, server_ctx);
                } else if self.mode == PrefabEditorMode::Tiles {
                    self.tiles_dock.activate(ui, ctx, project, server_ctx);
                } else if self.mode == PrefabEditorMode::Palette {
                    self.palette_dock.activate(ui, ctx, project, server_ctx);
                }
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::SnapperStateChanged(id, _, _)
                if project
                    .block_props
                    .get(&asset_id)
                    .is_some_and(|asset| asset.parts.iter().any(|part| part.id == id.uuid)) =>
            {
                self.selected_part_id = Some(id.uuid);
                self.selected_support_surface_id = None;
                crate::block_props::select_prefab_part(project, id.uuid);
                self.sync_part_inspector(ui, ctx, project, asset_id);
                TOOLLIST
                    .write()
                    .unwrap()
                    .update_geometry_overlay_3d(project, server_ctx);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::NewListItemSelected(id, layout_id)
                if id.name == PART_OBJECT_ITEM && layout_id.name == PART_TREE =>
            {
                self.selected_support_surface_id = None;
                if let Some(map) = project.prefab_editor_map.as_mut()
                    && map
                        .geometry_objects
                        .iter()
                        .any(|object| object.id == id.uuid)
                {
                    map.clear_selection();
                    map.selected_geometry_objects.push(id.uuid);
                }
                self.selected_part_id = project.prefab_editor_part_by_object.get(&id.uuid).copied();
                self.sync_part_inspector(ui, ctx, project, asset_id);
                TOOLLIST
                    .write()
                    .unwrap()
                    .update_geometry_overlay_3d(project, server_ctx);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::NewListItemSelected(id, layout_id)
                if id.name == SUPPORT_SURFACE_ITEM && layout_id.name == PART_TREE =>
            {
                match crate::block_props::select_prefab_support_surface(project, asset_id, id.uuid)
                {
                    Ok(part_id) => {
                        self.selected_part_id = Some(part_id);
                        self.selected_support_surface_id = Some(id.uuid);
                        self.sync_part_inspector(ui, ctx, project, asset_id);
                        // Re-activating the already active face tool clears its
                        // selection. Only switch when necessary; a real switch
                        // carries these selected faces into face mode.
                        if TOOLLIST.read().unwrap().current_game_tool_command_id()
                            != Some("tool.sector")
                        {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Set Tool"),
                                TheValue::Text("tool.sector".to_string()),
                            ));
                        }
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(project, server_ctx);
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == PART_PARENT => {
                let Some(part_id) = self.selected_part_id else {
                    return false;
                };
                let Some(parent_id) = self.parent_options.get(*index).copied() else {
                    return false;
                };
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_part(part_id))
                    .and_then(|part| part.parent_part_id);
                if current == parent_id {
                    return true;
                }
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                match crate::block_props::set_prefab_part_parent(
                    project, asset_id, part_id, parent_id,
                ) {
                    Ok(()) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_parent_changed"),
                        ));
                    }
                    Err(message) => {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), message));
                        self.sync_part_inspector(ui, ctx, project, asset_id);
                    }
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == PART_ASSIGNMENT => {
                let Some(part_id) = self.assignment_options.get(*index).copied() else {
                    return false;
                };
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                match crate::block_props::move_prefab_selection_to_part(project, asset_id, part_id)
                {
                    Ok(count) => {
                        self.selected_part_id = Some(part_id);
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_objects_reassigned", count = count),
                        ));
                    }
                    Err(message) => {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), message));
                        self.sync_part_inspector(ui, ctx, project, asset_id);
                    }
                }
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(name)) if id.name == PART_NAME => {
                let Some(part_id) = self.selected_part_id else {
                    return false;
                };
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_part(part_id))
                    .map(|part| part.name.as_str());
                if current == Some(name.trim()) || name.trim().is_empty() {
                    return false;
                }
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                if let Err(message) =
                    crate::block_props::rename_prefab_part(project, asset_id, part_id, name.clone())
                {
                    ctx.ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message));
                    return true;
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                self.sync_part_tree(ui, ctx, project, asset_id);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(name)) if id.name == PREFAB_NAME => {
                let current = project
                    .block_props
                    .get(&asset_id)
                    .map(|asset| asset.name.as_str());
                if current == Some(name.trim()) {
                    return true;
                }
                if name.trim().is_empty() {
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_prefab_name_required"),
                    ));
                    return true;
                }
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                if let Err(message) =
                    crate::block_props::rename_prefab_asset(project, asset_id, name.clone())
                {
                    ctx.ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message));
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                    return true;
                }
                server_ctx.curr_block_asset_name = project
                    .block_props
                    .get(&asset_id)
                    .map(|asset| asset.name.clone());
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                self.sync_part_tree(ui, ctx, project, asset_id);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_prefab_renamed"),
                ));
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(name)) if id.name == SUPPORT_SURFACE_NAME => {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let name = name.trim();
                if name.is_empty() {
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                    return true;
                }
                let Some(surface) = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_support_surface(surface_id))
                else {
                    return false;
                };
                if surface.name == name {
                    return true;
                }
                let before = project.clone();
                if let Some(surface) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .support_surfaces
                        .iter_mut()
                        .find(|surface| surface.id == surface_id)
                }) {
                    surface.name = name.to_string();
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                self.sync_part_tree(ui, ctx, project, asset_id);
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(value))
                if id.name == SUPPORT_SURFACE_SNAP =>
            {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let Some(spacing) = value
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|value| *value >= 0.0)
                else {
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                    return true;
                };
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_support_surface(surface_id))
                    .map(|surface| surface.snap_spacing);
                if current.is_some_and(|current| (current - spacing).abs() < f32::EPSILON) {
                    return true;
                }
                let before = project.clone();
                if let Some(surface) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .support_surfaces
                        .iter_mut()
                        .find(|surface| surface.id == surface_id)
                }) {
                    surface.snap_spacing = spacing;
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(value))
                if id.name == SUPPORT_SURFACE_TAGS =>
            {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let mut seen = FxHashSet::default();
                let tags = value
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .filter(|tag| seen.insert(tag.to_ascii_lowercase()))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_support_surface(surface_id))
                    .map(|surface| &surface.allowed_item_tags);
                if current == Some(&tags) {
                    return true;
                }
                let before = project.clone();
                if let Some(surface) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .support_surfaces
                        .iter_mut()
                        .find(|surface| surface.id == surface_id)
                }) {
                    surface.allowed_item_tags = tags;
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(value))
                if id.name == SUPPORT_SURFACE_CAPACITY =>
            {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let capacity = if value.trim().is_empty() {
                    None
                } else if let Some(capacity) = value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|capacity| *capacity > 0)
                {
                    Some(capacity)
                } else {
                    self.sync_part_inspector(ui, ctx, project, asset_id);
                    return true;
                };
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_support_surface(surface_id))
                    .and_then(|surface| surface.capacity);
                if current == capacity {
                    return true;
                }
                let before = project.clone();
                if let Some(surface) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .support_surfaces
                        .iter_mut()
                        .find(|surface| surface.id == surface_id)
                }) {
                    surface.capacity = capacity;
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == SUPPORT_SURFACE_POLICY => {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let policy = match *index {
                    1 => rusterix::BlockPropOccupancyPolicy::AllowOverlap,
                    2 => rusterix::BlockPropOccupancyPolicy::SingleOccupant,
                    _ => rusterix::BlockPropOccupancyPolicy::RejectOverlap,
                };
                let current = project
                    .block_props
                    .get(&asset_id)
                    .and_then(|asset| asset.find_support_surface(surface_id))
                    .map(|surface| &surface.occupancy_policy);
                if current == Some(&policy) {
                    return true;
                }
                let before = project.clone();
                if let Some(surface) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .support_surfaces
                        .iter_mut()
                        .find(|surface| surface.id == surface_id)
                }) {
                    surface.occupancy_policy = policy;
                }
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == PART_CREATE => {
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                let number = project
                    .block_props
                    .get(&asset_id)
                    .map(|asset| asset.parts.len() + 1)
                    .unwrap_or(1);
                match crate::block_props::create_prefab_part_from_selection(
                    project,
                    asset_id,
                    fl!("prefab_editor_default_part", number = number),
                ) {
                    Ok(part_id) => {
                        self.selected_part_id = Some(part_id);
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_part_created"),
                        ));
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == SUPPORT_SURFACE_CREATE =>
            {
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                let number = project
                    .block_props
                    .get(&asset_id)
                    .map(|asset| asset.support_surfaces.len() + 1)
                    .unwrap_or(1);
                match crate::block_props::create_prefab_support_surface_from_selection(
                    project,
                    asset_id,
                    fl!("prefab_editor_default_surface", number = number),
                ) {
                    Ok((surface_id, part_id, face_count)) => {
                        self.selected_support_surface_id = Some(surface_id);
                        self.selected_part_id = Some(part_id);
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        self.open_support_surface_popover(
                            ui,
                            ctx,
                            project,
                            asset_id,
                            SUPPORT_SURFACE_CREATE,
                        );
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(project, server_ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_surface_created", count = face_count),
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == SUPPORT_SURFACE_EDIT =>
            {
                if self.selected_support_surface_id.is_none() {
                    return false;
                }
                self.open_support_surface_popover(ui, ctx, project, asset_id, SUPPORT_SURFACE_EDIT)
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == SUPPORT_SURFACE_REMOVE =>
            {
                let Some(surface_id) = self.selected_support_surface_id else {
                    return false;
                };
                let before = project.clone();
                match crate::block_props::remove_prefab_support_surface(
                    project, asset_id, surface_id,
                ) {
                    Ok(part_id) => {
                        self.selected_support_surface_id = None;
                        self.selected_part_id = Some(part_id);
                        crate::block_props::select_prefab_part(project, part_id);
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(project, server_ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_surface_removed"),
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == PART_SET_PIVOT => {
                let Some(part_id) = self.selected_part_id else {
                    return false;
                };
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                match crate::block_props::set_prefab_part_pivot_from_selection(
                    project, asset_id, part_id,
                ) {
                    Ok(pivot) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_inspector(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!(
                                "status_prefab_part_pivot_set",
                                x = format!("{:.3}", pivot[0]),
                                y = format!("{:.3}", pivot[1]),
                                z = format!("{:.3}", pivot[2])
                            ),
                        ));
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == PART_CONFIGURE_DOOR =>
            {
                let Some(selected_part_id) = self.selected_part_id else {
                    return false;
                };
                self.close_door_preview(project, asset_id, server_ctx);
                let angle = ui
                    .get_widget_value(PART_DOOR_ANGLE)
                    .and_then(|value| match value {
                        TheValue::Text(text) => text.trim().parse::<f32>().ok(),
                        _ => None,
                    })
                    .unwrap_or(90.0);
                let slide_distance = ui
                    .get_widget_value(PART_DOOR_SLIDE_DISTANCE)
                    .and_then(|value| match value {
                        TheValue::Text(text) => text.trim().parse::<f32>().ok(),
                        _ => None,
                    })
                    .unwrap_or(1.0);
                let usage_distance = ui
                    .get_widget_value(PART_DOOR_USAGE_DISTANCE)
                    .and_then(|value| match value {
                        TheValue::Text(text) => text.trim().parse::<f32>().ok(),
                        _ => None,
                    })
                    .unwrap_or(3.0);
                let split = ui
                    .get_widget_value(PART_DOOR_LAYOUT)
                    .and_then(|value| value.to_i32())
                    .unwrap_or(0)
                    == 1;
                let motion = if ui
                    .get_widget_value(PART_DOOR_MOTION)
                    .and_then(|value| value.to_i32())
                    .unwrap_or(0)
                    == 1
                {
                    crate::block_props::PrefabDoorMotion::Slide
                } else {
                    crate::block_props::PrefabDoorMotion::Swing
                };
                let before = project.clone();
                let existing_component = project.block_props.get(&asset_id).and_then(|asset| {
                    asset
                        .components
                        .iter()
                        .find(|component| {
                            rusterix::block_prop_door_controls_part(component, selected_part_id)
                        })
                        .cloned()
                });
                let prepared = if split {
                    if let Some(component) = existing_component.as_ref()
                        && let (Some(primary), Some(secondary)) = (
                            component.properties.get_id("part_id"),
                            component.properties.get_id("secondary_part_id"),
                        )
                    {
                        Ok((
                            primary,
                            secondary,
                            component
                                .properties
                                .get_vec3_default("slide_axis", [1.0, 0.0, 0.0]),
                        ))
                    } else {
                        crate::block_props::prepare_prefab_split_door_parts(project, asset_id)
                    }
                } else {
                    let axis = project
                        .prefab_editor_map
                        .as_ref()
                        .and_then(|map| {
                            map.geometry_objects.iter().find(|object| {
                                project.prefab_editor_part_by_object.get(&object.id)
                                    == Some(&selected_part_id)
                            })
                        })
                        .and_then(|object| object.properties.get_vec3("fitted_motion_axis"))
                        .unwrap_or([1.0, 0.0, 0.0]);
                    Ok((selected_part_id, Uuid::nil(), axis))
                };
                let result = prepared.and_then(|(part_id, secondary_part_id, slide_axis)| {
                    self.selected_part_id = Some(part_id);
                    crate::block_props::configure_prefab_door_with_options(
                        project,
                        asset_id,
                        part_id,
                        crate::block_props::PrefabDoorOptions {
                            secondary_part_id: split.then_some(secondary_part_id),
                            motion,
                            angle_degrees: angle,
                            slide_distance,
                            interaction_range: usage_distance,
                            slide_axis,
                        },
                    )
                });
                match result {
                    Ok(_) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_door_configured"),
                        ));
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == PART_PREVIEW_DOOR => {
                if self.door_preview_open {
                    self.close_door_preview(project, asset_id, server_ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_prefab_door_preview_closed"),
                    ));
                } else {
                    match self.open_door_preview(project, asset_id, server_ctx) {
                        Ok(()) => ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_door_preview_open"),
                        )),
                        Err(message) => ctx
                            .ui
                            .send(TheEvent::SetStatusText(TheId::empty(), message)),
                    }
                }
                TOOLLIST
                    .write()
                    .unwrap()
                    .update_geometry_overlay_3d(project, server_ctx);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == PART_REMOVE => {
                let Some(part_id) = self.selected_part_id else {
                    return false;
                };
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                match crate::block_props::remove_prefab_part(project, asset_id, part_id) {
                    Ok(fallback_id) => {
                        self.selected_part_id = Some(fallback_id);
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_tree(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_part_removed"),
                        ));
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == "Map Selection Changed" => {
                if self.selected_support_surface_id.is_some_and(|surface_id| {
                    !Self::support_surface_matches_selection(project, asset_id, surface_id)
                }) {
                    self.selected_support_surface_id = None;
                }

                if self.selected_support_surface_id.is_none() {
                    self.selected_part_id = project
                        .prefab_editor_map
                        .as_ref()
                        .and_then(|map| map.selected_geometry_objects.first())
                        .and_then(|object_id| project.prefab_editor_part_by_object.get(object_id))
                        .copied()
                        .or(self.selected_part_id);
                }

                // A selection change does not alter the hierarchy. Keep the
                // existing tree and only move its selection marker; rebuilding
                // it made support-surface clicks look like a full UI refresh.
                if let Some(tree) = ui.get_tree_layout(PART_TREE) {
                    if let Some(surface_id) = self.selected_support_surface_id {
                        tree.new_item_selected(TheId::named_with_id(
                            SUPPORT_SURFACE_ITEM,
                            surface_id,
                        ));
                    } else if let Some(object_id) = project
                        .prefab_editor_map
                        .as_ref()
                        .and_then(|map| map.selected_geometry_objects.first())
                    {
                        tree.new_item_selected(TheId::named_with_id(PART_OBJECT_ITEM, *object_id));
                    } else {
                        tree.get_root().clear_selection();
                    }
                }
                self.sync_part_inspector(ui, ctx, project, asset_id);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::Custom(id, _) if id.name == crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT => {
                self.sync_part_tree(ui, ctx, project, asset_id);
                self.paint_dock.activate(ui, ctx, project, server_ctx);
                if self.mode == PrefabEditorMode::Tiles {
                    self.tiles_dock.activate(ui, ctx, project, server_ctx);
                } else if self.mode == PrefabEditorMode::Palette {
                    self.palette_dock.activate(ui, ctx, project, server_ctx);
                }
                true
            }
            _ => false,
        }
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        self.mode == PrefabEditorMode::Tiles
            && self
                .tiles_dock
                .draw_minimap(buffer, project, ctx, server_ctx)
    }

    fn supports_minimap_animation(&self) -> bool {
        self.mode == PrefabEditorMode::Tiles && self.tiles_dock.supports_minimap_animation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_view_input_is_translated_to_geometry_view_input() {
        let event = TheEvent::RenderViewClicked(TheId::named(PREFAB_VIEW), Vec2::new(12, 34));
        let translated = PrefabsEditorDock::translated_view_event(&event).unwrap();
        assert!(matches!(
            translated,
            TheEvent::RenderViewClicked(id, coord)
                if id.name == MAP_VIEW && coord == Vec2::new(12, 34)
        ));
    }

    #[test]
    fn prefab_editor_modes_have_stable_stack_indices() {
        assert_eq!(PrefabEditorMode::Parts.index(), 0);
        assert_eq!(PrefabEditorMode::Paint.index(), 1);
        assert_eq!(PrefabEditorMode::Tiles.index(), 2);
        assert_eq!(PrefabEditorMode::Palette.index(), 3);
    }
}
