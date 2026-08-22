use crate::docks::iso_paint::IsoPaintDock;
use crate::editor::{RUSTERIX, SCENEMANAGER, TOOLLIST, UNDOMANAGER};
use crate::prelude::*;

const PREFAB_VIEW: &str = "PrefabView";
const MAP_VIEW: &str = "PolyView";
const MODE_STACK: &str = "Prefab Editor Mode Stack";
const PART_TREE: &str = "Prefab Editor Part Tree";
const PART_OBJECT_ITEM: &str = "Prefab Editor Geometry Object";
const PART_NAME: &str = "Prefab Editor Part Name";
const PART_PARENT: &str = "Prefab Editor Part Parent";
const PART_ASSIGNMENT: &str = "Prefab Editor Object Assignment";
const PART_PIVOT: &str = "Prefab Editor Part Pivot";
const PART_DOOR_ANGLE: &str = "Prefab Editor Door Angle";
const PART_BEHAVIOR: &str = "Prefab Editor Part Behavior";
const PART_TARGET_COUNT: &str = "Prefab Editor Part Target Count";
const PART_CREATE: &str = "Prefab Editor Create Part";
const PART_SET_PIVOT: &str = "Prefab Editor Set Pivot";
const PART_REMOVE: &str = "Prefab Editor Remove Part";
const PART_CONFIGURE_DOOR: &str = "Prefab Editor Configure Door";
const PART_CREATE_TARGET: &str = "Prefab Editor Create Interaction Target";
const PART_PREVIEW_DOOR: &str = "Prefab Editor Preview Door";
const PREFAB_ACTION_LIST: &str = "Prefab Action List";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PrefabEditorMode {
    #[default]
    Parts,
    Paint,
}

impl PrefabEditorMode {
    fn index(self) -> i32 {
        match self {
            Self::Parts => 0,
            Self::Paint => 1,
        }
    }
}

/// Full-screen editor shell for authored Prefabs.
///
/// Geometry tools still consume the established PolyView event contract. The
/// dedicated canvas translates its input at the dock boundary, keeping the
/// region canvas and its visual state completely separate. Its lower split is
/// owned by the Prefab editor and therefore remains available in maximized mode.
pub struct PrefabsEditorDock {
    mode: PrefabEditorMode,
    selected_part_id: Option<Uuid>,
    parent_options: Vec<Option<Uuid>>,
    assignment_options: Vec<Uuid>,
    door_preview_open: bool,
    paint_dock: IsoPaintDock,
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
                PART_CONFIGURE_DOOR,
                fl!("prefab_editor_configure_door"),
                fl!("status_prefab_editor_configure_door"),
            ),
            (
                PART_CREATE_TARGET,
                fl!("prefab_editor_create_target"),
                fl!("status_prefab_editor_create_target"),
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
        layout.set_reverse_index(Some(3));
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
        inspector.set_fixed_text_width(88);
        inspector.set_text_align(TheHorizontalAlign::Right);

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
        inspector.add_pair(fl!("prefab_editor_part_pivot"), Box::new(pivot));

        let mut door_angle = TheTextLineEdit::new(TheId::named(PART_DOOR_ANGLE));
        door_angle.limiter_mut().set_max_width(i32::MAX);
        door_angle.set_value(TheValue::Text("90".to_string()));
        door_angle.set_status_text(&fl!("status_prefab_editor_door_angle"));
        inspector.add_pair(fl!("prefab_editor_door_angle"), Box::new(door_angle));

        let mut behavior = TheTextLineEdit::new(TheId::named(PART_BEHAVIOR));
        behavior.limiter_mut().set_max_width(i32::MAX);
        behavior.set_disabled(true);
        inspector.add_pair(fl!("prefab_editor_behavior"), Box::new(behavior));

        let mut targets = TheTextLineEdit::new(TheId::named(PART_TARGET_COUNT));
        targets.limiter_mut().set_max_width(i32::MAX);
        targets.set_disabled(true);
        inspector.add_pair(fl!("prefab_editor_targets"), Box::new(targets));

        inspector_canvas.set_layout(inspector);

        let mut split = TheSharedHLayout::new(TheId::named("Prefab Parts Shared HLayout"));
        split.set_shared_ratio(0.58);
        split.set_mode(TheSharedHLayoutMode::Shared);
        split.add_canvas(tree_canvas);
        split.add_canvas(inspector_canvas);
        let mut content = TheCanvas::new();
        content.set_layout(split);

        canvas.set_center(content);
        canvas.set_top(Self::part_actions_toolbar());
        canvas
    }

    fn active_asset_id(server_ctx: &ServerContext) -> Option<Uuid> {
        match server_ctx.pc {
            ProjectContext::Prefab(asset_id) => Some(asset_id),
            _ => None,
        }
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
        if let Some(object_id) = project
            .prefab_editor_map
            .as_ref()
            .and_then(|map| map.selected_geometry_objects.first())
            && let Some(part_id) = project.prefab_editor_part_by_object.get(object_id)
        {
            self.selected_part_id = Some(*part_id);
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

            if let Some(object_id) = project
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
                component.kind == "Door"
                    && component.properties.get_id("part_id") == self.selected_part_id
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
            PART_BEHAVIOR,
            ctx,
            TheValue::Text(if door_component.is_some() {
                fl!("prefab_editor_behavior_door")
            } else {
                fl!("prefab_editor_behavior_none")
            }),
        );
        let target_count = asset
            .map(|asset| {
                asset
                    .interaction_targets
                    .iter()
                    .filter(|target| Some(target.part_id) == self.selected_part_id)
                    .count()
            })
            .unwrap_or(0);
        ui.set_widget_value(
            PART_TARGET_COUNT,
            ctx,
            TheValue::Text(target_count.to_string()),
        );
        if part.is_some() && self.mode == PrefabEditorMode::Parts {
            ui.set_enabled(PART_NAME, ctx);
            ui.set_enabled(PART_PARENT, ctx);
            ui.set_enabled(PART_ASSIGNMENT, ctx);
            ui.set_enabled(PART_SET_PIVOT, ctx);
            ui.set_enabled(PART_REMOVE, ctx);
            ui.set_enabled(PART_DOOR_ANGLE, ctx);
            ui.set_enabled(PART_CONFIGURE_DOOR, ctx);
            ui.set_enabled(PART_CREATE_TARGET, ctx);
            ui.set_enabled(PART_PREVIEW_DOOR, ctx);
        } else {
            ui.set_disabled(PART_NAME, ctx);
            ui.set_disabled(PART_PARENT, ctx);
            ui.set_disabled(PART_ASSIGNMENT, ctx);
            ui.set_disabled(PART_SET_PIVOT, ctx);
            ui.set_disabled(PART_REMOVE, ctx);
            ui.set_disabled(PART_DOOR_ANGLE, ctx);
            ui.set_disabled(PART_CONFIGURE_DOOR, ctx);
            ui.set_disabled(PART_CREATE_TARGET, ctx);
            ui.set_disabled(PART_PREVIEW_DOOR, ctx);
        }
    }

    fn sync_mode(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(stack) = ui.get_stack_layout(MODE_STACK) {
            stack.set_index(self.mode.index() as usize);
        }
        let parts = self.mode == PrefabEditorMode::Parts;
        for id in [
            PART_CREATE,
            PART_SET_PIVOT,
            PART_REMOVE,
            PART_CONFIGURE_DOOR,
            PART_CREATE_TARGET,
            PART_PREVIEW_DOOR,
        ] {
            if parts {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
    }

    fn active_tool_mode() -> PrefabEditorMode {
        let tools = TOOLLIST.read().unwrap();
        if tools.current_game_tool_command_id() == Some("tool.iso_paint") {
            PrefabEditorMode::Paint
        } else {
            PrefabEditorMode::Parts
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
                .find(|component| {
                    component.kind == "Door"
                        && component.properties.get_id("part_id") == Some(part_id)
                })
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

    fn sync_prefab_runtime(project: &Project) {
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
            parent_options: Vec::new(),
            assignment_options: Vec::new(),
            door_preview_open: false,
            paint_dock: IsoPaintDock::new_prefab(),
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
        lower_content.set_layout(stack);

        let mut lower = TheSharedHLayout::new(TheId::named("Prefab Editor Lower Shared HLayout"));
        lower.set_shared_ratio(0.77);
        lower.set_mode(TheSharedHLayoutMode::Shared);
        lower.add_canvas(lower_content);
        lower.add_canvas(crate::dockmanager::DockManager::action_panel(
            PREFAB_ACTION_LIST,
        ));
        let mut lower_canvas = TheCanvas::new();
        lower_canvas.set_layout(lower);
        split.add_canvas(lower_canvas);

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
        self.door_preview_open = false;
        self.sync_mode(ui, ctx);
        self.sync_part_tree(ui, ctx, project, asset_id);
        self.paint_dock.activate(ui, ctx, project, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
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

        match event {
            TheEvent::Custom(id, _) if id.name == "Tool Changed" => {
                self.close_door_preview(project, asset_id, server_ctx);
                self.mode = Self::active_tool_mode();
                self.sync_mode(ui, ctx);
                if self.mode == PrefabEditorMode::Paint {
                    self.paint_dock.activate(ui, ctx, project, server_ctx);
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
                let Some(part_id) = self.selected_part_id else {
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
                let before = project.clone();
                match crate::block_props::configure_prefab_door(project, asset_id, part_id, angle) {
                    Ok(_) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_inspector(ui, ctx, project, asset_id);
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
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == PART_CREATE_TARGET =>
            {
                let Some(part_id) = self.selected_part_id else {
                    return false;
                };
                self.close_door_preview(project, asset_id, server_ctx);
                let before = project.clone();
                match crate::block_props::create_prefab_interaction_target_from_selected_faces(
                    project,
                    asset_id,
                    part_id,
                    fl!("prefab_editor_default_door_target"),
                ) {
                    Ok(_) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_part_inspector(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_prefab_target_created"),
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
                self.sync_part_tree(ui, ctx, project, asset_id);
                true
            }
            TheEvent::Custom(id, _) if id.name == crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT => {
                self.sync_part_tree(ui, ctx, project, asset_id);
                self.paint_dock.activate(ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
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
    }
}
