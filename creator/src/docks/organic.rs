use crate::editor::{PALETTE, SCENEMANAGER, UNDOMANAGER};
use crate::prelude::*;
use crate::tools::organic::OrganicTool;

const ORGANIC_SETTINGS_LAYOUT: &str = "Organic Brush Settings";
const ORGANIC_PREVIEW_LAYOUT: &str = "Organic Brush Preview Layout";
const ORGANIC_BRUSH_SELECTOR_LAYOUT: &str = "Organic Brush Selector Layout";
const ORGANIC_MAIN_SPLIT_LAYOUT: &str = "Organic Brush Main Split Layout";
const ORGANIC_RENDER_TOGGLE_BUTTON: &str = "Organic Brush Render Toggle";
const ORGANIC_LOCK_MODE_DROPDOWN: &str = "Organic Brush Lock Mode";
const ORGANIC_CLEAR_BUTTON: &str = "Organic Brush Clear";
const ORGANIC_COLOR_BASE: &str = "organicBrushColorBase";
const ORGANIC_COLOR_BORDER: &str = "organicBrushColorBorder";
const ORGANIC_COLOR_NOISE: &str = "organicBrushColorNoise";
const ORGANIC_BRUSH_SIZE: &str = "organicBrushSize";
const ORGANIC_BORDER_SIZE: &str = "organicBrushBorderSize";
const ORGANIC_NOISE_AMOUNT: &str = "organicBrushNoiseAmount";
const ORGANIC_OPACITY: &str = "organicBrushOpacity";
const ORGANIC_PAINT_MODE: &str = "organicBrushPaintMode";

const ORGANIC_PRESET_VIEW_MOSS: &str = "organicBrushPresetMoss";
const ORGANIC_PRESET_VIEW_MUD: &str = "organicBrushPresetMud";
const ORGANIC_PRESET_VIEW_GRIME: &str = "organicBrushPresetGrime";
const ORGANIC_PRESET_VIEW_BUBBLES: &str = "organicBrushPresetBubbles";
const ORGANIC_PRESET_VIEW_VINES: &str = "organicBrushPresetVines";

const PROP_PRESET: &str = "organic_brush_preset";
const PROP_RADIUS: &str = "organic_brush_radius";
const PROP_FLOW: &str = "organic_brush_flow";
const PROP_JITTER: &str = "organic_brush_jitter";
const PROP_DEPTH: &str = "organic_brush_depth";
const PROP_CELL_SIZE: &str = "organic_brush_cell_size";
const PROP_SHAPE_MODE: &str = "organic_brush_shape_mode";
const PROP_SOFTNESS: &str = "organic_brush_softness";
const PROP_SCATTER_COUNT: &str = "organic_brush_scatter_count";
const PROP_SCATTER_JITTER: &str = "organic_brush_scatter_jitter";
const PROP_HEIGHT_FALLOFF: &str = "organic_brush_height_falloff";
const PROP_NOISE_SCALE: &str = "organic_brush_noise_scale";
const PROP_NOISE_STRENGTH: &str = "organic_brush_noise_strength";
const PROP_NOISE_SEED: &str = "organic_brush_noise_seed";
const PROP_CHANNEL: &str = "organic_brush_channel";
const PROP_LINE_LENGTH: &str = "organic_brush_line_length";
const PROP_LINE_WIDTH: &str = "organic_brush_line_width";
const PROP_LINE_SOFTNESS: &str = "organic_brush_line_softness";
const PROP_PALETTE_1: &str = "organic_brush_palette_1";
const PROP_PALETTE_2: &str = "organic_brush_palette_2";
const PROP_PALETTE_3: &str = "organic_brush_palette_3";
const PROP_BORDER_SIZE: &str = "organic_brush_border_size";
const PROP_OPACITY: &str = "organic_brush_opacity";
const PROP_PAINT_MODE: &str = "organic_brush_paint_mode";
const PROP_RENDER_ACTIVE: &str = "organic_render_active";
const PROP_LOCK_MODE: &str = "organic_paint_lock_mode";

const PREVIEW_SIZE: i32 = 220;
const PRESET_PREVIEW_WIDTH: i32 = 116;
const PRESET_PREVIEW_HEIGHT: i32 = 72;

#[derive(Clone, Copy)]
enum OrganicPreset {
    Moss,
    Mud,
    Grime,
    Bubbles,
    Vines,
}

impl OrganicPreset {
    fn all() -> [Self; 5] {
        [
            Self::Moss,
            Self::Mud,
            Self::Grime,
            Self::Bubbles,
            Self::Vines,
        ]
    }

    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Mud,
            2 => Self::Grime,
            3 => Self::Bubbles,
            4 => Self::Vines,
            _ => Self::Moss,
        }
    }

    fn index(self) -> i32 {
        match self {
            Self::Moss => 0,
            Self::Mud => 1,
            Self::Grime => 2,
            Self::Bubbles => 3,
            Self::Vines => 4,
        }
    }

    fn widget_id(self) -> &'static str {
        match self {
            Self::Moss => ORGANIC_PRESET_VIEW_MOSS,
            Self::Mud => ORGANIC_PRESET_VIEW_MUD,
            Self::Grime => ORGANIC_PRESET_VIEW_GRIME,
            Self::Bubbles => ORGANIC_PRESET_VIEW_BUBBLES,
            Self::Vines => ORGANIC_PRESET_VIEW_VINES,
        }
    }

    fn from_widget_id(id: &str) -> Option<Self> {
        match id {
            ORGANIC_PRESET_VIEW_MOSS => Some(Self::Moss),
            ORGANIC_PRESET_VIEW_MUD => Some(Self::Mud),
            ORGANIC_PRESET_VIEW_GRIME => Some(Self::Grime),
            ORGANIC_PRESET_VIEW_BUBBLES => Some(Self::Bubbles),
            ORGANIC_PRESET_VIEW_VINES => Some(Self::Vines),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct BrushPreviewStyle {
    base: [u8; 4],
    border: [u8; 4],
    noise: [u8; 4],
    border_size: f32,
    noise_amount: f32,
    opacity: f32,
    paint_mode: i32,
}

pub struct OrganicDock;

impl Dock for OrganicDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut lock_mode = TheDropdownMenu::new(TheId::named(ORGANIC_LOCK_MODE_DROPDOWN));
        lock_mode.add_option(fl!("organic_mode_free"));
        lock_mode.add_option(fl!("organic_mode_locked"));
        lock_mode.set_status_text(&fl!("status_organic_lock_mode"));
        toolbar_hlayout.add_widget(Box::new(lock_mode));

        let mut clear_button = TheTraybarButton::new(TheId::named(ORGANIC_CLEAR_BUTTON));
        clear_button.set_text(fl!("clear"));
        clear_button.set_status_text(&fl!("status_organic_clear"));
        toolbar_hlayout.add_widget(Box::new(clear_button));

        let mut toggle_button = TheTraybarButton::new(TheId::named(ORGANIC_RENDER_TOGGLE_BUTTON));
        toggle_button.set_text(fl!("organic_toggle_active"));
        toggle_button.set_status_text(&fl!("status_organic_toggle_visibility"));
        toolbar_hlayout.add_widget(Box::new(toggle_button));
        toolbar_hlayout.set_reverse_index(Some(1));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut main_canvas = TheCanvas::new();
        let mut main_split = TheSharedHLayout::new(TheId::named(ORGANIC_MAIN_SPLIT_LAYOUT));
        main_split.set_mode(TheSharedHLayoutMode::Shared);
        main_split.set_shared_ratio(0.62);
        main_split.set_background_color(None);

        let mut preview_canvas = TheCanvas::new();
        let mut preview_layout = TheRGBALayout::new(TheId::named(ORGANIC_PREVIEW_LAYOUT));
        preview_layout
            .limiter_mut()
            .set_max_size(Vec2::new(PREVIEW_SIZE, PREVIEW_SIZE));
        if let Some(rgba_view) = preview_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::Display);
            rgba_view.set_zoom(1.0);
            rgba_view.set_show_transparency(true);
            rgba_view.set_background([22, 22, 24, 255]);
            rgba_view.set_buffer(TheRGBABuffer::new(TheDim::sized(
                PREVIEW_SIZE,
                PREVIEW_SIZE,
            )));
        }
        preview_canvas.set_layout(preview_layout);
        main_split.add_canvas(preview_canvas);

        let mut selector_canvas = TheCanvas::new();
        selector_canvas.limiter_mut().set_max_width(180);
        let mut selector_layout = TheVLayout::new(TheId::named(ORGANIC_BRUSH_SELECTOR_LAYOUT));
        selector_layout.set_background_color(None);
        selector_layout.set_padding(10);
        selector_layout.set_margin(Vec4::new(10, 10, 10, 10));
        selector_layout.set_alignment(TheHorizontalAlign::Center);
        for preset in OrganicPreset::all() {
            selector_layout.add_widget(Box::new(Self::preset_preview_view(preset)));
        }
        selector_canvas.set_layout(selector_layout);
        main_split.add_canvas(selector_canvas);
        main_canvas.set_layout(main_split);
        canvas.set_center(main_canvas);

        let mut settings_canvas = TheCanvas::new();
        settings_canvas.limiter_mut().set_max_width(360);
        let mut text_layout = TheTextLayout::new(TheId::named(ORGANIC_SETTINGS_LAYOUT));
        text_layout.limiter_mut().set_max_width(340);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        settings_canvas.set_layout(text_layout);
        canvas.set_right(settings_canvas);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.sync_settings_ui(ui, ctx, project, server_ctx);
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn default_state(&self) -> DockDefaultState {
        DockDefaultState::Minimized
    }

    fn maximized_state(&self) -> DockMaximizedState {
        DockMaximizedState::Maximized
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::Resize => {
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                false
            }
            TheEvent::Custom(id, _) if id.name == "Map Selection Changed" => {
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                false
            }
            TheEvent::IndexChanged(id, index) if id.name == ORGANIC_LOCK_MODE_DROPDOWN => {
                let Some(map) = project.get_map_mut(server_ctx) else {
                    return false;
                };
                map.properties
                    .set(PROP_LOCK_MODE, Value::Int((*index).clamp(0, 1) as i32));
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                true
            }
            TheEvent::NewListItemSelected(id, _)
                if OrganicPreset::from_widget_id(&id.name).is_some() =>
            {
                let Some(map) = project.get_map_mut(server_ctx) else {
                    return false;
                };
                let preset = OrganicPreset::from_widget_id(&id.name).unwrap_or(OrganicPreset::Moss);
                Self::apply_preset(map, preset);
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                true
            }
            TheEvent::ValueChanged(id, value) => {
                let Some(map) = project.get_map_mut(server_ctx) else {
                    return false;
                };
                let changed = match id.name.as_str() {
                    ORGANIC_COLOR_BASE => {
                        Self::set_int_property(map, PROP_PALETTE_1, value, 0, 255)
                    }
                    ORGANIC_COLOR_BORDER => {
                        Self::set_int_property(map, PROP_PALETTE_2, value, 0, 255)
                    }
                    ORGANIC_COLOR_NOISE => {
                        Self::set_int_property(map, PROP_PALETTE_3, value, 0, 255)
                    }
                    ORGANIC_BRUSH_SIZE => {
                        Self::set_float_property(map, PROP_RADIUS, value, 0.05, 4.0)
                    }
                    ORGANIC_BORDER_SIZE => {
                        let changed =
                            Self::set_float_property(map, PROP_BORDER_SIZE, value, 0.02, 0.48);
                        if let Some(border_size) = value.to_f32() {
                            map.properties.set(
                                PROP_SOFTNESS,
                                Value::Float((1.0 - border_size * 1.6).clamp(0.0, 1.0)),
                            );
                        }
                        changed
                    }
                    ORGANIC_NOISE_AMOUNT => {
                        Self::set_float_property(map, PROP_NOISE_STRENGTH, value, 0.0, 1.0)
                    }
                    ORGANIC_PAINT_MODE => Self::set_int_property(map, PROP_PAINT_MODE, value, 0, 1),
                    ORGANIC_OPACITY => {
                        let changed = Self::set_float_property(map, PROP_OPACITY, value, 0.05, 1.0);
                        if let Some(opacity) = value.to_f32() {
                            map.properties
                                .set(PROP_FLOW, Value::Float(opacity.clamp(0.05, 1.0)));
                        }
                        changed
                    }
                    _ => false,
                };
                if changed {
                    self.sync_settings_ui(ui, ctx, project, server_ctx);
                }
                changed
            }
            TheEvent::StateChanged(id, _) if id.name == ORGANIC_RENDER_TOGGLE_BUTTON => {
                self.toggle_render_active(project, ctx, server_ctx);
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                true
            }
            TheEvent::StateChanged(id, _) if id.name == ORGANIC_CLEAR_BUTTON => {
                self.clear_organic(project, ctx, server_ctx);
                self.sync_settings_ui(ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}

impl OrganicDock {
    fn sync_settings_ui(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let Some(map) = project.get_map(server_ctx) else {
            ctx.ui.relayout = true;
            return;
        };

        let preset = OrganicPreset::from_index(map.properties.get_int_default(PROP_PRESET, 0));
        let palette = PALETTE.read().unwrap().clone();
        let style = Self::brush_style_from_map(map, &palette);
        let render_active = map.properties.get_bool_default(PROP_RENDER_ACTIVE, true);
        let lock_mode = map
            .properties
            .get_int_default(PROP_LOCK_MODE, 0)
            .clamp(0, 1);

        OrganicTool::sync_render_active_to_vm(map);

        if let Some(dropdown) = ui.get_drop_down_menu(ORGANIC_LOCK_MODE_DROPDOWN) {
            dropdown.set_selected_index(lock_mode);
        }
        if let Some(widget) = ui.get_widget(ORGANIC_RENDER_TOGGLE_BUTTON) {
            widget.set_value(TheValue::Text(if render_active {
                fl!("organic_toggle_active")
            } else {
                fl!("organic_toggle_deactive")
            }));
            widget.set_status_text(&fl!("status_organic_toggle_visibility"));
        }

        if let Some(layout) = ui.get_rgba_layout(ORGANIC_PREVIEW_LAYOUT) {
            let buffer =
                Self::render_preview_buffer(preset, style, PREVIEW_SIZE, PREVIEW_SIZE, true);
            layout.set_buffer(buffer);
            layout.set_zoom(1.0);
        }

        let (preset_width, preset_height) = Self::preset_preview_size(ui);
        for brush_preset in OrganicPreset::all() {
            if let Some(widget) = ui.get_widget(brush_preset.widget_id()) {
                widget.set_state(if brush_preset.index() == preset.index() {
                    TheWidgetState::Selected
                } else {
                    TheWidgetState::None
                });
                widget
                    .limiter_mut()
                    .set_max_size(Vec2::new(preset_width, preset_height));
                widget
                    .limiter_mut()
                    .set_min_size(Vec2::new(preset_width, preset_height));
                if let Some(rgba_view) = widget.as_rgba_view() {
                    let preview_style = Self::preset_style(brush_preset, style);
                    rgba_view.set_buffer(Self::render_preview_buffer(
                        brush_preset,
                        preview_style,
                        preset_width,
                        preset_height,
                        false,
                    ));
                    rgba_view.set_zoom(1.0);
                }
            }
        }

        let Some(layout) = ui.get_text_layout(ORGANIC_SETTINGS_LAYOUT) else {
            return;
        };
        layout.clear();

        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::OpenTree("colors".into()));
        nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
            ORGANIC_COLOR_BASE.into(),
            "Base".into(),
            "Main fill color of the organic brush.".into(),
            map.properties.get_int_default(PROP_PALETTE_1, 4),
            palette.clone(),
        ));
        nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
            ORGANIC_COLOR_BORDER.into(),
            "Border".into(),
            "Outer outline color shown in the brush preview.".into(),
            map.properties.get_int_default(PROP_PALETTE_2, 8),
            palette.clone(),
        ));
        nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
            ORGANIC_COLOR_NOISE.into(),
            "Noise".into(),
            "Breakup speckle color shown around the brush edge.".into(),
            map.properties.get_int_default(PROP_PALETTE_3, 10),
            palette,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        nodeui.add_item(TheNodeUIItem::OpenTree("settings".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            ORGANIC_PAINT_MODE.into(),
            "Paint Mode".into(),
            "Choose whether the brush paints the full mask or only noise breakup.".into(),
            vec!["Full".into(), "Noise Only".into()],
            map.properties
                .get_int_default(PROP_PAINT_MODE, 0)
                .clamp(0, 1),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ORGANIC_BRUSH_SIZE.into(),
            "Brush Size".into(),
            "Overall brush size on the surface.".into(),
            map.properties.get_float_default(PROP_RADIUS, 0.6),
            0.05..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ORGANIC_BORDER_SIZE.into(),
            "Border Size".into(),
            "How thick the outline band is in the brush mask.".into(),
            map.properties
                .get_float_default(PROP_BORDER_SIZE, Self::default_border_size(preset)),
            0.02..=0.48,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ORGANIC_NOISE_AMOUNT.into(),
            "Noise Amount".into(),
            "How much irregular breakup is added across the brush.".into(),
            map.properties.get_float_default(PROP_NOISE_STRENGTH, 0.0),
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ORGANIC_OPACITY.into(),
            "Opacity".into(),
            "How strong the painted detail is applied.".into(),
            map.properties.get_float_default(
                PROP_OPACITY,
                map.properties.get_float_default(PROP_FLOW, 0.7),
            ),
            0.05..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        nodeui.apply_to_text_layout(layout);

        ctx.ui.relayout = true;
    }

    fn preset_preview_view(preset: OrganicPreset) -> TheRGBAView {
        let mut view = TheRGBAView::new(TheId::named(preset.widget_id()));
        view.set_mode(TheRGBAViewMode::Display);
        view.set_show_transparency(true);
        view.set_background([24, 24, 28, 255]);
        view.limiter_mut()
            .set_max_size(Vec2::new(PRESET_PREVIEW_WIDTH, PRESET_PREVIEW_HEIGHT));
        view.limiter_mut()
            .set_min_size(Vec2::new(PRESET_PREVIEW_WIDTH, PRESET_PREVIEW_HEIGHT));
        view
    }

    fn brush_style_from_map(map: &Map, palette: &ThePalette) -> BrushPreviewStyle {
        let preset = OrganicPreset::from_index(map.properties.get_int_default(PROP_PRESET, 0));
        BrushPreviewStyle {
            base: Self::palette_index_to_rgba(
                palette,
                map.properties.get_int_default(PROP_PALETTE_1, 4),
            ),
            border: Self::palette_index_to_rgba(
                palette,
                map.properties.get_int_default(PROP_PALETTE_2, 8),
            ),
            noise: Self::palette_index_to_rgba(
                palette,
                map.properties.get_int_default(PROP_PALETTE_3, 10),
            ),
            border_size: map
                .properties
                .get_float_default(PROP_BORDER_SIZE, Self::default_border_size(preset))
                .clamp(0.02, 0.48),
            noise_amount: map
                .properties
                .get_float_default(PROP_NOISE_STRENGTH, 0.0)
                .clamp(0.0, 1.0),
            opacity: map
                .properties
                .get_float_default(
                    PROP_OPACITY,
                    map.properties.get_float_default(PROP_FLOW, 0.7),
                )
                .clamp(0.05, 1.0),
            paint_mode: map
                .properties
                .get_int_default(PROP_PAINT_MODE, 0)
                .clamp(0, 1),
        }
    }

    fn preset_style(preset: OrganicPreset, current: BrushPreviewStyle) -> BrushPreviewStyle {
        BrushPreviewStyle {
            base: current.base,
            border: current.border,
            noise: current.noise,
            border_size: Self::default_border_size(preset),
            noise_amount: current.noise_amount,
            opacity: Self::default_opacity(preset),
            paint_mode: current.paint_mode,
        }
    }

    fn preset_preview_size(ui: &mut TheUI) -> (i32, i32) {
        let mut height = PRESET_PREVIEW_HEIGHT;
        let mut width = PRESET_PREVIEW_WIDTH;
        if let Some(layout) = ui.get_layout(ORGANIC_BRUSH_SELECTOR_LAYOUT) {
            let available_h = layout.dim().height.max(PRESET_PREVIEW_HEIGHT);
            let available_w = layout.dim().width.max(PRESET_PREVIEW_WIDTH);
            let target_h = ((available_h - 56) / OrganicPreset::all().len() as i32)
                .clamp(48, PRESET_PREVIEW_HEIGHT);
            height = target_h.max(32);
            width = ((height as f32 * 1.55).round() as i32)
                .clamp(76, PRESET_PREVIEW_WIDTH)
                .min((available_w - 20).max(76));
        }
        (width, height)
    }

    fn default_border_size(preset: OrganicPreset) -> f32 {
        match preset {
            OrganicPreset::Moss => 0.18,
            OrganicPreset::Mud => 0.14,
            OrganicPreset::Grime => 0.10,
            OrganicPreset::Bubbles => 0.20,
            OrganicPreset::Vines => 0.09,
        }
    }

    fn default_opacity(preset: OrganicPreset) -> f32 {
        match preset {
            OrganicPreset::Moss => 0.55,
            OrganicPreset::Mud => 0.82,
            OrganicPreset::Grime => 0.48,
            OrganicPreset::Bubbles => 0.44,
            OrganicPreset::Vines => 0.62,
        }
    }

    fn palette_index_to_rgba(palette: &ThePalette, index: i32) -> [u8; 4] {
        palette
            .colors
            .get(index.clamp(0, 255) as usize)
            .and_then(|entry| entry.clone())
            .unwrap_or(TheColor::from([0_u8, 0, 0, 255]))
            .to_u8_array()
    }

    fn render_preview_buffer(
        preset: OrganicPreset,
        style: BrushPreviewStyle,
        width: i32,
        height: i32,
        large: bool,
    ) -> TheRGBABuffer {
        let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
        buffer.fill([0, 0, 0, 0]);

        let bg = if large {
            [22, 24, 28, 255]
        } else {
            [18, 20, 24, 255]
        };
        let stride = buffer.stride();
        for y in 0..height.max(0) as usize {
            for x in 0..width.max(0) as usize {
                let index = (x + y * stride) * 4;
                buffer.pixels_mut()[index..index + 4].copy_from_slice(&bg);
            }
        }

        for y in 0..height.max(0) {
            for x in 0..width.max(0) {
                let uv = Vec2::new(
                    (x as f32 + 0.5) / width as f32 * 2.0 - 1.0,
                    (y as f32 + 0.5) / height as f32 * 2.0 - 1.0,
                );
                let (fill, border, noise) =
                    Self::sample_preset_shape(preset, uv, style.border_size);
                let mut pixel = bg;
                let opacity = style.opacity.clamp(0.05, 1.0);

                let noise_only = style.paint_mode == 1;
                if fill > 0.0 && !noise_only {
                    pixel = Self::blend_rgba(pixel, style.base, fill * opacity);
                }
                if border > 0.0 && !noise_only {
                    pixel = Self::blend_rgba(pixel, style.border, border * opacity);
                }
                if noise > 0.0 {
                    pixel = Self::blend_rgba(
                        pixel,
                        style.noise,
                        noise
                            * style.noise_amount.max(if noise_only { 0.35 } else { 0.0 })
                            * opacity
                            * 0.9,
                    );
                }

                let index = (x as usize + y as usize * stride) * 4;
                buffer.pixels_mut()[index..index + 4].copy_from_slice(&pixel);
            }
        }

        let outline = if large {
            [72, 77, 86, 255]
        } else {
            [58, 62, 70, 255]
        };
        let selected_outline = [186, 195, 210, 255];
        let stroke = if large { outline } else { selected_outline };
        let rect = (0, 0, width as usize, height as usize);
        TheDraw2D::new().rect_outline(buffer.pixels_mut(), &rect, stride, &stroke);
        buffer
    }

    fn sample_preset_shape(
        preset: OrganicPreset,
        uv: Vec2<f32>,
        border_size: f32,
    ) -> (f32, f32, f32) {
        match preset {
            OrganicPreset::Moss => Self::sample_blob(
                Vec2::new(uv.x * 0.95, uv.y * 1.05),
                0.70,
                border_size,
                13.0,
                29.0,
            ),
            OrganicPreset::Mud => {
                let warped = Vec2::new(uv.x * 1.22, uv.y * 0.74 + 0.08 * uv.x.sin());
                Self::sample_blob(warped, 0.76, border_size * 0.85, 9.0, 18.0)
            }
            OrganicPreset::Grime => {
                let fill =
                    Self::segment_mask(uv, Vec2::new(-0.72, 0.54), Vec2::new(0.68, -0.48), 0.18);
                let border = (fill
                    - Self::segment_mask(
                        uv,
                        Vec2::new(-0.72, 0.54),
                        Vec2::new(0.68, -0.48),
                        (0.18 - border_size * 0.4).max(0.02),
                    ))
                .clamp(0.0, 1.0);
                let noise = Self::speckle_mask(uv * 1.4, 9.0, 0.74) * fill;
                (fill.clamp(0.0, 1.0), border, noise * 0.9)
            }
            OrganicPreset::Bubbles => Self::sample_bubbles(uv, border_size),
            OrganicPreset::Vines => Self::sample_vines(uv, border_size),
        }
    }

    fn sample_blob(
        uv: Vec2<f32>,
        radius: f32,
        border_size: f32,
        noise_scale: f32,
        noise_seed: f32,
    ) -> (f32, f32, f32) {
        let wobble = (Self::hash2(
            Vec2::new(uv.x * noise_scale, uv.y * noise_scale),
            noise_seed,
        ) - 0.5)
            * 0.18;
        let dist = uv.magnitude() + wobble;
        let outer = radius;
        let inner = (outer - border_size.clamp(0.02, 0.48)).max(0.03);
        let fill = Self::smooth_band(dist, outer, 0.07);
        let inner_fill = Self::smooth_band(dist, inner, 0.06);
        let border = (fill - inner_fill).clamp(0.0, 1.0);
        let noise = Self::speckle_mask(uv, noise_scale * 0.8, 0.76) * fill.max(border * 0.45);
        (inner_fill, border, noise)
    }

    fn sample_bubbles(uv: Vec2<f32>, border_size: f32) -> (f32, f32, f32) {
        let centers = [
            (Vec2::new(-0.34, -0.12), 0.28),
            (Vec2::new(0.06, 0.04), 0.22),
            (Vec2::new(0.34, -0.18), 0.18),
            (Vec2::new(-0.02, -0.34), 0.14),
            (Vec2::new(0.24, 0.24), 0.11),
        ];
        let mut fill: f32 = 0.0;
        let mut border: f32 = 0.0;
        for (center, radius) in centers {
            let delta = uv - center;
            let outer = Self::smooth_band(delta.magnitude(), radius, 0.05);
            let inner = Self::smooth_band(
                delta.magnitude(),
                (radius - border_size * 0.45).max(0.02),
                0.04,
            );
            fill = fill.max(inner);
            border = border.max((outer - inner).clamp(0.0, 1.0));
        }
        let noise = Self::speckle_mask(uv * 1.1, 12.0, 0.82) * fill.max(border * 0.45);
        (fill, border, noise)
    }

    fn sample_vines(uv: Vec2<f32>, border_size: f32) -> (f32, f32, f32) {
        let segments = [
            (Vec2::new(-0.72, 0.64), Vec2::new(-0.08, 0.08), 0.10),
            (Vec2::new(-0.10, 0.08), Vec2::new(0.30, -0.18), 0.08),
            (Vec2::new(0.08, -0.02), Vec2::new(0.60, -0.54), 0.07),
            (Vec2::new(0.00, -0.02), Vec2::new(-0.38, -0.42), 0.06),
        ];
        let mut outer: f32 = 0.0;
        let mut inner: f32 = 0.0;
        for (a, b, width) in segments {
            outer = outer.max(Self::segment_mask(uv, a, b, width));
            inner = inner.max(Self::segment_mask(
                uv,
                a,
                b,
                (width - border_size * 0.35).max(0.015),
            ));
        }
        let leaf_1 = Self::sample_blob(
            uv - Vec2::new(0.28, -0.18),
            0.18,
            border_size * 0.6,
            8.0,
            41.0,
        );
        let leaf_2 = Self::sample_blob(
            uv - Vec2::new(-0.22, -0.28),
            0.16,
            border_size * 0.6,
            8.0,
            53.0,
        );
        let fill = outer.max(leaf_1.0).max(leaf_2.0);
        let border = (outer - inner).max(leaf_1.1).max(leaf_2.1).clamp(0.0, 1.0);
        let noise = Self::speckle_mask(uv * 1.8, 10.0, 0.78) * fill.max(border * 0.45);
        (fill, border, noise)
    }

    fn smooth_band(dist: f32, radius: f32, feather: f32) -> f32 {
        let t = ((radius - dist) / feather).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn segment_mask(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>, radius: f32) -> f32 {
        let pa = p - a;
        let ba = b - a;
        let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
        Self::smooth_band((pa - ba * h).magnitude(), radius, radius * 0.45)
    }

    fn speckle_mask(uv: Vec2<f32>, scale: f32, threshold: f32) -> f32 {
        let cell = Vec2::new((uv.x * scale).floor(), (uv.y * scale).floor());
        let value = Self::hash2(cell, 17.0);
        ((value - threshold) / (1.0 - threshold)).clamp(0.0, 1.0)
    }

    fn hash2(v: Vec2<f32>, seed: f32) -> f32 {
        let dot = v.x * 127.1 + v.y * 311.7 + seed * 17.13;
        (dot.sin() * 43_758.547).fract().abs()
    }

    fn blend_rgba(dst: [u8; 4], src: [u8; 4], alpha: f32) -> [u8; 4] {
        let a = alpha.clamp(0.0, 1.0);
        let inv = 1.0 - a;
        [
            (dst[0] as f32 * inv + src[0] as f32 * a).round() as u8,
            (dst[1] as f32 * inv + src[1] as f32 * a).round() as u8,
            (dst[2] as f32 * inv + src[2] as f32 * a).round() as u8,
            255,
        ]
    }

    fn set_float_property(map: &mut Map, key: &str, value: &TheValue, min: f32, max: f32) -> bool {
        let Some(value) = value.to_f32() else {
            return false;
        };
        map.properties.set(key, Value::Float(value.clamp(min, max)));
        true
    }

    fn set_int_property(map: &mut Map, key: &str, value: &TheValue, min: i32, max: i32) -> bool {
        let Some(value) = value.to_i32() else {
            return false;
        };
        map.properties.set(key, Value::Int(value.clamp(min, max)));
        true
    }

    fn apply_preset(map: &mut Map, preset: OrganicPreset) {
        map.properties.set(PROP_PRESET, Value::Int(preset.index()));
        match preset {
            OrganicPreset::Moss => {
                map.properties.set(PROP_SHAPE_MODE, Value::Int(0));
                map.properties.set(PROP_RADIUS, Value::Float(0.55));
                map.properties.set(PROP_FLOW, Value::Float(0.55));
                map.properties.set(PROP_JITTER, Value::Float(0.18));
                map.properties.set(PROP_DEPTH, Value::Float(0.16));
                map.properties.set(PROP_CELL_SIZE, Value::Float(0.05));
                map.properties.set(PROP_SOFTNESS, Value::Float(0.48));
                map.properties.set(PROP_SCATTER_COUNT, Value::Int(7));
                map.properties.set(PROP_SCATTER_JITTER, Value::Float(0.55));
                map.properties.set(PROP_HEIGHT_FALLOFF, Value::Float(0.82));
                map.properties.set(PROP_NOISE_SCALE, Value::Float(0.55));
                map.properties.set(PROP_NOISE_STRENGTH, Value::Float(0.22));
                map.properties.set(PROP_NOISE_SEED, Value::Int(11));
                map.properties.set(PROP_CHANNEL, Value::Int(0));
                map.properties.set(PROP_LINE_LENGTH, Value::Float(1.8));
                map.properties.set(PROP_LINE_WIDTH, Value::Float(0.22));
                map.properties.set(PROP_LINE_SOFTNESS, Value::Float(0.4));
            }
            OrganicPreset::Mud => {
                map.properties.set(PROP_SHAPE_MODE, Value::Int(0));
                map.properties.set(PROP_RADIUS, Value::Float(0.68));
                map.properties.set(PROP_FLOW, Value::Float(0.82));
                map.properties.set(PROP_JITTER, Value::Float(0.08));
                map.properties.set(PROP_DEPTH, Value::Float(0.10));
                map.properties.set(PROP_CELL_SIZE, Value::Float(0.06));
                map.properties.set(PROP_SOFTNESS, Value::Float(0.72));
                map.properties.set(PROP_SCATTER_COUNT, Value::Int(4));
                map.properties.set(PROP_SCATTER_JITTER, Value::Float(0.26));
                map.properties.set(PROP_HEIGHT_FALLOFF, Value::Float(0.88));
                map.properties.set(PROP_NOISE_SCALE, Value::Float(0.28));
                map.properties.set(PROP_NOISE_STRENGTH, Value::Float(0.12));
                map.properties.set(PROP_NOISE_SEED, Value::Int(17));
                map.properties.set(PROP_CHANNEL, Value::Int(1));
                map.properties.set(PROP_LINE_LENGTH, Value::Float(1.8));
                map.properties.set(PROP_LINE_WIDTH, Value::Float(0.22));
                map.properties.set(PROP_LINE_SOFTNESS, Value::Float(0.4));
            }
            OrganicPreset::Grime => {
                map.properties.set(PROP_SHAPE_MODE, Value::Int(1));
                map.properties.set(PROP_RADIUS, Value::Float(0.50));
                map.properties.set(PROP_FLOW, Value::Float(0.48));
                map.properties.set(PROP_JITTER, Value::Float(0.24));
                map.properties.set(PROP_DEPTH, Value::Float(0.08));
                map.properties.set(PROP_CELL_SIZE, Value::Float(0.05));
                map.properties.set(PROP_SOFTNESS, Value::Float(0.42));
                map.properties.set(PROP_SCATTER_COUNT, Value::Int(3));
                map.properties.set(PROP_SCATTER_JITTER, Value::Float(0.35));
                map.properties.set(PROP_HEIGHT_FALLOFF, Value::Float(0.70));
                map.properties.set(PROP_NOISE_SCALE, Value::Float(0.65));
                map.properties.set(PROP_NOISE_STRENGTH, Value::Float(0.32));
                map.properties.set(PROP_NOISE_SEED, Value::Int(23));
                map.properties.set(PROP_CHANNEL, Value::Int(2));
                map.properties.set(PROP_LINE_LENGTH, Value::Float(2.1));
                map.properties.set(PROP_LINE_WIDTH, Value::Float(0.18));
                map.properties.set(PROP_LINE_SOFTNESS, Value::Float(0.55));
            }
            OrganicPreset::Bubbles => {
                map.properties.set(PROP_SHAPE_MODE, Value::Int(0));
                map.properties.set(PROP_RADIUS, Value::Float(0.42));
                map.properties.set(PROP_FLOW, Value::Float(0.44));
                map.properties.set(PROP_JITTER, Value::Float(0.32));
                map.properties.set(PROP_DEPTH, Value::Float(0.09));
                map.properties.set(PROP_CELL_SIZE, Value::Float(0.04));
                map.properties.set(PROP_SOFTNESS, Value::Float(0.24));
                map.properties.set(PROP_SCATTER_COUNT, Value::Int(11));
                map.properties.set(PROP_SCATTER_JITTER, Value::Float(0.72));
                map.properties.set(PROP_HEIGHT_FALLOFF, Value::Float(0.55));
                map.properties.set(PROP_NOISE_SCALE, Value::Float(0.95));
                map.properties.set(PROP_NOISE_STRENGTH, Value::Float(0.26));
                map.properties.set(PROP_NOISE_SEED, Value::Int(29));
                map.properties.set(PROP_CHANNEL, Value::Int(1));
                map.properties.set(PROP_LINE_LENGTH, Value::Float(1.8));
                map.properties.set(PROP_LINE_WIDTH, Value::Float(0.22));
                map.properties.set(PROP_LINE_SOFTNESS, Value::Float(0.4));
            }
            OrganicPreset::Vines => {
                map.properties.set(PROP_SHAPE_MODE, Value::Int(1));
                map.properties.set(PROP_RADIUS, Value::Float(0.52));
                map.properties.set(PROP_FLOW, Value::Float(0.62));
                map.properties.set(PROP_JITTER, Value::Float(0.16));
                map.properties.set(PROP_DEPTH, Value::Float(0.13));
                map.properties.set(PROP_CELL_SIZE, Value::Float(0.04));
                map.properties.set(PROP_SOFTNESS, Value::Float(0.38));
                map.properties.set(PROP_SCATTER_COUNT, Value::Int(1));
                map.properties.set(PROP_SCATTER_JITTER, Value::Float(0.0));
                map.properties.set(PROP_HEIGHT_FALLOFF, Value::Float(0.72));
                map.properties.set(PROP_NOISE_SCALE, Value::Float(0.42));
                map.properties.set(PROP_NOISE_STRENGTH, Value::Float(0.18));
                map.properties.set(PROP_NOISE_SEED, Value::Int(31));
                map.properties.set(PROP_CHANNEL, Value::Int(0));
                map.properties.set(PROP_LINE_LENGTH, Value::Float(2.6));
                map.properties.set(PROP_LINE_WIDTH, Value::Float(0.14));
                map.properties.set(PROP_LINE_SOFTNESS, Value::Float(0.36));
            }
        }

        map.properties.set(
            PROP_BORDER_SIZE,
            Value::Float(Self::default_border_size(preset)),
        );
        map.properties
            .set(PROP_OPACITY, Value::Float(Self::default_opacity(preset)));
    }

    fn toggle_render_active(
        &self,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        let Some(map) = project.get_map_mut(server_ctx) else {
            return;
        };
        let prev = map.clone();
        let next = !map.properties.get_bool_default(PROP_RENDER_ACTIVE, true);
        map.properties.set(PROP_RENDER_ACTIVE, Value::Bool(next));
        map.changed += 1;
        OrganicTool::sync_render_active_to_vm(map);
        SCENEMANAGER.write().unwrap().update_map(map.clone());
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(prev), Box::new(map.clone())),
            ctx,
        );
    }

    fn clear_organic(
        &self,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        let Some(map) = project.get_map_mut(server_ctx) else {
            return;
        };
        let prev = map.clone();
        let changed = if map.properties.get_int_default(PROP_LOCK_MODE, 0) == 0 {
            Self::clear_all_organic(map)
        } else {
            Self::clear_locked_organic(map, server_ctx)
        };
        if !changed {
            return;
        }
        map.changed += 1;
        SCENEMANAGER.write().unwrap().update_map(map.clone());
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(prev), Box::new(map.clone())),
            ctx,
        );
    }

    fn clear_all_organic(map: &mut Map) -> bool {
        let mut changed = false;
        let terrain_tiles = OrganicTool::terrain_tiles_for_sync(&map.terrain_organic_layer);
        if !map.terrain_organic_layer.pages.is_empty() {
            map.terrain_organic_layer.pages.clear();
            changed = true;
        }
        for surface in map.surfaces.values_mut() {
            if !surface.organic_layers.is_empty() {
                surface.organic_layers.clear();
                changed = true;
            }
        }
        if changed {
            for surface_id in map.surfaces.keys().copied() {
                OrganicTool::sync_surface_detail_to_vm(map, surface_id);
            }
            for (tile_x, tile_z) in terrain_tiles {
                OrganicTool::sync_terrain_detail_to_vm(map, tile_x, tile_z);
            }
            OrganicTool::sync_render_active_to_vm(map);
        }
        changed
    }

    fn clear_locked_organic(map: &mut Map, server_ctx: &ServerContext) -> bool {
        let mut target_surface_ids = Vec::new();
        if let Some(surface) = server_ctx.active_detail_surface.as_ref() {
            target_surface_ids.push(surface.id);
        } else {
            for surface in map.surfaces.values() {
                if map.selected_sectors.contains(&surface.sector_id) {
                    target_surface_ids.push(surface.id);
                }
            }
        }
        target_surface_ids.sort_unstable();
        target_surface_ids.dedup();

        let mut changed = false;
        for surface_id in &target_surface_ids {
            if let Some(surface) = map.surfaces.get_mut(surface_id)
                && !surface.organic_layers.is_empty()
            {
                surface.organic_layers.clear();
                changed = true;
            }
        }
        if changed {
            for surface_id in target_surface_ids {
                OrganicTool::sync_surface_detail_to_vm(map, surface_id);
            }
            OrganicTool::sync_render_active_to_vm(map);
        }
        changed
    }
}
