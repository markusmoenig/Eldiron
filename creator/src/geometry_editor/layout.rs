//! The modelling viewport owns the central editor; surface materials keep the minimap.
use super::*;
pub const MAIN_STACK: &str = "Geometry Viewport Stack";
const CONTEXT: &str = "Geometry Context Tools";

pub fn main_canvas(normal: TheCanvas) -> TheCanvas {
    let mut stack = TheStackLayout::new(TheId::named(MAIN_STACK));
    stack.add_canvas(normal);
    stack.add_canvas(viewport_canvas());
    let mut canvas = TheCanvas::new();
    canvas.set_layout(stack);
    canvas
}
pub fn is_visible(ui: &mut TheUI) -> bool {
    ui.get_stack_layout(MAIN_STACK)
        .is_some_and(|s| s.index() == 1)
}
pub fn sync_workspace(ui: &mut TheUI, ctx: &mut TheContext) -> bool {
    let page = ui
        .get_stack_layout("Sidebar Page Stack")
        .is_some_and(|s| s.index() == crate::procedural_material::SIDEBAR_PAGE);
    let geometry = page && ui.get_stack_layout(STACK).is_some_and(|s| s.index() == 1);
    let material = page && ui.get_stack_layout(STACK).is_some_and(|s| s.index() == 0);
    let mut changed = false;
    if let Some(s) = ui.get_stack_layout(MAIN_STACK) {
        if s.index() != usize::from(geometry) {
            s.set_index(usize::from(geometry));
            changed = true;
            if let Some(shared) = ui.get_sharedvlayout("Sidebar Map Shared") {
                shared.set_mode(if geometry {
                    TheSharedVLayoutMode::Top
                } else {
                    TheSharedVLayoutMode::Shared
                });
            }
            ctx.ui.clear_focus();
            ctx.ui.keyboard_focus = None;
        }
    }
    if let Some(s) = ui.get_stack_layout(crate::procedural_material::PREVIEW_STACK) {
        let index = usize::from(material);
        if s.index() != index {
            s.set_index(index);
            changed = true;
        }
    }
    if changed {
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }
    changed
}
pub(super) fn set_choice(ui: &mut TheUI, name: &str, index: usize) {
    if let Some(w) = ui.get_widget(name) {
        // Dropdowns preview their hovered choice until mouse-up commits it.
        // Never overwrite that transient selection from the saved editor state.
        if w.state() == TheWidgetState::Clicked {
            return;
        }
        if let Some(m) = w.as_drop_down_menu() {
            if m.selected_index() != index {
                m.set_selected_index(index as i32);
            }
        } else if let Some(b) = w.as_group_button() {
            if b.index() != index as i32 {
                b.set_index(index as i32);
            }
        }
    }
}
fn bar(name: &str) -> TheHLayout {
    let mut b = TheHLayout::new(TheId::named(name));
    b.limiter_mut().set_max_height(36);
    b.set_margin(Vec4::new(8, 4, 8, 4));
    b.set_padding(6);
    b
}
fn button(id: &str, text: &str, icon: &str, status: &str) -> Box<dyn TheWidget> {
    let mut b = TheTraybarButton::new(TheId::named(id));
    b.set_text(text.into());
    if text.is_empty() {
        b.set_icon_name(icon.into());
    }
    b.set_status_text(status);
    Box::new(b)
}
fn name(b: &mut TheHLayout, id: &str) {
    let mut label = TheText::new(TheId::empty());
    label.set_text("Name".into());
    label.limiter_mut().set_max_width(38);
    b.add_widget(Box::new(label));
    let mut edit = TheTextLineEdit::new(TheId::named(id));
    edit.limiter_mut().set_max_width(220);
    b.add_widget(Box::new(edit));
}
fn hint(b: &mut TheHLayout, text: &str) {
    let mut label = TheText::new(TheId::empty());
    label.set_text(text.into());
    label.set_text_size(12.);
    b.add_widget(Box::new(label));
}
pub(super) fn viewport_canvas() -> TheCanvas {
    let mut canvas = TheCanvas::new();
    let mut view = TheRenderView::new(TheId::named(PREVIEW));
    view.set_empty_background(MaterialCanvas);
    view.set_auto_focus(true);
    canvas.set_widget(view);
    let mut header = bar("Geometry Viewport Toolbar");
    let mut mode = TheDropdownMenu::new(TheId::named("MZ Geometry Tool"));
    for text in [
        "View model",
        "Rig / joints",
        "Add joints",
        "Shape profiles",
        "Pose / animate",
        "Attachments",
    ] {
        mode.add_option(text.into());
    }
    mode.limiter_mut().set_max_width(160);
    header.add_widget(Box::new(mode));
    let mut camera = TheDropdownMenu::new(TheId::named("MZ Geometry Camera"));
    for text in [
        "Orbit view",
        "Front view",
        "Side view",
        "Contour view",
        "Isometric tile",
    ] {
        camera.add_option(text.into());
    }
    camera.limiter_mut().set_max_width(130);
    header.add_widget(Box::new(camera));
    let mut target = TheGroupButton::new(TheId::named("MZ Geometry Target"));
    target.add_text_status_icon(String::new(), "Complete model".into(), "mz-output".into());
    target.add_text_status_icon(
        String::new(),
        "Selected operation".into(),
        "mz-selected".into(),
    );
    target.set_item_width(28);
    header.add_widget(Box::new(target));
    header.add_widget(button(
        "MZ Edit Texture Material",
        "Edit material",
        "mz-color",
        "Edit the selected Texture operation's material",
    ));
    let mut rig = TheGroupButton::new(TheId::named("MZ Rig Overlay"));
    rig.add_text_status_icon(
        String::new(),
        "Hide skeleton overlay; editing handles remain visible".into(),
        "mz-geometry".into(),
    );
    rig.add_text_status_icon(
        String::new(),
        "Show skeleton and attachment frames".into(),
        "mz-rig".into(),
    );
    rig.set_item_width(28);
    header.add_widget(Box::new(rig));
    header.add_widget(button(
        "MZ Close Geometry",
        "Back to map",
        "mz-back",
        "Return to the map and material workflow",
    ));
    header.set_reverse_index(Some(1));
    let mut top = TheCanvas::new();
    top.set_layout(header);
    let mut context = TheStackLayout::new(TheId::named(CONTEXT));
    // Playback stays visible independently of the current authoring tool.
    let mut playback = bar("Geometry Playback Toolbar");
    playback.limiter_mut().set_max_height(72);
    let mut clips = TheDropdownMenu::new(TheId::named("MZ Motion"));
    clips.add_option("Rest pose".into());
    clips.limiter_mut().set_max_width(175);
    playback.add_widget(Box::new(clips));
    let mut play = TheGroupButton::new(TheId::named("MZ Motion Play"));
    play.add_text_status_icon(String::new(), "Pause".into(), "mz-pause".into());
    play.add_text_status_icon(String::new(), "Play".into(), "mz-play".into());
    play.set_item_width(28);
    playback.add_widget(Box::new(play));
    let mut phase = TheTextLineEdit::new(TheId::named("MZ Motion Phase"));
    phase.set_range(TheValue::RangeF32(0.0..=1.0));
    phase.set_value(TheValue::Float(0.));
    phase.limiter_mut().set_max_width(100);
    playback.add_widget(Box::new(phase));
    playback.add_widget(button(
        "MZ Add Motion",
        "New clip",
        "mz-add",
        "Create an editable motion clip",
    ));
    hint(&mut playback, "Drag to orbit · pose mode edits joints");
    let mut c = TheCanvas::new();
    c.set_layout(playback);
    let mut playback_canvas = c;
    playback_canvas.set_top(top);
    let mut view_tools = bar("Geometry View Tools");
    hint(
        &mut view_tools,
        "Drag to orbit · pose mode edits animation keys",
    );
    let mut c = TheCanvas::new();
    c.set_layout(view_tools);
    context.add_canvas(c);
    let mut joints = bar("Geometry Skeleton Toolbar");
    joints.add_widget(button(
        "MZ Mirror Branch",
        "Mirror",
        "mz-mirror",
        "Mirror the selected branch across X",
    ));
    joints.add_widget(button(
        "MZ Add Part",
        "Add profile",
        "mz-profile",
        "Add a bone profile; Shift adds a rigid joint detail",
    ));
    joints.add_widget(button(
        "MZ Delete Part",
        "Delete branch",
        "mz-trash",
        "Remove selected branch and its profiles",
    ));
    name(&mut joints, "MZ Joint Name");
    hint(&mut joints, "Drag a joint · add mode creates children");
    let mut c = TheCanvas::new();
    c.set_layout(joints);
    context.add_canvas(c);
    let mut profiles = bar("Geometry Profiles Toolbar");
    profiles.add_widget(button(
        "MZ Add Profile",
        "Add profile",
        "mz-add",
        "Add a profile on the selected bone",
    ));
    profiles.add_widget(button(
        "MZ Delete Profile",
        "Delete profile",
        "mz-trash",
        "Remove the selected profile part",
    ));
    name(&mut profiles, "MZ Profile Name");
    hint(
        &mut profiles,
        "Drag ring handles · right-click adds / removes rings",
    );
    let mut c = TheCanvas::new();
    c.set_layout(profiles);
    context.add_canvas(c);
    let mut attach = bar("Geometry Attachment Toolbar");
    hint(
        &mut attach,
        "Drag a socket or grip to position it · Alt-drag rotates · exact values in Props",
    );
    let mut c = TheCanvas::new();
    c.set_layout(attach);
    context.add_canvas(c);
    context.limiter_mut().set_max_height(108);
    let mut controls = TheCanvas::new();
    controls.limiter_mut().set_max_height(108);
    controls.set_layout(context);
    controls.set_top(playback_canvas);
    canvas.set_top(controls);
    canvas
}
/// Offer Contour only when it can actually edit the selected operation.
pub(super) fn sync_camera_options(editor: &mut GeometryEditor, ui: &mut TheUI) {
    let available = editor.document.as_ref().is_some_and(|doc| {
        doc.rows.iter().flat_map(|r| &r.tiles).any(|t| {
            Some(&t.id) == editor.selected_operation.as_ref()
                && matches!(t.op, Operation::Contour { .. } | Operation::Loft { .. })
        })
    });
    if editor.contour_camera_available == Some(available) {
        return;
    }
    if let Some(w) = ui.get_widget("MZ Geometry Camera") {
        if w.state() == TheWidgetState::Clicked {
            return;
        }
        if let Some(menu) = w.as_drop_down_menu() {
            menu.clear_options();
            for label in ["Orbit view", "Front view", "Side view"] {
                menu.add_option(label.into());
            }
            menu.add_option(
                if available {
                    "Contour view"
                } else {
                    "Contour view (select a profile)"
                }
                .into(),
            );
            menu.add_option("Isometric tile".into());
            if !available && editor.camera == 3 {
                editor.camera = 0;
            }
            menu.set_selected_index(editor.camera as i32);
            editor.contour_camera_available = Some(available);
        }
    }
}
pub(super) fn sync_controls(editor: &GeometryEditor, ui: &mut TheUI, ctx: &mut TheContext) {
    let index = match editor.scene.tool {
        1 | 2 => 1,
        3 => 2,
        5 => 3,
        _ => 0,
    };
    if let Some(s) = ui.get_stack_layout(CONTEXT) {
        if s.index() != index {
            s.set_index(index);
            ctx.ui.relayout = true;
        }
    }
    set_choice(ui, "MZ Geometry Tool", editor.scene.tool);
    set_choice(ui, "MZ Geometry Camera", editor.camera);
    set_choice(
        ui,
        "MZ Geometry Target",
        usize::from(editor.preview_selected),
    );
    let has_rig = editor
        .evaluation
        .as_ref()
        .is_some_and(|e| e.mesh.rig.is_some());
    let joint = editor
        .document
        .as_ref()
        .and_then(scene::armature)
        .and_then(|(_, joints)| joints.get(editor.inspector.joint));
    let has_profiles = editor.document.as_ref().and_then(scene::profiles).is_some();
    let has_part = editor
        .document
        .as_ref()
        .and_then(scene::profiles)
        .is_some_and(|(_, _, parts)| editor.inspector.part < parts.len());
    for (name, enabled) in [
        (
            "MZ Edit Texture Material",
            editor.selected_texture().is_some(),
        ),
        (
            "MZ Use Ruleset Humanoid",
            has_rig
                && editor
                    .evaluation
                    .as_ref()
                    .is_some_and(|e| e.mesh.vertex_count() > 0)
                && editor.selected.is_some_and(|id| {
                    id.to_string() != shared::rulesets::HUMANOID_APPEARANCE_ASSET_ID
                }),
        ),
        ("MZ Add Motion", has_rig),
        ("MZ Motion Play", !editor.playback.clips.is_empty()),
        ("MZ Motion Phase", !editor.playback.clips.is_empty()),
        (
            "MZ Mirror Branch",
            joint.is_some_and(|j| j.parent.is_some()),
        ),
        ("MZ Delete Part", joint.is_some_and(|j| j.parent.is_some())),
        ("MZ Add Part", joint.is_some() && has_profiles),
        ("MZ Add Profile", joint.is_some() && has_profiles),
        ("MZ Delete Profile", has_part),
    ] {
        if let Some(w) = ui.get_widget(name) {
            if w.disabled() == enabled {
                w.set_disabled(!enabled);
            }
        }
    }
    set_choice(ui, "MZ Rig Overlay", usize::from(editor.playback.show_rig));
}
