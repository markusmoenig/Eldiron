use crate::docks::iso_paint::IsoPaintDock;
use crate::docks::palette::PaletteDock;
use crate::docks::tiles::TilesDock;
use crate::editor::{RUSTERIX, SCENEMANAGER, TOOLLIST, UNDOMANAGER};
use crate::prelude::*;
use scenevm::GeoId;

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
const EFFECT_TREE: &str = "Prefab Editor Effect Tree";
const EFFECT_PRESET_STRIP: &str = "Prefab Editor Effect Preset Strip";
const EFFECT_APPLY_PRESET: &str = "Prefab Editor Apply Effect Preset";
const EFFECT_FINISH_PRESET: &str = "Prefab Editor Finish Effect Preset";
const EFFECT_INSPECTOR_PAGES: &str = "Prefab Editor Effect Inspector Pages";
const EFFECT_INSPECTOR_STACK: &str = "Prefab Editor Effect Inspector Stack";
const PARTICLE_EFFECT_ITEM: &str = "Prefab Editor Particle Effect";
const LIGHT_EFFECT_ITEM: &str = "Prefab Editor Light Effect";
const EFFECT_ADD_PARTICLES: &str = "Prefab Editor Add Particles";
const EFFECT_ADD_LIGHT: &str = "Prefab Editor Add Light";
const EFFECT_DUPLICATE: &str = "Prefab Editor Duplicate Effect";
const EFFECT_REMOVE: &str = "Prefab Editor Remove Effect";
const EFFECT_ENABLED: &str = "Prefab Editor Effect Enabled";
const EFFECT_PART: &str = "Prefab Editor Effect Part";
const EFFECT_NAME: &str = "Prefab Editor Effect Name";
const EFFECT_POSITION: &str = "Prefab Editor Effect Position";
const EFFECT_DIRECTION: &str = "Prefab Editor Effect Direction";
const EFFECT_RATE: &str = "Prefab Editor Effect Rate";
const EFFECT_SPREAD: &str = "Prefab Editor Effect Spread";
const EFFECT_LIFETIME: &str = "Prefab Editor Effect Lifetime";
const EFFECT_RADIUS: &str = "Prefab Editor Effect Radius";
const EFFECT_SPEED: &str = "Prefab Editor Effect Speed";
const EFFECT_COLOR: &str = "Prefab Editor Effect Color";
const EFFECT_COLOR_RAMP: &str = "Prefab Editor Effect Color Ramp";
const EFFECT_EMISSION_SHAPE: &str = "Prefab Editor Effect Emission Shape";
const EFFECT_FIT_PART_TOP: &str = "Prefab Editor Effect Fit Part Top";
const EFFECT_EMISSION_WIDTH: &str = "Prefab Editor Effect Emission Width";
const EFFECT_EMISSION_HEIGHT: &str = "Prefab Editor Effect Emission Height";
const EFFECT_EMISSION_DEPTH: &str = "Prefab Editor Effect Emission Depth";
const EFFECT_SIZE_PROFILE: &str = "Prefab Editor Effect Size Profile";
const EFFECT_FADE_PROFILE: &str = "Prefab Editor Effect Fade Profile";
const EFFECT_GRAVITY: &str = "Prefab Editor Effect Gravity";
const EFFECT_TURBULENCE: &str = "Prefab Editor Effect Turbulence";
const EFFECT_INTENSITY: &str = "Prefab Editor Light Intensity";
const EFFECT_RANGE: &str = "Prefab Editor Light Range";
const EFFECT_FLICKER: &str = "Prefab Editor Light Flicker";
const EFFECT_LIGHT_LIFT: &str = "Prefab Editor Light Lift";
const EFFECT_PLACEMENT_MODE: &str = "Prefab Editor Placement Mode";
const EFFECT_SURFACE_OFFSET: &str = "Prefab Editor Surface Offset";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PrefabEditorMode {
    #[default]
    Parts,
    Paint,
    Tiles,
    Palette,
    Effects,
}

struct PrefabEffectDrag {
    effect_id: Uuid,
    direction_handle: bool,
    origin: Vec3<f32>,
    plane_normal: Vec3<f32>,
    before: Box<Project>,
    changed: bool,
}

impl PrefabEditorMode {
    fn index(self) -> i32 {
        match self {
            Self::Parts => 0,
            Self::Paint => 1,
            Self::Tiles => 2,
            Self::Palette => 3,
            Self::Effects => 4,
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
    selected_effect_id: Option<Uuid>,
    effect_inspector_page: usize,
    effect_preset_applying: bool,
    effect_drag: Option<PrefabEffectDrag>,
    effect_part_options: Vec<Uuid>,
    parent_options: Vec<Option<Uuid>>,
    assignment_options: Vec<Uuid>,
    door_preview_open: bool,
    paint_dock: IsoPaintDock,
    tiles_dock: TilesDock,
    palette_dock: PaletteDock,
}

impl PrefabsEditorDock {
    fn effect_id_for_gizmo(asset: &rusterix::BlockPropAsset, gizmo: u32) -> Option<(Uuid, bool)> {
        let (index, direction) = if (0xEFFE_0000..0xEFFF_0000).contains(&gizmo) {
            ((gizmo - 0xEFFE_0000) as usize, false)
        } else if gizmo >= 0xEFFF_0000 {
            ((gizmo - 0xEFFF_0000) as usize, true)
        } else {
            return None;
        };
        let id = asset
            .particle_effects
            .iter()
            .map(|effect| effect.id)
            .chain(asset.light_effects.iter().map(|effect| effect.id))
            .nth(index)?;
        Some((id, direction))
    }

    fn effect_attachment_ref(
        asset: &rusterix::BlockPropAsset,
        effect_id: Uuid,
    ) -> Option<(Uuid, Uuid)> {
        asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .map(|effect| (effect.part_id, effect.attachment_id))
            .or_else(|| {
                asset
                    .light_effects
                    .iter()
                    .find(|effect| effect.id == effect_id)
                    .map(|effect| (effect.part_id, effect.attachment_id))
            })
    }

    fn effect_attachment_mut(
        asset: &mut rusterix::BlockPropAsset,
        effect_id: Uuid,
    ) -> Option<&mut rusterix::BlockPropAttachment> {
        let (part_id, attachment_id) = Self::effect_attachment_ref(asset, effect_id)?;
        asset
            .parts
            .iter_mut()
            .find(|part| part.id == part_id)?
            .attachments
            .iter_mut()
            .find(|attachment| attachment.id == attachment_id)
    }

    fn ray_drag_point(
        server_ctx: &ServerContext,
        origin: Vec3<f32>,
        normal: Vec3<f32>,
    ) -> Option<Vec3<f32>> {
        let ray_origin = server_ctx.hover_ray_origin_3d?;
        let ray_direction = server_ctx.hover_ray_dir_3d?;
        let denominator = ray_direction.dot(normal);
        if denominator.abs() <= 1e-6 {
            return None;
        }
        let t = (origin - ray_origin).dot(normal) / denominator;
        (t >= 0.0).then_some(ray_origin + ray_direction * t)
    }

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

    fn effects_canvas() -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::new();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar = TheHLayout::new(TheId::named("Prefab Effect Actions"));
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(6, 2, 6, 2));
        toolbar.set_padding(5);
        for (id, label) in [
            (EFFECT_ADD_PARTICLES, "Add Particles"),
            (EFFECT_ADD_LIGHT, "Add Light"),
            (EFFECT_DUPLICATE, "Duplicate"),
            (EFFECT_REMOVE, "Remove"),
        ] {
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(label.to_string());
            button.set_fixed_size(false);
            toolbar.add_widget(Box::new(button));
        }
        toolbar_canvas.set_layout(toolbar);

        let mut tree_canvas = TheCanvas::new();
        tree_canvas.set_layout(TheTreeLayout::new(TheId::named(EFFECT_TREE)));

        let configure_layout = |layout: &mut TheTextLayout| {
            layout.set_margin(Vec4::new(8, 8, 8, 8));
            layout.set_padding(7);
            layout.set_text_margin(8);
            layout.set_fixed_text_width(100);
            layout.set_text_align(TheHorizontalAlign::Right);
        };
        let add_edit = |layout: &mut TheTextLayout, id: &str, label: &str, status: &str| {
            let mut edit = TheTextLineEdit::new(TheId::named(id));
            edit.limiter_mut().set_max_width(i32::MAX);
            edit.set_status_text(status);
            layout.add_pair(label.to_string(), Box::new(edit));
        };
        let add_number = |layout: &mut TheTextLayout,
                          id: &str,
                          label: &str,
                          status: &str,
                          range: std::ops::RangeInclusive<f32>| {
            let mut edit = TheTextLineEdit::new(TheId::named(id));
            edit.limiter_mut().set_max_width(i32::MAX);
            edit.set_range(TheValue::RangeF32(range));
            edit.set_status_text(status);
            layout.add_pair(label.to_string(), Box::new(edit));
        };

        let mut setup_canvas = TheCanvas::new();
        let mut setup = TheTextLayout::new(TheId::named("Prefab Effect Setup Inspector"));
        configure_layout(&mut setup);
        let mut placement = TheGroupButton::new(TheId::named(EFFECT_PLACEMENT_MODE));
        for label in ["Ground", "Wall", "Surface", "Free"] {
            placement.add_text_status(label.to_string(), format!("Mount on {label}"));
        }
        placement.set_index(0);
        setup.add_pair("Mount".to_string(), Box::new(placement));
        add_number(
            &mut setup,
            EFFECT_SURFACE_OFFSET,
            "Surface offset",
            "Distance from the mounting surface",
            0.0..=5.0,
        );

        let mut enabled = TheCheckButton::new(TheId::named(EFFECT_ENABLED));
        enabled.set_status_text("Temporarily enable or disable the selected effect");
        setup.add_pair("Enabled".to_string(), Box::new(enabled));

        let mut part = TheDropdownMenu::new(TheId::named(EFFECT_PART));
        part.limiter_mut().set_max_width(i32::MAX);
        part.set_status_text("Prefab part which owns this effect attachment");
        setup.add_pair("Attach to".to_string(), Box::new(part));
        add_edit(
            &mut setup,
            EFFECT_NAME,
            "Name",
            "Name shown in the effect list",
        );
        add_edit(
            &mut setup,
            EFFECT_POSITION,
            "Position",
            "Local attachment position: x, y, z",
        );
        add_edit(
            &mut setup,
            EFFECT_DIRECTION,
            "Direction",
            "Emission direction: x, y, z",
        );

        let mut color_ramp = ThePaletteIndexRowPicker::new(TheId::named(EFFECT_COLOR_RAMP));
        color_ramp.set_selected_indices(vec![0, 0, 0, 0]);
        color_ramp.set_status_text("Choose four particle colors from birth to fade-out");
        setup.add_pair("Colors".to_string(), Box::new(color_ramp));
        setup_canvas.set_layout(setup);

        let mut emission_canvas = TheCanvas::new();
        let mut emission = TheTextLayout::new(TheId::named("Prefab Effect Emission Inspector"));
        configure_layout(&mut emission);
        let mut emission_shape = TheGroupButton::new(TheId::named(EFFECT_EMISSION_SHAPE));
        for (label, status) in [
            ("Point", "Emit at the attachment origin"),
            ("Box", "Emit throughout a box around the attachment"),
            ("Surface", "Emit across a horizontal rectangular surface"),
        ] {
            emission_shape.add_text_status(label.to_string(), status.to_string());
        }
        emission_shape.set_index(0);
        emission.add_pair("Emit from".to_string(), Box::new(emission_shape));
        let mut fit_part = TheTraybarButton::new(TheId::named(EFFECT_FIT_PART_TOP));
        fit_part.set_text("Fit attached part top".to_string());
        fit_part.set_fixed_size(false);
        fit_part.limiter_mut().set_max_width(i32::MAX);
        fit_part.set_status_text("Cover the complete top surface of the attached Prefab part");
        emission.add_pair("Bounds".to_string(), Box::new(fit_part));
        for (id, label, status, range) in [
            (
                EFFECT_RATE,
                "Amount",
                "Particles emitted per second",
                0.0..=200.0,
            ),
            (
                EFFECT_EMISSION_WIDTH,
                "Width",
                "Full width of the emission region",
                0.0..=20.0,
            ),
            (
                EFFECT_EMISSION_HEIGHT,
                "Height",
                "Full height of a box emission region",
                0.0..=20.0,
            ),
            (
                EFFECT_EMISSION_DEPTH,
                "Depth",
                "Full depth of the emission region",
                0.0..=20.0,
            ),
            (
                EFFECT_SPREAD,
                "Spread",
                "How widely particles fan out",
                0.0..=std::f32::consts::PI,
            ),
            (
                EFFECT_SPEED,
                "Speed",
                "Average particle movement speed",
                0.0..=10.0,
            ),
            (
                EFFECT_TURBULENCE,
                "Random motion",
                "Random motion strength",
                0.0..=5.0,
            ),
        ] {
            add_number(&mut emission, id, label, status, range);
        }
        add_edit(
            &mut emission,
            EFFECT_GRAVITY,
            "Gravity",
            "World acceleration: x, y, z",
        );
        emission_canvas.set_layout(emission);

        let mut lifetime_canvas = TheCanvas::new();
        let mut lifetime = TheTextLayout::new(TheId::named("Prefab Effect Lifetime Inspector"));
        configure_layout(&mut lifetime);
        for (id, label, status, range) in [
            (
                EFFECT_LIFETIME,
                "Duration",
                "Average particle lifetime in seconds",
                0.01..=20.0,
            ),
            (
                EFFECT_RADIUS,
                "Particle size",
                "Average particle size in world units",
                0.001..=2.0,
            ),
        ] {
            add_number(&mut lifetime, id, label, status, range);
        }
        let mut size_profile = TheGroupButton::new(TheId::named(EFFECT_SIZE_PROFILE));
        for (label, status) in [
            ("Grow", "Particles expand over their lifetime"),
            ("Steady", "Particles keep roughly the same size"),
            ("Shrink", "Particles become smaller over their lifetime"),
        ] {
            size_profile.add_text_status(label.to_string(), status.to_string());
        }
        lifetime.add_pair("Size behavior".to_string(), Box::new(size_profile));
        let mut fade_profile = TheGroupButton::new(TheId::named(EFFECT_FADE_PROFILE));
        for (label, status) in [
            ("Soft", "Fade in and out smoothly"),
            ("Late", "Stay visible, then fade near the end"),
            ("None", "Keep opacity throughout the lifetime"),
        ] {
            fade_profile.add_text_status(label.to_string(), status.to_string());
        }
        lifetime.add_pair("Fade".to_string(), Box::new(fade_profile));
        lifetime_canvas.set_layout(lifetime);

        let mut light_canvas = TheCanvas::new();
        let mut light = TheTextLayout::new(TheId::named("Prefab Effect Light Inspector"));
        configure_layout(&mut light);
        let mut light_color = ThePaletteIndexPicker::new(TheId::named(EFFECT_COLOR));
        light_color.set_status_text("Choose the emitted light color from the project palette");
        light.add_pair("Color".to_string(), Box::new(light_color));
        for (id, label, status, range) in [
            (
                EFFECT_INTENSITY,
                "Brightness",
                "Light intensity",
                0.0..=20.0,
            ),
            (
                EFFECT_RANGE,
                "Range",
                "Light range in world units",
                0.0..=50.0,
            ),
            (
                EFFECT_FLICKER,
                "Flicker",
                "Random light intensity variation",
                0.0..=1.0,
            ),
            (
                EFFECT_LIGHT_LIFT,
                "Height",
                "Vertical offset from the attachment",
                -5.0..=5.0,
            ),
        ] {
            add_number(&mut light, id, label, status, range);
        }
        light_canvas.set_layout(light);

        let mut inspector_stack = TheStackLayout::new(TheId::named(EFFECT_INSPECTOR_STACK));
        inspector_stack.add_canvas(setup_canvas);
        inspector_stack.add_canvas(emission_canvas);
        inspector_stack.add_canvas(lifetime_canvas);
        inspector_stack.add_canvas(light_canvas);
        let mut inspector = TheCanvas::new();
        inspector.set_layout(inspector_stack);

        let mut pages_canvas = TheCanvas::new();
        pages_canvas.limiter_mut().set_min_height(28);
        pages_canvas.limiter_mut().set_max_height(28);
        let mut pages_layout = TheHLayout::new(TheId::named("Prefab Effect Inspector Tabs"));
        pages_layout.limiter_mut().set_min_height(28);
        pages_layout.limiter_mut().set_max_height(28);
        pages_layout.set_margin(Vec4::new(8, 3, 8, 2));
        let mut pages = TheGroupButton::new(TheId::named(EFFECT_INSPECTOR_PAGES));
        for (label, status) in [
            ("Setup", "Attachment and particle colors"),
            ("Emission", "Where particles are born and how they move"),
            ("Lifetime", "Particle size and fading"),
            ("Light", "Light emission settings"),
        ] {
            pages.add_text_status(label.to_string(), status.to_string());
        }
        pages.set_index(0);
        pages_layout.add_widget(Box::new(pages));
        pages_canvas.set_layout(pages_layout);

        let mut inspector_panel = TheCanvas::new();
        inspector_panel.set_top(pages_canvas);
        inspector_panel.set_center(inspector);

        let mut split = TheSharedHLayout::new(TheId::named("Prefab Effects Shared HLayout"));
        split.set_shared_ratio(0.25);
        split.set_mode(TheSharedHLayoutMode::Shared);
        split.add_canvas(tree_canvas);
        split.add_canvas(inspector_panel);

        let mut preset_canvas = TheCanvas::new();
        preset_canvas.limiter_mut().set_min_height(50);
        preset_canvas.limiter_mut().set_max_height(50);
        let mut presets = TheScrollableIconRow::new(TheId::named(EFFECT_PRESET_STRIP));
        presets.set_tile_width(82);
        presets.set_icon_padding(3);
        presets.set_show_labels(true);
        presets.limiter_mut().set_min_height(58);
        presets.limiter_mut().set_max_height(58);
        presets.set_items(Self::effect_preset_items());
        preset_canvas.set_widget(presets);

        let mut content = TheCanvas::new();
        let mut split_canvas = TheCanvas::new();
        split_canvas.set_layout(split);
        content.set_top(preset_canvas);
        content.set_center(split_canvas);
        canvas.set_top(toolbar_canvas);
        canvas.set_center(content);
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
        if let Some(widget) = ui.get_widget(EFFECT_PLACEMENT_MODE)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(match asset.placement.mode {
                rusterix::BlockPropPlacementMode::Ground => 0,
                rusterix::BlockPropPlacementMode::Wall => 1,
                rusterix::BlockPropPlacementMode::AnySurface => 2,
                rusterix::BlockPropPlacementMode::Free => 3,
            });
        }
        ui.set_widget_value(
            EFFECT_SURFACE_OFFSET,
            ctx,
            TheValue::Text(format!("{:.3}", asset.placement.surface_offset)),
        );
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

    fn parse_vec3(value: &str) -> Option<[f32; 3]> {
        let values = value
            .split([',', ' '])
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().parse::<f32>())
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (values.len() == 3).then(|| [values[0], values[1], values[2]])
    }

    fn effect_range_around(center: f32, variation: f32) -> (f32, f32) {
        let center = center.max(0.0);
        let variation = variation.clamp(0.0, 0.95);
        (center * (1.0 - variation), center * (1.0 + variation))
    }

    fn effect_palette_index(project: &Project, color: [u8; 4]) -> i32 {
        project
            .art_palette
            .find_closest_color_index(&TheColor::from(color))
            .unwrap_or(project.art_palette.current_index as usize) as i32
    }

    fn effect_palette_color(project: &Project, index: u16) -> Option<[u8; 4]> {
        project
            .art_palette
            .colors
            .get(index as usize)
            .and_then(|color| color.as_ref())
            .map(TheColor::to_u8_array)
    }

    fn effect_color_ramp_slot(id: &str) -> Option<usize> {
        id.strip_prefix(EFFECT_COLOR_RAMP)?
            .trim()
            .parse::<usize>()
            .ok()?
            .checked_sub(1)
            .filter(|slot| *slot < 4)
    }

    fn fit_particle_emission_to_part_top(
        asset: &mut rusterix::BlockPropAsset,
        effect_id: Uuid,
    ) -> bool {
        let Some((part_id, attachment_id)) = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .map(|effect| (effect.part_id, effect.attachment_id))
        else {
            return false;
        };

        let Some(part) = asset.find_part(part_id) else {
            return false;
        };
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        let mut found = false;
        for object in part.geometry_source.geometry_objects() {
            for vertex in &object.vertices {
                let point = object.transform_point(*vertex);
                min = min.map2(point, f32::min);
                max = max.map2(point, f32::max);
                found = true;
            }
        }
        if !found {
            return false;
        }

        let center = (min + max) * 0.5;
        let half = ((max - min) * 0.5).map(|value| value.max(0.0));
        let position = [center.x, max.y, center.z];
        let spawn_area = [half.x, 0.0, half.z];
        let mut changed = false;
        if let Some(effect) = asset
            .particle_effects
            .iter_mut()
            .find(|effect| effect.id == effect_id)
        {
            changed |= effect.emitter.emission_shape != rusterix::ParticleEmissionShape::Surface;
            changed |= effect.emitter.spawn_area != spawn_area;
            effect.emitter.emission_shape = rusterix::ParticleEmissionShape::Surface;
            effect.emitter.spawn_area = spawn_area;
        }
        if let Some(attachment) = asset
            .parts
            .iter_mut()
            .find(|part| part.id == part_id)
            .and_then(|part| {
                part.attachments
                    .iter_mut()
                    .find(|attachment| attachment.id == attachment_id)
            })
        {
            changed |= attachment.position != position;
            attachment.position = position;
            attachment.direction = [0.0, 1.0, 0.0];
        }
        changed
    }

    const PARTICLE_PRESETS: [&'static str; 7] =
        ["Flame", "Smoke", "Vapor", "Fog", "Sparks", "Embers", "Dust"];

    fn particle_preset_definition(preset: &str) -> (&'static str, rusterix::ParticleEmitterDef) {
        if preset == "Flame" {
            return ("Flame", crate::blocks::stonefall_torch_flame_emitter());
        }
        if preset == "Smoke" {
            return ("Smoke", crate::blocks::smoke_emitter(9.0, 1.0));
        }
        let mut emitter = rusterix::ParticleEmitterDef::default();
        let (name, color) = match preset {
            "Fog" => {
                emitter.rate = 14.0;
                emitter.spread = 1.15;
                emitter.lifetime_range = (2.0, 4.5);
                emitter.radius_range = (0.035, 0.11);
                emitter.speed_range = (0.03, 0.16);
                emitter.spawn_area = [0.35, 0.03, 0.35];
                emitter.emission_shape = rusterix::ParticleEmissionShape::Surface;
                emitter.size_curve = [0.75, 1.1, 1.55, 2.0];
                emitter.opacity_curve = [0.0, 0.52, 0.32, 0.0];
                emitter.gravity = [0.0, 0.025, 0.0];
                emitter.turbulence = 0.08;
                ("Fog", [170, 184, 190, 125])
            }
            "Vapor" => {
                emitter.rate = 18.0;
                emitter.spread = 0.24;
                emitter.lifetime_range = (0.8, 2.2);
                emitter.radius_range = (0.012, 0.045);
                emitter.speed_range = (0.1, 0.36);
                emitter.spawn_area = [0.14, 0.015, 0.14];
                emitter.emission_shape = rusterix::ParticleEmissionShape::Surface;
                emitter.size_curve = [0.45, 0.85, 1.3, 1.65];
                emitter.opacity_curve = [0.0, 0.7, 0.38, 0.0];
                emitter.gravity = [0.0, 0.1, 0.0];
                emitter.turbulence = 0.16;
                ("Vapor", [188, 205, 211, 150])
            }
            "Sparks" => {
                emitter.rate = 12.0;
                emitter.spread = 0.85;
                emitter.lifetime_range = (0.25, 0.9);
                emitter.radius_range = (0.003, 0.012);
                emitter.speed_range = (0.5, 1.8);
                emitter.spawn_area = [0.04, 0.02, 0.04];
                emitter.size_curve = [1.0, 0.8, 0.4, 0.05];
                emitter.opacity_curve = [1.0, 1.0, 0.72, 0.0];
                emitter.gravity = [0.0, -1.8, 0.0];
                emitter.turbulence = 0.05;
                ("Sparks", [255, 176, 52, 255])
            }
            "Embers" => {
                emitter.rate = 5.0;
                emitter.spread = 0.75;
                emitter.lifetime_range = (0.65, 2.0);
                emitter.radius_range = (0.002, 0.008);
                emitter.speed_range = (0.18, 0.65);
                emitter.spawn_area = [0.12, 0.02, 0.12];
                emitter.emission_shape = rusterix::ParticleEmissionShape::Surface;
                emitter.size_curve = [1.0, 0.9, 0.55, 0.05];
                emitter.opacity_curve = [1.0, 1.0, 0.75, 0.0];
                emitter.gravity = [0.0, 0.05, 0.0];
                emitter.turbulence = 0.18;
                ("Embers", [255, 112, 30, 255])
            }
            "Dust" => {
                emitter.rate = 7.0;
                emitter.spread = 1.3;
                emitter.lifetime_range = (1.8, 4.0);
                emitter.radius_range = (0.004, 0.018);
                emitter.speed_range = (0.015, 0.09);
                emitter.spawn_area = [0.4, 0.18, 0.4];
                emitter.emission_shape = rusterix::ParticleEmissionShape::Box;
                emitter.size_curve = [0.75, 1.0, 1.0, 0.7];
                emitter.opacity_curve = [0.0, 0.58, 0.45, 0.0];
                emitter.gravity = [0.0, -0.008, 0.0];
                emitter.turbulence = 0.1;
                ("Dust", [182, 158, 119, 135])
            }
            _ => return ("Flame", crate::blocks::stonefall_torch_flame_emitter()),
        };
        emitter.color = color;
        emitter.color_ramp = Some(if name == "Flame" || name == "Sparks" || name == "Embers" {
            [
                [255, 242, 168, 255],
                [255, 193, 79, 255],
                [240, 100, 31, 255],
                [64, 16, 8, 0],
            ]
        } else {
            [
                color,
                color,
                [color[0] / 2, color[1] / 2, color[2] / 2, color[3]],
                [0, 0, 0, 0],
            ]
        });
        (name, emitter)
    }

    fn effect_preset_items() -> Vec<TheScrollableIconRowItem> {
        Self::PARTICLE_PRESETS
            .iter()
            .map(|preset| {
                let (name, _) = Self::particle_preset_definition(preset);
                let mut item = TheScrollableIconRowItem::new(name);
                item.status = format!("Apply the {name} preset to the selected particles");
                item.icon = Some(Self::effect_preset_icon(name));
                item
            })
            .collect()
    }

    fn effect_preset_icon(preset: &str) -> TheRGBABuffer {
        let mut icon = TheRGBABuffer::new(TheDim::sized(48, 28));
        match preset {
            "Flame" => {
                Self::draw_effect_icon_disc(&mut icon, 16, 8, 16, [239, 80, 26, 255]);
                Self::draw_effect_icon_disc(&mut icon, 19, 11, 10, [255, 191, 54, 255]);
                Self::draw_effect_icon_disc(&mut icon, 22, 15, 5, [255, 242, 172, 255]);
            }
            "Smoke" => {
                Self::draw_effect_icon_disc(&mut icon, 10, 13, 11, [82, 88, 96, 220]);
                Self::draw_effect_icon_disc(&mut icon, 18, 8, 14, [111, 117, 125, 210]);
                Self::draw_effect_icon_disc(&mut icon, 29, 12, 10, [76, 81, 89, 190]);
            }
            "Vapor" => {
                for x in [14, 23, 32] {
                    icon.draw_line(x, 23, x - 2, 15, [174, 207, 218, 220]);
                    icon.draw_line(x - 2, 15, x + 2, 8, [205, 228, 234, 180]);
                }
            }
            "Fog" => {
                icon.draw_horizontal_line(7, 39, 10, [154, 174, 184, 180]);
                icon.draw_horizontal_line(12, 43, 15, [190, 205, 211, 205]);
                icon.draw_horizontal_line(5, 34, 20, [132, 151, 162, 160]);
            }
            "Sparks" => {
                for (x, y) in [(12, 19), (20, 9), (29, 17), (37, 7)] {
                    icon.draw_line(x - 2, y, x + 2, y, [255, 185, 54, 255]);
                    icon.draw_line(x, y - 2, x, y + 2, [255, 232, 128, 255]);
                }
            }
            "Embers" => {
                for (x, y, size) in [(9, 17, 4), (17, 9, 3), (25, 18, 5), (35, 11, 3)] {
                    Self::draw_effect_icon_disc(&mut icon, x, y, size, [255, 104, 28, 255]);
                }
            }
            _ => {
                for (x, y, size) in [
                    (7, 9, 3),
                    (14, 18, 2),
                    (21, 12, 3),
                    (28, 21, 2),
                    (35, 8, 2),
                    (40, 16, 3),
                ] {
                    Self::draw_effect_icon_disc(&mut icon, x, y, size, [191, 166, 125, 220]);
                }
            }
        }
        icon
    }

    fn draw_effect_icon_disc(icon: &mut TheRGBABuffer, x: i32, y: i32, size: i32, color: [u8; 4]) {
        icon.draw_disc(&TheDim::new(x, y, size, size), &color, 0.0, &[0, 0, 0, 0]);
    }

    fn add_particle_preset(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
        preset: &str,
    ) -> Result<Uuid, String> {
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or_else(|| "Prefab is missing".to_string())?;
        let part_id = self
            .selected_part_id
            .filter(|part_id| asset.find_part(*part_id).is_some())
            .or_else(|| asset.parts.first().map(|part| part.id))
            .ok_or_else(|| "Create a Prefab part before adding an effect".to_string())?;
        let part = asset
            .parts
            .iter_mut()
            .find(|part| part.id == part_id)
            .expect("validated part");
        let attachment_id = Uuid::new_v4();
        part.attachments.push(rusterix::BlockPropAttachment {
            id: attachment_id,
            name: format!("{preset} Emitter"),
            position: part.pivot,
            direction: [0.0, 1.0, 0.0],
            up: [0.0, 0.0, 1.0],
        });

        let (name, emitter) = Self::particle_preset_definition(preset);
        let effect_id = Uuid::new_v4();
        asset
            .particle_effects
            .push(rusterix::BlockPropParticleEffect {
                id: effect_id,
                name: name.to_string(),
                part_id,
                attachment_id,
                enabled: true,
                emitter,
            });
        self.selected_part_id = Some(part_id);
        self.selected_effect_id = Some(effect_id);
        Ok(effect_id)
    }

    fn apply_particle_preset(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
        preset: &str,
    ) -> Result<(), String> {
        let effect_id = self
            .selected_effect_id
            .ok_or_else(|| "Select a particle effect first".to_string())?;
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or_else(|| "Prefab is missing".to_string())?;
        let effect_index = asset
            .particle_effects
            .iter()
            .position(|effect| effect.id == effect_id)
            .ok_or_else(|| "Select a particle effect first".to_string())?;
        let (name, mut emitter) = Self::particle_preset_definition(preset);
        let (part_id, attachment_id) = {
            let effect = &mut asset.particle_effects[effect_index];
            // Presets are absolute visual definitions. The source footprint is
            // layout authored on the Prefab, so it remains independent of the
            // selected visual style.
            emitter.emission_shape = effect.emitter.emission_shape;
            emitter.spawn_area = effect.emitter.spawn_area;

            effect.name = name.to_string();
            effect.emitter = emitter;
            (effect.part_id, effect.attachment_id)
        };
        if let Some(attachment) = asset
            .parts
            .iter_mut()
            .find(|part| part.id == part_id)
            .and_then(|part| {
                part.attachments
                    .iter_mut()
                    .find(|attachment| attachment.id == attachment_id)
            })
        {
            attachment.name = format!("{name} Emitter");
        }
        Ok(())
    }

    fn add_light_effect(&mut self, project: &mut Project, asset_id: Uuid) -> Result<Uuid, String> {
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or_else(|| "Prefab is missing".to_string())?;
        let linked = self.selected_effect_id.and_then(|effect_id| {
            asset
                .particle_effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .map(|effect| (effect.part_id, effect.attachment_id))
        });
        let (part_id, attachment_id) = if let Some(linked) = linked {
            linked
        } else {
            let part_id = self
                .selected_part_id
                .filter(|part_id| asset.find_part(*part_id).is_some())
                .or_else(|| asset.parts.first().map(|part| part.id))
                .ok_or_else(|| "Create a Prefab part before adding a light".to_string())?;
            let part = asset
                .parts
                .iter_mut()
                .find(|part| part.id == part_id)
                .expect("validated part");
            let attachment_id = Uuid::new_v4();
            part.attachments.push(rusterix::BlockPropAttachment {
                id: attachment_id,
                name: "Light".to_string(),
                position: part.pivot,
                direction: [0.0, 1.0, 0.0],
                up: [0.0, 0.0, 1.0],
            });
            (part_id, attachment_id)
        };
        let id = Uuid::new_v4();
        asset.light_effects.push(rusterix::BlockPropLightEffect {
            id,
            name: "Fire Light".to_string(),
            part_id,
            attachment_id,
            enabled: true,
            color: [255, 154, 69, 255],
            intensity: 2.0,
            range: 4.0,
            flicker: 0.2,
            lift: 0.04,
        });
        self.selected_part_id = Some(part_id);
        self.selected_effect_id = Some(id);
        Ok(id)
    }

    fn duplicate_selected_effect(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
    ) -> Result<Uuid, String> {
        let effect_id = self
            .selected_effect_id
            .ok_or_else(|| "Select an effect to duplicate".to_string())?;
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or_else(|| "Prefab is missing".to_string())?;

        if let Some(mut effect) = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .cloned()
        {
            let mut attachment = asset
                .find_part(effect.part_id)
                .and_then(|part| {
                    part.attachments
                        .iter()
                        .find(|attachment| attachment.id == effect.attachment_id)
                })
                .cloned()
                .ok_or_else(|| "The effect attachment is missing".to_string())?;
            attachment.id = Uuid::new_v4();
            attachment.name = format!("{} Copy", attachment.name);
            let attachment_id = attachment.id;
            asset
                .parts
                .iter_mut()
                .find(|part| part.id == effect.part_id)
                .expect("validated effect part")
                .attachments
                .push(attachment);
            effect.id = Uuid::new_v4();
            effect.attachment_id = attachment_id;
            effect.name = format!("{} Copy", effect.name);
            let id = effect.id;
            asset.particle_effects.push(effect);
            self.selected_effect_id = Some(id);
            return Ok(id);
        }

        if let Some(mut effect) = asset
            .light_effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .cloned()
        {
            let mut attachment = asset
                .find_part(effect.part_id)
                .and_then(|part| {
                    part.attachments
                        .iter()
                        .find(|attachment| attachment.id == effect.attachment_id)
                })
                .cloned()
                .ok_or_else(|| "The light attachment is missing".to_string())?;
            attachment.id = Uuid::new_v4();
            attachment.name = format!("{} Copy", attachment.name);
            let attachment_id = attachment.id;
            asset
                .parts
                .iter_mut()
                .find(|part| part.id == effect.part_id)
                .expect("validated light part")
                .attachments
                .push(attachment);
            effect.id = Uuid::new_v4();
            effect.attachment_id = attachment_id;
            effect.name = format!("{} Copy", effect.name);
            let id = effect.id;
            asset.light_effects.push(effect);
            self.selected_effect_id = Some(id);
            return Ok(id);
        }

        Err("The selected effect is missing".to_string())
    }

    fn move_selected_effect_to_part(
        &mut self,
        project: &mut Project,
        asset_id: Uuid,
        target_part_id: Uuid,
    ) -> Result<bool, String> {
        let effect_id = self
            .selected_effect_id
            .ok_or_else(|| "Select an effect first".to_string())?;
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or_else(|| "Prefab is missing".to_string())?;
        if asset.find_part(target_part_id).is_none() {
            return Err("The target part is missing".to_string());
        }
        let source = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .map(|effect| (effect.part_id, effect.attachment_id, true))
            .or_else(|| {
                asset
                    .light_effects
                    .iter()
                    .find(|effect| effect.id == effect_id)
                    .map(|effect| (effect.part_id, effect.attachment_id, false))
            })
            .ok_or_else(|| "The selected effect is missing".to_string())?;
        if source.0 == target_part_id {
            return Ok(false);
        }

        let mut attachment = asset
            .find_part(source.0)
            .and_then(|part| {
                part.attachments
                    .iter()
                    .find(|attachment| attachment.id == source.1)
            })
            .cloned()
            .ok_or_else(|| "The effect attachment is missing".to_string())?;
        attachment.id = Uuid::new_v4();
        let new_attachment_id = attachment.id;
        asset
            .parts
            .iter_mut()
            .find(|part| part.id == target_part_id)
            .expect("validated target part")
            .attachments
            .push(attachment);

        if source.2 {
            let effect = asset
                .particle_effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
                .expect("validated particle effect");
            effect.part_id = target_part_id;
            effect.attachment_id = new_attachment_id;
        } else {
            let effect = asset
                .light_effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
                .expect("validated light effect");
            effect.part_id = target_part_id;
            effect.attachment_id = new_attachment_id;
        }

        let old_attachment_still_used = asset
            .particle_effects
            .iter()
            .any(|effect| effect.part_id == source.0 && effect.attachment_id == source.1)
            || asset
                .light_effects
                .iter()
                .any(|effect| effect.part_id == source.0 && effect.attachment_id == source.1);
        if !old_attachment_still_used
            && let Some(part) = asset.parts.iter_mut().find(|part| part.id == source.0)
        {
            part.attachments
                .retain(|attachment| attachment.id != source.1);
        }
        self.selected_part_id = Some(target_part_id);
        Ok(true)
    }

    fn sync_effect_editor(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        asset_id: Uuid,
    ) {
        let Some(asset) = project.block_props.get(&asset_id) else {
            return;
        };
        if let Some(widget) = ui.get_widget(EFFECT_INSPECTOR_PAGES)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(self.effect_inspector_page as i32);
        }
        if let Some(stack) = ui.get_stack_layout(EFFECT_INSPECTOR_STACK) {
            stack.set_index(self.effect_inspector_page);
        }
        if let Some(widget) = ui.get_widget(EFFECT_PLACEMENT_MODE)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(match asset.placement.mode {
                rusterix::BlockPropPlacementMode::Ground => 0,
                rusterix::BlockPropPlacementMode::Wall => 1,
                rusterix::BlockPropPlacementMode::AnySurface => 2,
                rusterix::BlockPropPlacementMode::Free => 3,
            });
        }
        ui.set_widget_value(
            EFFECT_SURFACE_OFFSET,
            ctx,
            TheValue::Text(format!("{:.3}", asset.placement.surface_offset)),
        );
        if self.selected_effect_id.is_none_or(|id| {
            asset.particle_effects.iter().all(|effect| effect.id != id)
                && asset.light_effects.iter().all(|effect| effect.id != id)
        }) {
            self.selected_effect_id = asset
                .particle_effects
                .first()
                .map(|effect| effect.id)
                .or_else(|| asset.light_effects.first().map(|effect| effect.id));
        }
        if let Some(tree) = ui.get_tree_layout(EFFECT_TREE) {
            let root = tree.get_root();
            root.childs.clear();
            root.widgets.clear();
            let mut asset_node = TheTreeNode::new(TheId::named_with_id(&asset.name, asset.id));
            asset_node.set_open(true);
            for effect in &asset.particle_effects {
                let mut item =
                    TheTreeItem::new(TheId::named_with_id(PARTICLE_EFFECT_ITEM, effect.id));
                item.set_text(format!(
                    "Particles: {}{}",
                    effect.name,
                    if effect.enabled { "" } else { " (off)" }
                ));
                asset_node.add_widget(Box::new(item));
            }
            for effect in &asset.light_effects {
                let mut item = TheTreeItem::new(TheId::named_with_id(LIGHT_EFFECT_ITEM, effect.id));
                item.set_text(format!(
                    "Light: {}{}",
                    effect.name,
                    if effect.enabled { "" } else { " (off)" }
                ));
                asset_node.add_widget(Box::new(item));
            }
            root.add_child(asset_node);
            if let Some(id) = self.selected_effect_id {
                let name = if asset.particle_effects.iter().any(|effect| effect.id == id) {
                    PARTICLE_EFFECT_ITEM
                } else {
                    LIGHT_EFFECT_ITEM
                };
                tree.new_item_selected(TheId::named_with_id(name, id));
            }
        }

        let particle = self
            .selected_effect_id
            .and_then(|id| asset.particle_effects.iter().find(|effect| effect.id == id));
        let light = self
            .selected_effect_id
            .and_then(|id| asset.light_effects.iter().find(|effect| effect.id == id));
        if let Some(effect) = particle
            && let Some(index) = Self::PARTICLE_PRESETS
                .iter()
                .position(|preset| *preset == effect.name)
            && let Some(widget) = ui.get_widget(EFFECT_PRESET_STRIP)
            && let Some(strip) = widget.as_any().downcast_mut::<TheScrollableIconRow>()
        {
            strip.set_selected(index);
        }
        if let Some(widget) = ui.get_widget(EFFECT_EMISSION_SHAPE)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(match particle.map(|effect| effect.emitter.emission_shape) {
                Some(rusterix::ParticleEmissionShape::Point) => 0,
                Some(rusterix::ParticleEmissionShape::Box) => 1,
                Some(rusterix::ParticleEmissionShape::Surface) => 2,
                None => 0,
            });
        }
        if let Some(widget) = ui.get_widget(EFFECT_SIZE_PROFILE)
            && let Some(group) = widget.as_group_button()
        {
            let index = particle
                .map(|effect| {
                    let curve = effect.emitter.size_curve;
                    if curve[3] > curve[0] * 1.15 {
                        0
                    } else if curve[3] < curve[0] * 0.75 {
                        2
                    } else {
                        1
                    }
                })
                .unwrap_or(1);
            group.set_index(index);
        }
        if let Some(widget) = ui.get_widget(EFFECT_FADE_PROFILE)
            && let Some(group) = widget.as_group_button()
        {
            let index = particle
                .map(|effect| {
                    let curve = effect.emitter.opacity_curve;
                    if curve[3] > 0.75 {
                        2
                    } else if curve[2] > 0.75 {
                        1
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            group.set_index(index);
        }
        let selected_part_id = particle
            .map(|effect| effect.part_id)
            .or_else(|| light.map(|effect| effect.part_id));
        let enabled = particle
            .map(|effect| effect.enabled)
            .or_else(|| light.map(|effect| effect.enabled))
            .unwrap_or(false);
        ctx.ui.set_widget_state(
            EFFECT_ENABLED.to_string(),
            if enabled {
                TheWidgetState::Selected
            } else {
                TheWidgetState::None
            },
        );
        self.effect_part_options.clear();
        if let Some(dropdown) = ui.get_drop_down_menu(EFFECT_PART) {
            dropdown.clear_options();
            for part in &asset.parts {
                dropdown.add_option(part.name.clone());
                self.effect_part_options.push(part.id);
            }
            let selected = self
                .effect_part_options
                .iter()
                .position(|part_id| Some(*part_id) == selected_part_id)
                .unwrap_or(0);
            dropdown.set_selected_index(selected as i32);
        }
        let attachment = particle
            .map(|effect| (effect.part_id, effect.attachment_id))
            .or_else(|| light.map(|effect| (effect.part_id, effect.attachment_id)))
            .and_then(|(part_id, attachment_id)| {
                asset.find_part(part_id).and_then(|part| {
                    part.attachments
                        .iter()
                        .find(|attachment| attachment.id == attachment_id)
                })
            });
        let set_text = |ui: &mut TheUI, id: &str, value: String, ctx: &mut TheContext| {
            ui.set_widget_value(id, ctx, TheValue::Text(value));
        };
        set_text(
            ui,
            EFFECT_NAME,
            particle
                .map(|effect| effect.name.clone())
                .or_else(|| light.map(|effect| effect.name.clone()))
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_POSITION,
            attachment
                .map(|a| {
                    format!(
                        "{:.3}, {:.3}, {:.3}",
                        a.position[0], a.position[1], a.position[2]
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_DIRECTION,
            attachment
                .map(|a| {
                    format!(
                        "{:.3}, {:.3}, {:.3}",
                        a.direction[0], a.direction[1], a.direction[2]
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_RATE,
            particle
                .map(|e| format!("{:.3}", e.emitter.rate))
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_SPREAD,
            particle
                .map(|e| format!("{:.3}", e.emitter.spread))
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_LIFETIME,
            particle
                .map(|e| {
                    format!(
                        "{:.3}",
                        (e.emitter.lifetime_range.0 + e.emitter.lifetime_range.1) * 0.5
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_RADIUS,
            particle
                .map(|e| {
                    format!(
                        "{:.3}",
                        (e.emitter.radius_range.0 + e.emitter.radius_range.1) * 0.5
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_SPEED,
            particle
                .map(|e| {
                    format!(
                        "{:.3}",
                        (e.emitter.speed_range.0 + e.emitter.speed_range.1) * 0.5
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        for (id, axis) in [
            (EFFECT_EMISSION_WIDTH, 0),
            (EFFECT_EMISSION_HEIGHT, 1),
            (EFFECT_EMISSION_DEPTH, 2),
        ] {
            set_text(
                ui,
                id,
                particle
                    .map(|effect| format!("{:.3}", effect.emitter.spawn_area[axis] * 2.0))
                    .unwrap_or_default(),
                ctx,
            );
        }
        set_text(
            ui,
            EFFECT_GRAVITY,
            particle
                .map(|e| {
                    format!(
                        "{:.3}, {:.3}, {:.3}",
                        e.emitter.gravity[0], e.emitter.gravity[1], e.emitter.gravity[2]
                    )
                })
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_TURBULENCE,
            particle
                .map(|e| format!("{:.3}", e.emitter.turbulence))
                .unwrap_or_default(),
            ctx,
        );
        if let Some(widget) = ui.get_widget(EFFECT_COLOR)
            && let Some(picker) = widget.as_any().downcast_mut::<ThePaletteIndexPicker>()
        {
            picker.set_palette(project.art_palette.clone());
            picker.set_selected_index(
                light
                    .map(|effect| effect.color)
                    .map(|color| Self::effect_palette_index(project, color))
                    .unwrap_or(project.art_palette.current_index as i32),
            );
        }
        if let Some(widget) = ui.get_widget(EFFECT_COLOR_RAMP)
            && let Some(picker) = widget.as_any().downcast_mut::<ThePaletteIndexRowPicker>()
        {
            picker.set_palette(project.art_palette.clone());
            picker.set_selected_indices(
                particle
                    .and_then(|effect| effect.emitter.color_ramp)
                    .map(|ramp| {
                        ramp.into_iter()
                            .map(|color| Self::effect_palette_index(project, color))
                            .collect()
                    })
                    .unwrap_or_else(|| vec![project.art_palette.current_index as i32; 4]),
            );
        }
        set_text(
            ui,
            EFFECT_INTENSITY,
            light
                .map(|e| format!("{:.3}", e.intensity))
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_RANGE,
            light.map(|e| format!("{:.3}", e.range)).unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_FLICKER,
            light
                .map(|e| format!("{:.3}", e.flicker))
                .unwrap_or_default(),
            ctx,
        );
        set_text(
            ui,
            EFFECT_LIGHT_LIFT,
            light.map(|e| format!("{:.3}", e.lift)).unwrap_or_default(),
            ctx,
        );
        for id in [
            EFFECT_NAME,
            EFFECT_POSITION,
            EFFECT_DIRECTION,
            EFFECT_ENABLED,
            EFFECT_PART,
            EFFECT_DUPLICATE,
            EFFECT_REMOVE,
        ] {
            if self.selected_effect_id.is_some() {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
        for id in [
            EFFECT_RATE,
            EFFECT_SPREAD,
            EFFECT_LIFETIME,
            EFFECT_RADIUS,
            EFFECT_SPEED,
            EFFECT_EMISSION_WIDTH,
            EFFECT_EMISSION_HEIGHT,
            EFFECT_EMISSION_DEPTH,
            EFFECT_SIZE_PROFILE,
            EFFECT_FADE_PROFILE,
            EFFECT_GRAVITY,
            EFFECT_TURBULENCE,
            EFFECT_COLOR_RAMP,
            EFFECT_EMISSION_SHAPE,
            EFFECT_FIT_PART_TOP,
        ] {
            if particle.is_some() {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
        for id in [
            EFFECT_COLOR,
            EFFECT_INTENSITY,
            EFFECT_RANGE,
            EFFECT_FLICKER,
            EFFECT_LIGHT_LIFT,
        ] {
            if light.is_some() {
                ui.set_enabled(id, ctx);
            } else {
                ui.set_disabled(id, ctx);
            }
        }
        ctx.ui.relayout = true;
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
            Some("tool.effects") => PrefabEditorMode::Effects,
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
            selected_effect_id: None,
            effect_inspector_page: 0,
            effect_preset_applying: false,
            effect_drag: None,
            effect_part_options: Vec::new(),
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
        stack.add_canvas(Self::effects_canvas());
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
        self.selected_effect_id = None;
        self.effect_preset_applying = false;
        self.effect_drag = None;
        server_ctx.selected_prefab_effect_id = None;
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
        if self.mode == PrefabEditorMode::Effects {
            match event {
                TheEvent::RenderViewClicked(id, _) if id.name == PREFAB_VIEW => {
                    if let Some(GeoId::Gizmo(gizmo)) = server_ctx.geo_hit
                        && let Some(asset) = project.block_props.get(&asset_id)
                        && let Some((effect_id, direction_handle)) =
                            Self::effect_id_for_gizmo(asset, gizmo)
                        && let Some((part_id, attachment_id)) =
                            Self::effect_attachment_ref(asset, effect_id)
                        && let Some(attachment) = asset.find_part(part_id).and_then(|part| {
                            part.attachments.iter().find(|a| a.id == attachment_id)
                        })
                    {
                        self.selected_effect_id = Some(effect_id);
                        server_ctx.selected_prefab_effect_id = Some(effect_id);
                        let origin = Vec3::from(attachment.position);
                        let plane_normal = server_ctx
                            .hover_ray_dir_3d
                            .and_then(|direction| direction.try_normalized())
                            .unwrap_or(Vec3::unit_z());
                        self.effect_drag = Some(PrefabEffectDrag {
                            effect_id,
                            direction_handle,
                            origin,
                            plane_normal,
                            before: Box::new(project.clone()),
                            changed: false,
                        });
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Geometry Overlay 3D"),
                            TheValue::Empty,
                        ));
                        return true;
                    }
                }
                TheEvent::RenderViewDragged(id, _) if id.name == PREFAB_VIEW => {
                    if let Some(drag) = self.effect_drag.as_mut()
                        && let Some(point) =
                            Self::ray_drag_point(server_ctx, drag.origin, drag.plane_normal)
                        && let Some(asset) = project.block_props.get_mut(&asset_id)
                        && let Some(attachment) = Self::effect_attachment_mut(asset, drag.effect_id)
                    {
                        if drag.direction_handle {
                            if let Some(direction) = (point - drag.origin).try_normalized() {
                                attachment.direction = [direction.x, direction.y, direction.z];
                                drag.changed = true;
                            }
                        } else {
                            attachment.position = [point.x, point.y, point.z];
                            drag.origin = point;
                            drag.changed = true;
                        }
                        Self::sync_prefab_runtime(project);
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        if let Some(translated) = Self::translated_view_event(event) {
                            ctx.ui.send(translated);
                        }
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Geometry Overlay 3D"),
                            TheValue::Empty,
                        ));
                        return true;
                    }
                }
                TheEvent::RenderViewUp(id, _) if id.name == PREFAB_VIEW => {
                    if let Some(drag) = self.effect_drag.take() {
                        if drag.changed {
                            Self::push_project_undo(*drag.before, project, ctx);
                        }
                        return true;
                    }
                }
                _ => {}
            }
        }
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
        if self.mode == PrefabEditorMode::Palette {
            let edits_prefab = self.palette_dock.edits_map_for_event(event);
            let handled = self
                .palette_dock
                .handle_event(event, ui, ctx, project, server_ctx);
            if edits_prefab {
                match crate::block_props::sync_prefab_editor(project, asset_id) {
                    Ok(()) => Self::sync_prefab_runtime(project),
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
            }
            if handled || edits_prefab {
                return true;
            }
        }

        match event {
            TheEvent::Custom(id, _) if id.name == "Tool Changed" => {
                self.close_door_preview(project, asset_id, server_ctx);
                self.mode = Self::active_tool_mode();
                if let Some(layout) = ui.get_sharedvlayout("Prefab Editor Shared VLayout") {
                    // Effects need enough vertical room for two compact
                    // property columns while keeping a useful live viewport.
                    layout.set_shared_ratio(if self.mode == PrefabEditorMode::Effects {
                        0.56
                    } else {
                        0.68
                    });
                }
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
                } else if self.mode == PrefabEditorMode::Effects {
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    server_ctx.selected_prefab_effect_id = self.selected_effect_id;
                }
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::NewListItemSelected(id, layout_id)
                if layout_id.name == EFFECT_TREE
                    && (id.name == PARTICLE_EFFECT_ITEM || id.name == LIGHT_EFFECT_ITEM) =>
            {
                self.selected_effect_id = Some(id.uuid);
                if id.name == LIGHT_EFFECT_ITEM {
                    self.effect_inspector_page = 3;
                } else if self.effect_inspector_page == 3 {
                    self.effect_inspector_page = 1;
                }
                server_ctx.selected_prefab_effect_id = Some(id.uuid);
                self.sync_effect_editor(ui, ctx, project, asset_id);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == EFFECT_INSPECTOR_PAGES => {
                self.effect_inspector_page = (*index).min(3);
                if let Some(stack) = ui.get_stack_layout(EFFECT_INSPECTOR_STACK) {
                    stack.set_index(self.effect_inspector_page);
                }
                ctx.ui.relayout = true;
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == EFFECT_PRESET_STRIP => {
                self.effect_preset_applying = true;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(EFFECT_APPLY_PRESET),
                    TheValue::Int(*index as i32),
                ));
                true
            }
            TheEvent::Custom(id, TheValue::Int(index)) if id.name == EFFECT_APPLY_PRESET => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(EFFECT_FINISH_PRESET),
                    TheValue::Empty,
                ));
                let Some(preset) = usize::try_from(*index)
                    .ok()
                    .and_then(|index| Self::PARTICLE_PRESETS.get(index))
                    .copied()
                else {
                    return true;
                };
                let Some(before) = project.block_props.get(&asset_id).cloned() else {
                    return true;
                };
                match self.apply_particle_preset(project, asset_id, preset) {
                    Ok(()) => {
                        if let Some(after) = project.block_props.get(&asset_id).cloned() {
                            UNDOMANAGER.write().unwrap().add_undo(
                                ProjectUndoAtom::PrefabAssetEdit(
                                    asset_id,
                                    Box::new(before),
                                    Box::new(after),
                                ),
                                ctx,
                            );
                        }
                        Self::sync_prefab_runtime(project);
                        RUSTERIX
                            .write()
                            .unwrap()
                            .scene_handler
                            .reset_builder_particle_emitters();
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            format!("Applied {preset} to the selected particles"),
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == EFFECT_FINISH_PRESET => {
                self.effect_preset_applying = false;
                true
            }
            TheEvent::ValueChanged(_, _) if self.effect_preset_applying => true,
            TheEvent::IndexChanged(id, index) if id.name == EFFECT_PLACEMENT_MODE => {
                let before = project.clone();
                if let Some(asset) = project.block_props.get_mut(&asset_id) {
                    asset.placement.mode = match index {
                        1 => rusterix::BlockPropPlacementMode::Wall,
                        2 => rusterix::BlockPropPlacementMode::AnySurface,
                        3 => rusterix::BlockPropPlacementMode::Free,
                        _ => rusterix::BlockPropPlacementMode::Ground,
                    };
                    asset.placement.snap_to_grid = *index == 0;
                    asset.placement.snap_to_surfaces = *index != 3;
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == EFFECT_ADD_PARTICLES =>
            {
                let before = project.clone();
                let preset = "Flame";
                match self.add_particle_preset(project, asset_id, preset) {
                    Ok(_) => {
                        server_ctx.selected_prefab_effect_id = self.selected_effect_id;
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            format!("Added {preset} particles"),
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == EFFECT_ADD_LIGHT => {
                let before = project.clone();
                match self.add_light_effect(project, asset_id) {
                    Ok(_) => {
                        self.effect_inspector_page = 3;
                        server_ctx.selected_prefab_effect_id = self.selected_effect_id;
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Added Prefab light".to_string(),
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == EFFECT_DUPLICATE => {
                let before = project.clone();
                match self.duplicate_selected_effect(project, asset_id) {
                    Ok(_) => {
                        server_ctx.selected_prefab_effect_id = self.selected_effect_id;
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Duplicated Prefab effect".to_string(),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Geometry Overlay 3D"),
                            TheValue::Empty,
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Err(message) => ctx
                        .ui
                        .send(TheEvent::SetStatusText(TheId::empty(), message)),
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == EFFECT_REMOVE => {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let before = project.clone();
                let mut removed_attachment = None;
                if let Some(asset) = project.block_props.get_mut(&asset_id) {
                    if let Some(index) = asset
                        .particle_effects
                        .iter()
                        .position(|effect| effect.id == effect_id)
                    {
                        let effect = asset.particle_effects.remove(index);
                        removed_attachment = Some((effect.part_id, effect.attachment_id));
                    } else if let Some(index) = asset
                        .light_effects
                        .iter()
                        .position(|effect| effect.id == effect_id)
                    {
                        let effect = asset.light_effects.remove(index);
                        removed_attachment = Some((effect.part_id, effect.attachment_id));
                    }
                    if let Some((part_id, attachment_id)) = removed_attachment {
                        let still_used = asset.particle_effects.iter().any(|effect| {
                            effect.part_id == part_id && effect.attachment_id == attachment_id
                        }) || asset.light_effects.iter().any(|effect| {
                            effect.part_id == part_id && effect.attachment_id == attachment_id
                        });
                        if !still_used
                            && let Some(part) =
                                asset.parts.iter_mut().find(|part| part.id == part_id)
                        {
                            part.attachments
                                .retain(|attachment| attachment.id != attachment_id);
                        }
                    }
                }
                self.selected_effect_id = None;
                server_ctx.selected_prefab_effect_id = None;
                Self::push_project_undo(before, project, ctx);
                Self::sync_prefab_runtime(project);
                self.sync_effect_editor(ui, ctx, project, asset_id);
                ctx.ui.redraw_all = true;
                true
            }
            TheEvent::StateChanged(id, state) if id.name == EFFECT_ENABLED => {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let enabled = *state == TheWidgetState::Selected;
                let before = project.clone();
                let mut changed = false;
                if let Some(asset) = project.block_props.get_mut(&asset_id) {
                    if let Some(effect) = asset
                        .particle_effects
                        .iter_mut()
                        .find(|effect| effect.id == effect_id)
                    {
                        changed = effect.enabled != enabled;
                        effect.enabled = enabled;
                    } else if let Some(effect) = asset
                        .light_effects
                        .iter_mut()
                        .find(|effect| effect.id == effect_id)
                    {
                        changed = effect.enabled != enabled;
                        effect.enabled = enabled;
                    }
                }
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Geometry Overlay 3D"),
                        TheValue::Empty,
                    ));
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == EFFECT_PART => {
                let Some(part_id) = self.effect_part_options.get(*index).copied() else {
                    return true;
                };
                let before = project.clone();
                match self.move_selected_effect_to_part(project, asset_id, part_id) {
                    Ok(true) => {
                        Self::push_project_undo(before, project, ctx);
                        Self::sync_prefab_runtime(project);
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            "Moved effect attachment to the selected Prefab part".to_string(),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Geometry Overlay 3D"),
                            TheValue::Empty,
                        ));
                        ctx.ui.redraw_all = true;
                    }
                    Ok(false) => {}
                    Err(message) => {
                        self.sync_effect_editor(ui, ctx, project, asset_id);
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), message));
                    }
                }
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == EFFECT_EMISSION_SHAPE => {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let shape = match index {
                    0 => rusterix::ParticleEmissionShape::Point,
                    2 => rusterix::ParticleEmissionShape::Surface,
                    _ => rusterix::ParticleEmissionShape::Box,
                };
                let before = project.clone();
                let mut changed = false;
                if let Some(effect) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .particle_effects
                        .iter_mut()
                        .find(|effect| effect.id == effect_id)
                }) {
                    changed = effect.emitter.emission_shape != shape;
                    effect.emitter.emission_shape = shape;
                    let spawn_area = match shape {
                        rusterix::ParticleEmissionShape::Point => [0.0; 3],
                        rusterix::ParticleEmissionShape::Box
                            if effect.emitter.spawn_area == [0.0; 3] =>
                        {
                            [0.25, 0.25, 0.25]
                        }
                        rusterix::ParticleEmissionShape::Surface
                            if effect.emitter.spawn_area == [0.0; 3] =>
                        {
                            [0.5, 0.0, 0.5]
                        }
                        _ => effect.emitter.spawn_area,
                    };
                    if effect.emitter.spawn_area != spawn_area {
                        effect.emitter.spawn_area = spawn_area;
                        changed = true;
                    }
                }
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    RUSTERIX
                        .write()
                        .unwrap()
                        .scene_handler
                        .reset_builder_particle_emitters();
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::IndexChanged(id, index)
                if id.name == EFFECT_SIZE_PROFILE || id.name == EFFECT_FADE_PROFILE =>
            {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let before = project.clone();
                let mut changed = false;
                if let Some(effect) = project.block_props.get_mut(&asset_id).and_then(|asset| {
                    asset
                        .particle_effects
                        .iter_mut()
                        .find(|effect| effect.id == effect_id)
                }) {
                    if id.name == EFFECT_SIZE_PROFILE {
                        let curve = match index {
                            0 => [0.55, 0.9, 1.35, 1.8],
                            2 => [1.0, 0.75, 0.4, 0.05],
                            _ => [1.0; 4],
                        };
                        changed = effect.emitter.size_curve != curve;
                        effect.emitter.size_curve = curve;
                    } else {
                        let curve = match index {
                            1 => [1.0, 1.0, 0.9, 0.0],
                            2 => [1.0; 4],
                            _ => [0.0, 1.0, 0.55, 0.0],
                        };
                        changed = effect.emitter.opacity_curve != curve;
                        effect.emitter.opacity_curve = curve;
                    }
                }
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == EFFECT_FIT_PART_TOP =>
            {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let before = project.clone();
                let changed = project
                    .block_props
                    .get_mut(&asset_id)
                    .is_some_and(|asset| Self::fit_particle_emission_to_part_top(asset, effect_id));
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    RUSTERIX
                        .write()
                        .unwrap()
                        .scene_handler
                        .reset_builder_particle_emitters();
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Emission now covers the attached part's top surface".to_string(),
                    ));
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::PaletteIndexChanged(id, palette_index)
                if id.name == EFFECT_COLOR || Self::effect_color_ramp_slot(&id.name).is_some() =>
            {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let Some(palette_color) = Self::effect_palette_color(project, *palette_index)
                else {
                    return true;
                };
                let before = project.clone();
                let mut changed = false;
                if let Some(asset) = project.block_props.get_mut(&asset_id) {
                    if id.name == EFFECT_COLOR {
                        if let Some(effect) = asset
                            .particle_effects
                            .iter_mut()
                            .find(|effect| effect.id == effect_id)
                        {
                            changed = effect.emitter.color != palette_color;
                            effect.emitter.color = palette_color;
                        } else if let Some(effect) = asset
                            .light_effects
                            .iter_mut()
                            .find(|effect| effect.id == effect_id)
                        {
                            changed = effect.color != palette_color;
                            effect.color = palette_color;
                        }
                    } else if let Some(slot) = Self::effect_color_ramp_slot(&id.name)
                        && let Some(effect) = asset
                            .particle_effects
                            .iter_mut()
                            .find(|effect| effect.id == effect_id)
                    {
                        let mut ramp = effect
                            .emitter
                            .color_ramp
                            .unwrap_or([effect.emitter.color; 4]);
                        let mut color = palette_color;
                        // Lifetime fading is authored separately from the palette.
                        // Keep the stop's alpha while replacing its visible color.
                        color[3] = ramp[slot][3];
                        changed = ramp[slot] != color;
                        ramp[slot] = color;
                        effect.emitter.color_ramp = Some(ramp);
                    }
                }
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    if id.name != EFFECT_COLOR {
                        RUSTERIX
                            .write()
                            .unwrap()
                            .scene_handler
                            .reset_builder_particle_emitters();
                    }
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.redraw_all = true;
                }
                true
            }
            TheEvent::ValueChanged(id, value) if id.name == EFFECT_SURFACE_OFFSET => {
                if let Some(offset) = value.to_f32()
                    && offset >= 0.0
                {
                    let before = project.clone();
                    if let Some(asset) = project.block_props.get_mut(&asset_id) {
                        if (asset.placement.surface_offset - offset).abs() > f32::EPSILON {
                            asset.placement.surface_offset = offset;
                            Self::push_project_undo(before, project, ctx);
                            Self::sync_prefab_runtime(project);
                        }
                    }
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                } else {
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Surface offset must be zero or greater".to_string(),
                    ));
                }
                true
            }
            TheEvent::ValueChanged(id, value)
                if matches!(
                    id.name.as_str(),
                    EFFECT_NAME
                        | EFFECT_POSITION
                        | EFFECT_DIRECTION
                        | EFFECT_RATE
                        | EFFECT_SPREAD
                        | EFFECT_LIFETIME
                        | EFFECT_RADIUS
                        | EFFECT_SPEED
                        | EFFECT_EMISSION_WIDTH
                        | EFFECT_EMISSION_HEIGHT
                        | EFFECT_EMISSION_DEPTH
                        | EFFECT_GRAVITY
                        | EFFECT_TURBULENCE
                        | EFFECT_INTENSITY
                        | EFFECT_RANGE
                        | EFFECT_FLICKER
                        | EFFECT_LIGHT_LIFT
                ) =>
            {
                let Some(effect_id) = self.selected_effect_id else {
                    return true;
                };
                let text = value.to_string().unwrap_or_default();
                let number = value.to_f32().or_else(|| text.trim().parse::<f32>().ok());
                let before = project.clone();
                let mut changed = false;
                if let Some(asset) = project.block_props.get_mut(&asset_id) {
                    let particle_index = asset
                        .particle_effects
                        .iter()
                        .position(|effect| effect.id == effect_id);
                    let light_index = asset
                        .light_effects
                        .iter()
                        .position(|effect| effect.id == effect_id);
                    let attachment_ref = particle_index
                        .map(|index| {
                            let effect = &asset.particle_effects[index];
                            (effect.part_id, effect.attachment_id)
                        })
                        .or_else(|| {
                            light_index.map(|index| {
                                let effect = &asset.light_effects[index];
                                (effect.part_id, effect.attachment_id)
                            })
                        });
                    match id.name.as_str() {
                        EFFECT_NAME => {
                            let name = text.trim();
                            if !name.is_empty() {
                                if let Some(index) = particle_index {
                                    asset.particle_effects[index].name = name.to_string();
                                } else if let Some(index) = light_index {
                                    asset.light_effects[index].name = name.to_string();
                                }
                                changed = true;
                            }
                        }
                        EFFECT_POSITION | EFFECT_DIRECTION => {
                            if let Some(vector) = Self::parse_vec3(&text)
                                && let Some((part_id, attachment_id)) = attachment_ref
                                && let Some(attachment) = asset
                                    .parts
                                    .iter_mut()
                                    .find(|part| part.id == part_id)
                                    .and_then(|part| {
                                        part.attachments
                                            .iter_mut()
                                            .find(|attachment| attachment.id == attachment_id)
                                    })
                            {
                                if id.name == EFFECT_POSITION {
                                    attachment.position = vector;
                                } else {
                                    let direction = Vec3::new(vector[0], vector[1], vector[2])
                                        .try_normalized()
                                        .unwrap_or(Vec3::unit_y());
                                    attachment.direction = [direction.x, direction.y, direction.z];
                                }
                                changed = true;
                            }
                        }
                        EFFECT_RATE | EFFECT_SPREAD => {
                            if let Some(index) = particle_index
                                && let Some(number) = number
                            {
                                if id.name == EFFECT_RATE {
                                    asset.particle_effects[index].emitter.rate = number.max(0.0);
                                } else {
                                    asset.particle_effects[index].emitter.spread =
                                        number.clamp(0.0, std::f32::consts::PI);
                                }
                                changed = true;
                            }
                        }
                        EFFECT_LIFETIME | EFFECT_RADIUS | EFFECT_SPEED => {
                            if let Some(index) = particle_index
                                && let Some(center) = number
                            {
                                if id.name == EFFECT_LIFETIME {
                                    let range = Self::effect_range_around(center, 0.2);
                                    asset.particle_effects[index].emitter.lifetime_range = range;
                                } else if id.name == EFFECT_RADIUS {
                                    let range = Self::effect_range_around(center, 0.25);
                                    asset.particle_effects[index].emitter.radius_range = range;
                                } else {
                                    let range = Self::effect_range_around(center, 0.3);
                                    asset.particle_effects[index].emitter.speed_range = range;
                                }
                                changed = true;
                            }
                        }
                        EFFECT_EMISSION_WIDTH | EFFECT_EMISSION_HEIGHT | EFFECT_EMISSION_DEPTH => {
                            if let Some(index) = particle_index
                                && let Some(number) = number
                            {
                                let axis = match id.name.as_str() {
                                    EFFECT_EMISSION_HEIGHT => 1,
                                    EFFECT_EMISSION_DEPTH => 2,
                                    _ => 0,
                                };
                                asset.particle_effects[index].emitter.spawn_area[axis] =
                                    number.abs() * 0.5;
                                changed = true;
                            }
                        }
                        EFFECT_GRAVITY => {
                            if let Some(index) = particle_index
                                && let Some(vector) = Self::parse_vec3(&text)
                            {
                                asset.particle_effects[index].emitter.gravity = vector;
                                changed = true;
                            }
                        }
                        EFFECT_TURBULENCE => {
                            if let Some(index) = particle_index
                                && let Some(number) = number
                            {
                                asset.particle_effects[index].emitter.turbulence = number.max(0.0);
                                changed = true;
                            }
                        }
                        EFFECT_INTENSITY | EFFECT_RANGE | EFFECT_FLICKER | EFFECT_LIGHT_LIFT => {
                            if let Some(index) = light_index
                                && let Some(number) = number
                            {
                                if id.name == EFFECT_INTENSITY {
                                    asset.light_effects[index].intensity = number.max(0.0);
                                } else if id.name == EFFECT_RANGE {
                                    asset.light_effects[index].range = number.max(0.0);
                                } else if id.name == EFFECT_FLICKER {
                                    asset.light_effects[index].flicker = number.max(0.0);
                                } else {
                                    asset.light_effects[index].lift = number;
                                }
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                }
                if changed {
                    Self::push_project_undo(before, project, ctx);
                    Self::sync_prefab_runtime(project);
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.redraw_all = true;
                } else {
                    self.sync_effect_editor(ui, ctx, project, asset_id);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "The effect value is invalid; the previous value was restored".to_string(),
                    ));
                }
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
                } else if self.mode == PrefabEditorMode::Effects {
                    self.sync_effect_editor(ui, ctx, project, asset_id);
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
        assert_eq!(PrefabEditorMode::Effects.index(), 4);
    }

    #[test]
    fn prefab_effects_can_be_duplicated_and_moved_between_parts() {
        let mut project = Project::default();
        let mut asset = rusterix::BlockPropAsset::new("Effect Test");
        asset
            .parts
            .push(rusterix::BlockPropPart::new_authored("Body", Vec::new()));
        asset
            .parts
            .push(rusterix::BlockPropPart::new_authored("Top", Vec::new()));
        let asset_id = asset.id;
        let body_id = asset.parts[0].id;
        let top_id = asset.parts[1].id;
        project.block_props.insert(asset_id, asset);

        let mut dock = PrefabsEditorDock::new();
        dock.selected_part_id = Some(body_id);
        let original_id = dock
            .add_particle_preset(&mut project, asset_id, "Flame")
            .unwrap();
        let copy_id = dock
            .duplicate_selected_effect(&mut project, asset_id)
            .unwrap();
        assert_ne!(original_id, copy_id);

        let asset = project.block_props.get(&asset_id).unwrap();
        let original = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == original_id)
            .unwrap();
        let copy = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == copy_id)
            .unwrap();
        assert_ne!(original.attachment_id, copy.attachment_id);
        assert_eq!(copy.part_id, body_id);

        assert!(
            dock.move_selected_effect_to_part(&mut project, asset_id, top_id)
                .unwrap()
        );
        let asset = project.block_props.get(&asset_id).unwrap();
        let copy = asset
            .particle_effects
            .iter()
            .find(|effect| effect.id == copy_id)
            .unwrap();
        assert_eq!(copy.part_id, top_id);
        assert!(
            asset
                .find_part(top_id)
                .unwrap()
                .attachments
                .iter()
                .any(|attachment| attachment.id == copy.attachment_id)
        );
    }
}
