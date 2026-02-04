use crate::prelude::*;

/// The display and interaction mode for TheRGBAView
#[derive(PartialEq, Clone, Debug)]
pub enum TheRGBAViewMode {
    /// Display-only mode with no interaction
    Display,
    /// Allow selecting multiple tiles
    TileSelection,
    /// Edit tiles with click/drag events
    TileEditor,
    /// Pick a single tile
    TilePicker,
}

/// A widget for displaying and interacting with RGBA buffers with zoom and grid support
pub struct TheRGBAView {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,
    background: RGBA,

    buffer: TheRGBABuffer,

    scroll_offset: Vec2<i32>,
    zoom: f32,

    grid: Option<i32>,
    grid_color: RGBA,
    dont_show_grid: bool,
    selected: FxHashSet<(i32, i32)>,
    used: FxHashSet<(i32, i32)>,
    selection_color: RGBA,
    used_color: RGBA,
    hover_color: Option<RGBA>,
    hover: Option<(i32, i32)>,
    drop: Option<(i32, i32)>,

    rectangular_selection: bool,
    rectangle_start: Option<(i32, i32)>,

    last_loc: (i32, i32),
    float_pos: Vec2<f32>,

    hscrollbar: TheId,
    vscrollbar: TheId,
    wheel_scale: f32,

    mode: TheRGBAViewMode,

    dim: TheDim,
    is_dirty: bool,

    mouse_down_pos: Vec2<i32>,

    accumulated_wheel_delta: Vec2<f32>,

    layout_id: TheId,

    context_menu: Option<TheContextMenu>,

    icon_mode: bool,

    supports_external_zoom: bool,
    zoom_modifier_down: bool,

    show_transparency: bool,
    transparency_color: RGBA,
}

impl TheRGBAView {
    /// Calculate the centered offset for the X axis when the buffer is smaller than the view
    #[inline]
    fn centered_offset_x(&self) -> f32 {
        if (self.zoom * self.buffer.dim().width as f32) < self.dim.width as f32 {
            (self.dim.width as f32 - self.zoom * self.buffer.dim().width as f32) / 2.0
        } else {
            0.0
        }
    }

    /// Calculate the centered offset for the Y axis when the buffer is smaller than the view
    #[inline]
    fn centered_offset_y(&self) -> f32 {
        if (self.zoom * self.buffer.dim().height as f32) < self.dim.height as f32 {
            (self.dim.height as f32 - self.zoom * self.buffer.dim().height as f32) / 2.0
        } else {
            0.0
        }
    }

    /// Convert screen coordinates to buffer coordinates (in pixels)
    #[inline]
    fn screen_to_buffer(&self, coord: Vec2<i32>) -> (f32, f32) {
        let centered_offset_x = self.centered_offset_x();
        let centered_offset_y = self.centered_offset_y();

        let source_x =
            ((coord.x as f32 - centered_offset_x) + self.scroll_offset.x as f32) / self.zoom;
        let source_y =
            ((coord.y as f32 - centered_offset_y) + self.scroll_offset.y as f32) / self.zoom;

        (source_x, source_y)
    }

    /// Blend a source color with alpha over a background color
    #[inline]
    fn blend_alpha(src: &[u8; 4], bg: &RGBA) -> [u8; 4] {
        let alpha = src[3] as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        [
            ((src[0] as f32 * alpha) + (bg[0] as f32 * inv_alpha)) as u8,
            ((src[1] as f32 * alpha) + (bg[1] as f32 * inv_alpha)) as u8,
            ((src[2] as f32 * alpha) + (bg[2] as f32 * inv_alpha)) as u8,
            255,
        ]
    }
}

impl TheWidget for TheRGBAView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(17);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,
            background: BLACK,

            buffer: TheRGBABuffer::empty(),
            scroll_offset: Vec2::new(0, 0),
            zoom: 1.0,

            grid: None,
            grid_color: [200, 200, 200, 255],
            dont_show_grid: false,
            selected: FxHashSet::default(),
            used: FxHashSet::default(),
            selection_color: [255, 255, 255, 180],
            used_color: [255, 255, 255, 100],
            hover_color: None,
            hover: None,
            drop: None,

            rectangular_selection: false,
            rectangle_start: None,

            last_loc: (0, 0),
            float_pos: Vec2::zero(),

            hscrollbar: TheId::empty(),
            vscrollbar: TheId::empty(),
            wheel_scale: -0.4,

            mode: TheRGBAViewMode::Display,

            dim: TheDim::zero(),
            is_dirty: true,

            mouse_down_pos: Vec2::zero(),

            accumulated_wheel_delta: Vec2::zero(),

            layout_id: TheId::empty(),

            context_menu: None,

            icon_mode: false,

            supports_external_zoom: false,
            zoom_modifier_down: false,

            show_transparency: false,
            transparency_color: [255, 0, 255, 255], // Magenta - a cool default that stands out
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_context_menu(&mut self, menu: Option<TheContextMenu>) {
        self.context_menu = menu;
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::ModifierChanged(_shift, ctrl, _alt, logo) => {
                self.zoom_modifier_down = *ctrl || *logo;
            }
            TheEvent::Context(coord) => {
                if let Some(context_menu) = &self.context_menu {
                    ctx.ui.send(TheEvent::ShowContextMenu(
                        self.id().clone(),
                        *coord,
                        context_menu.clone(),
                    ));
                }
            }
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Selected {
                    self.is_dirty = true;
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.send(TheEvent::NewListItemSelected(
                        self.id().clone(),
                        self.layout_id.clone(),
                    ));
                    redraw = true;
                }

                ctx.ui.set_focus(self.id());

                self.mouse_down_pos = *coord;

                if self.mode != TheRGBAViewMode::Display {
                    if let Some(loc) = self.get_grid_location(*coord) {
                        self.last_loc = loc;
                        if let Some(fgrid) = self.get_grid_location_f(*coord) {
                            self.float_pos = Vec2::new(fgrid.0, fgrid.1);
                        }
                        if self.mode == TheRGBAViewMode::TileSelection {
                            if self.rectangular_selection {
                                self.rectangle_start = Some(loc);
                                self.selected.clear();
                                self.selected.insert((loc.0, loc.1));
                            } else {
                                if self.selected.contains(&(loc.0, loc.1)) {
                                    self.selected.remove(&(loc.0, loc.1));
                                } else {
                                    self.selected.insert((loc.0, loc.1));
                                }
                                ctx.ui.send(TheEvent::TileSelectionChanged(self.id.clone()));
                            }
                        } else if self.mode == TheRGBAViewMode::TilePicker {
                            self.selected.clear();
                            self.selected.insert((loc.0, loc.1));
                            ctx.ui.send(TheEvent::TilePicked(
                                self.id.clone(),
                                Vec2::new(loc.0, loc.1),
                            ));
                        } else if self.mode == TheRGBAViewMode::TileEditor {
                            ctx.ui.send(TheEvent::TileEditorClicked(
                                self.id.clone(),
                                Vec2::new(loc.0, loc.1),
                            ));
                        }
                    }
                    redraw = true;
                }
            }
            TheEvent::MouseDragged(coord) => {
                if self.state != TheWidgetState::Selected {
                    self.is_dirty = true;
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.send(TheEvent::NewListItemSelected(
                        self.id().clone(),
                        self.layout_id.clone(),
                    ));
                    ctx.ui.set_focus(self.id());
                }

                // Check if we should start dragging in TilePicker mode
                if self.mode == TheRGBAViewMode::TilePicker && ctx.ui.drop.is_none() {
                    if Vec2::new(self.mouse_down_pos.x as f32, self.mouse_down_pos.y as f32)
                        .distance(Vec2::new(coord.x as f32, coord.y as f32))
                        >= 5.0
                    {
                        if let Some(loc) = self.get_grid_location(self.mouse_down_pos) {
                            ctx.ui.send(TheEvent::TileDragStarted(
                                self.id().clone(),
                                Vec2::new(loc.0, loc.1),
                                Vec2::new(loc.0 + coord.x, loc.1 + coord.y),
                            ));
                        }
                    }
                }

                if self.mode != TheRGBAViewMode::Display {
                    if let Some(loc) = self.get_grid_location(*coord) {
                        if loc != self.last_loc {
                            self.last_loc = loc;
                            if let Some(fgrid) = self.get_grid_location_f(*coord) {
                                self.float_pos = Vec2::new(fgrid.0, fgrid.1);
                            }
                            if self.mode == TheRGBAViewMode::TileSelection {
                                if self.rectangular_selection {
                                    if let Some(rectangle_start) = self.rectangle_start {
                                        let mut min_x = rectangle_start.0;
                                        let mut min_y = rectangle_start.1;
                                        let mut max_x = self.last_loc.0;
                                        let mut max_y = self.last_loc.1;

                                        if min_x > max_x {
                                            std::mem::swap(&mut min_x, &mut max_x);
                                        }
                                        if min_y > max_y {
                                            std::mem::swap(&mut min_y, &mut max_y);
                                        }

                                        self.selected.clear();

                                        for x in min_x..=max_x {
                                            for y in min_y..=max_y {
                                                self.selected.insert((x, y));
                                            }
                                        }
                                    }
                                } else {
                                    if self.selected.contains(&(loc.0, loc.1)) {
                                        self.selected.remove(&(loc.0, loc.1));
                                    } else {
                                        self.selected.insert((loc.0, loc.1));
                                    }
                                    ctx.ui.send(TheEvent::TileSelectionChanged(self.id.clone()));
                                }
                            } else if self.mode == TheRGBAViewMode::TilePicker {
                                self.selected.clear();
                                self.selected.insert((loc.0, loc.1));
                                ctx.ui.send(TheEvent::TilePicked(
                                    self.id.clone(),
                                    Vec2::new(loc.0, loc.1),
                                ));
                            } else if self.mode == TheRGBAViewMode::TileEditor {
                                ctx.ui.send(TheEvent::TileEditorDragged(
                                    self.id.clone(),
                                    Vec2::new(loc.0, loc.1),
                                ));
                                self.hover = Some((loc.0, loc.1));
                            }
                        }
                    }
                }

                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseUp(_id) => {
                self.drop = None;
                if self.mode == TheRGBAViewMode::TileSelection {
                    if self.rectangular_selection {
                        if let Some(rectangle_start) = self.rectangle_start {
                            let mut min_x = rectangle_start.0;
                            let mut min_y = rectangle_start.1;
                            let mut max_x = self.last_loc.0;
                            let mut max_y = self.last_loc.1;

                            if min_x > max_x {
                                std::mem::swap(&mut min_x, &mut max_x);
                            }
                            if min_y > max_y {
                                std::mem::swap(&mut min_y, &mut max_y);
                            }

                            for x in min_x..=max_x {
                                for y in min_y..=max_y {
                                    self.selected.insert((x, y));
                                }
                            }
                            ctx.ui.send(TheEvent::TileSelectionChanged(self.id.clone()));
                            self.rectangle_start = None;
                        }
                    }
                } else if self.mode == TheRGBAViewMode::TileEditor {
                    ctx.ui.send(TheEvent::TileEditorUp(self.id.clone()));
                }
            }
            TheEvent::LostHover(_id) => {
                if self.hover.is_some() {
                    self.hover = None;
                    self.drop = None;
                    redraw = true;
                }
            }
            TheEvent::Hover(coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                if self.mode == TheRGBAViewMode::TileEditor
                    || self.mode == TheRGBAViewMode::TilePicker
                    || self.mode == TheRGBAViewMode::TileSelection
                {
                    if let Some(grid) = self.grid {
                        let (source_x, source_y) = self.screen_to_buffer(*coord);
                        let source_x = source_x.floor() as i32;
                        let source_y = source_y.floor() as i32;

                        if source_x >= 0
                            && source_x < self.buffer.dim().width
                            && source_y >= 0
                            && source_y < self.buffer.dim().height
                        {
                            let grid_x = source_x / grid;
                            let grid_y = source_y / grid;

                            if grid_x * grid < self.buffer.dim().width
                                && grid_y * grid < self.buffer.dim().height
                                && Some((grid_x, grid_y)) != self.hover
                            {
                                self.hover = Some((grid_x, grid_y));
                                self.is_dirty = true;
                                ctx.ui.send(TheEvent::TileEditorHoverChanged(
                                    self.id.clone(),
                                    Vec2::new(grid_x, grid_y),
                                ));
                            }
                        }
                    }
                    redraw = true;
                }
            }
            TheEvent::DropPreview(coord, _drop) => {
                if self.mode == TheRGBAViewMode::TileEditor {
                    let loc = self.get_grid_location(*coord);
                    if loc != self.drop {
                        self.drop = loc;
                        redraw = true;
                    }
                }
            }
            TheEvent::Drop(coord, drop) => {
                if self.mode == TheRGBAViewMode::TileEditor {
                    if let Some(loc) = self.get_grid_location(*coord) {
                        ctx.ui.send(TheEvent::TileEditorDrop(
                            self.id.clone(),
                            Vec2::new(loc.0, loc.1),
                            drop.clone(),
                        ));
                    }
                }
                self.drop = None;
                redraw = true;
            }
            TheEvent::MouseWheel(delta) => {
                let scale_factor = self.wheel_scale * 1.0 / (self.zoom.powf(0.5));

                let aspect_ratio = self.buffer.dim().width as f32 / self.buffer.dim().height as f32;

                let scale_x = if aspect_ratio > 1.0 {
                    1.0 / aspect_ratio
                } else {
                    1.0
                };
                let scale_y = if aspect_ratio < 1.0 {
                    aspect_ratio
                } else {
                    1.0
                };

                if self.supports_external_zoom {
                    if self.zoom_modifier_down {
                        let delta = delta.y as f32 * scale_factor * scale_y;
                        ctx.ui.send(TheEvent::TileZoomBy(self.id.clone(), delta));
                        return false;
                    }
                }

                // Update accumulated deltas
                self.accumulated_wheel_delta.x += delta.x as f32 * scale_factor * scale_x;
                self.accumulated_wheel_delta.y += delta.y as f32 * scale_factor * scale_y;

                let minimum_delta_threshold = 2.0;

                // Check if accumulated deltas exceed the threshold
                if self.accumulated_wheel_delta.x.abs() > minimum_delta_threshold
                    || self.accumulated_wheel_delta.y.abs() > minimum_delta_threshold
                {
                    // Convert accumulated deltas to integer and reset
                    let d = Vec2::new(
                        self.accumulated_wheel_delta.x as i32,
                        self.accumulated_wheel_delta.y as i32,
                    );
                    self.accumulated_wheel_delta = Vec2::zero();

                    // Send scroll events
                    ctx.ui.send(TheEvent::ScrollBy(self.hscrollbar.clone(), d));
                    ctx.ui.send(TheEvent::ScrollBy(self.vscrollbar.clone(), d));
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) => {
                if !self.selected.is_empty() && self.mode == TheRGBAViewMode::TilePicker {
                    ctx.ui.send(TheEvent::TileEditorDelete(
                        self.id.clone(),
                        self.selected.clone(),
                    ));
                }
                if self.hover.is_some() && self.mode == TheRGBAViewMode::TileEditor {
                    let mut selected = self.selected.clone();
                    selected.clear();
                    selected.insert(self.hover.unwrap());
                    ctx.ui.send(TheEvent::TileEditorDelete(
                        self.id.clone(),
                        selected.clone(),
                    ));
                }
            }
            _ => {}
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, ctx: &mut TheContext) {
        if self.dim != dim {
            if self.dim.width != dim.width || self.dim.height != dim.height {
                ctx.ui.send(TheEvent::WidgetResized(self.id.clone(), dim));
            }
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn state(&self) -> TheWidgetState {
        self.state
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        /// Mix two colors together with linear interpolation
        fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                255,
            ]
        }

        let stride: usize = buffer.stride();

        if !self.buffer.is_valid() {
            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                &self.background,
            );
            return;
        }

        let target = buffer;
        let target_len = target.pixels().len();

        let src_width = self.buffer.dim().width as f32;
        let src_height = self.buffer.dim().height as f32;
        let target_width = self.dim().width as f32;
        let target_height = self.dim().height as f32;

        // Calculate the scaled dimensions of the source image
        let scaled_width = src_width * self.zoom;
        let scaled_height = src_height * self.zoom;

        // Calculate the offset to center the image
        let offset_x = if scaled_width < target_width {
            (target_width - scaled_width) / 2.0
        } else {
            -self.scroll_offset.x as f32
        };

        let offset_y = if scaled_height < target_height {
            (target_height - scaled_height) / 2.0
        } else {
            -self.scroll_offset.y as f32
        };

        // Loop over every pixel in the target buffer
        for target_y in 0..self.dim.height {
            for target_x in 0..self.dim.width {
                // Calculate the corresponding source coordinates with the offset
                let src_x = (target_x as f32 - offset_x) / self.zoom;
                let src_y = (target_y as f32 - offset_y) / self.zoom;

                // Calculate the index for the destination pixel
                let target_index = ((self.dim.buffer_y + target_y) * target.dim().width
                    + target_x
                    + self.dim.buffer_x) as usize
                    * 4;

                if target_index + 4 > target_len {
                    continue;
                }

                // TileSelection mode - use original grid drawing logic
                if !self.dont_show_grid && self.mode == TheRGBAViewMode::TileSelection {
                    if let Some(grid) = self.grid {
                        if src_x as i32 % grid == 0 || src_y as i32 % grid == 0 {
                            if self.rectangular_selection {
                                let (source_x, source_y) =
                                    self.screen_to_buffer(Vec2::new(target_x, target_y));
                                let source_x = source_x.round() as i32;
                                let source_y = source_y.round() as i32;

                                if source_x >= 0
                                    && source_x < self.buffer.dim().width
                                    && source_y >= 0
                                    && source_y < self.buffer.dim().height
                                {
                                    target.pixels_mut()[target_index..target_index + 4]
                                        .copy_from_slice(&self.grid_color);
                                    continue;
                                }
                            } else {
                                target.pixels_mut()[target_index..target_index + 4]
                                    .copy_from_slice(&self.grid_color);
                                continue;
                            }
                        }
                    }
                }

                if src_x >= 0.0 && src_x < src_width && src_y >= 0.0 && src_y < src_height {
                    // Perform nearest neighbor interpolation
                    let src_x = src_x as i32;
                    let src_y = src_y as i32;
                    let src_index = (src_y * self.buffer.stride() as i32 + src_x) as usize * 4;

                    // TileEditor mode - check if we should draw grid line instead of image
                    let mut draw_grid_line = false;
                    if !self.dont_show_grid && self.mode == TheRGBAViewMode::TileEditor {
                        if let Some(grid) = self.grid {
                            // Calculate which screen pixel within the zoomed grid cell we're drawing
                            // This ensures grid lines are always 1 screen pixel wide regardless of zoom
                            let grid_size_pixels = grid as f32 * self.zoom;

                            // Find position within the current grid cell (in screen pixels)
                            let pos_x = (target_x as f32 - offset_x) % grid_size_pixels;
                            let pos_y = (target_y as f32 - offset_y) % grid_size_pixels;

                            // Draw 1-pixel grid line at the start of each cell
                            // Handle negative modulo for offsets
                            let pos_x = if pos_x < 0.0 {
                                pos_x + grid_size_pixels
                            } else {
                                pos_x
                            };
                            let pos_y = if pos_y < 0.0 {
                                pos_y + grid_size_pixels
                            } else {
                                pos_y
                            };

                            // Draw grid lines at cell boundaries (left/top edges)
                            let at_grid_edge = pos_x < 1.0 || pos_y < 1.0;

                            // Also draw right and bottom borders at image bounds (in screen space)
                            // Check if the NEXT screen pixel would be outside the image
                            let next_src_x = (target_x as f32 + 1.0 - offset_x) / self.zoom;
                            let next_src_y = (target_y as f32 + 1.0 - offset_y) / self.zoom;
                            let at_right_edge =
                                (src_x as f32) < src_width && next_src_x >= src_width;
                            let at_bottom_edge =
                                (src_y as f32) < src_height && next_src_y >= src_height;

                            draw_grid_line = at_grid_edge || at_right_edge || at_bottom_edge;
                        }
                    }

                    if draw_grid_line {
                        target.pixels_mut()[target_index..target_index + 4]
                            .copy_from_slice(&self.grid_color);
                        continue;
                    }

                    let mut copy = true;
                    if let Some(grid) = self.grid {
                        // Drop Preview
                        if self.mode == TheRGBAViewMode::TileEditor
                            && self.drop == Some((src_x / grid, src_y / grid))
                        {
                            target.pixels_mut()[target_index..target_index + 4]
                                .copy_from_slice(&WHITE);
                            copy = false;
                        } else if self.icon_mode
                            && !self.selected.contains(&(src_x / grid, src_y / grid))
                        {
                            let s = self.buffer.pixels();
                            let c = &[
                                s[src_index] / 2,
                                s[src_index + 1] / 2,
                                s[src_index + 2] / 2,
                                s[src_index + 3],
                            ];
                            target.pixels_mut()[target_index..target_index + 4].copy_from_slice(c);
                            copy = false;
                        } else
                        // Selected
                        if !self.icon_mode
                            && self.selected.contains(&(src_x / grid, src_y / grid))
                        {
                            let s = self.buffer.pixels();
                            let c = &[
                                s[src_index],
                                s[src_index + 1],
                                s[src_index + 2],
                                s[src_index + 3],
                            ];
                            let m = mix_color(
                                c,
                                &self.selection_color,
                                self.selection_color[3] as f32 / 255.0,
                            );
                            target.pixels_mut()[target_index..target_index + 4].copy_from_slice(&m);
                            copy = false;
                        } else
                        // Used
                        if !self.icon_mode
                            && self.used.contains(&(src_x / grid, src_y / grid))
                        {
                            let s = self.buffer.pixels();
                            let c = &[
                                s[src_index],
                                s[src_index + 1],
                                s[src_index + 2],
                                s[src_index + 3],
                            ];
                            let m =
                                mix_color(c, &self.used_color, self.used_color[3] as f32 / 255.0);
                            target.pixels_mut()[target_index..target_index + 4].copy_from_slice(&m);
                            copy = false;
                        }
                        // Hover
                        else if !self.icon_mode {
                            if let Some(hover_color) = self.hover_color {
                                if self.hover == Some((src_x / grid, src_y / grid)) {
                                    let s = self.buffer.pixels();
                                    let c = &[
                                        s[src_index],
                                        s[src_index + 1],
                                        s[src_index + 2],
                                        s[src_index + 3],
                                    ];
                                    let m =
                                        mix_color(c, &hover_color, hover_color[3] as f32 / 255.0);
                                    target.pixels_mut()[target_index..target_index + 4]
                                        .copy_from_slice(&m);
                                    copy = false;
                                }
                            }
                        }
                    }

                    // Copy the pixel from the source buffer to the target buffer
                    if copy {
                        let src_pixel = &self.buffer.pixels()[src_index..src_index + 4];

                        // If transparency is enabled and the pixel has alpha < 255, blend with solid color
                        if self.show_transparency && src_pixel[3] < 255 {
                            let blended = Self::blend_alpha(
                                &[src_pixel[0], src_pixel[1], src_pixel[2], src_pixel[3]],
                                &self.transparency_color,
                            );
                            target.pixels_mut()[target_index..target_index + 4]
                                .copy_from_slice(&blended);
                        } else {
                            target.pixels_mut()[target_index..target_index + 4]
                                .copy_from_slice(src_pixel);
                        }
                    }
                } else {
                    // Set the pixel to black if it's out of the source bounds
                    // target.pixels_mut()[target_index..target_index + 4].fill(0);
                    target.pixels_mut()[target_index..target_index + 4]
                        .copy_from_slice(&self.background);
                }
            }
        }

        if Some(self.id.clone()) == ctx.ui.focus {
            let tuple = self.dim().to_buffer_utuple();
            ctx.draw.rect_outline(
                target.pixels_mut(),
                &tuple,
                stride,
                style.theme().color(DefaultSelection),
            );
        }

        self.is_dirty = false;
    }

    fn as_rgba_view(&mut self) -> Option<&mut dyn TheRGBAViewTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheRGBAViewTrait: TheWidget {
    fn get_grid_location(&self, coord: Vec2<i32>) -> Option<(i32, i32)>;
    fn get_grid_location_f(&self, coord: Vec2<i32>) -> Option<(f32, f32)>;

    fn float_pos(&self) -> Vec2<f32>;

    fn buffer(&self) -> &TheRGBABuffer;
    fn buffer_mut(&mut self) -> &mut TheRGBABuffer;
    fn set_buffer(&mut self, buffer: TheRGBABuffer);
    fn set_background(&mut self, color: RGBA);
    fn zoom(&self) -> f32;
    fn set_zoom(&mut self, zoom: f32);
    fn visible_rect(&mut self) -> TheDim;
    fn set_scroll_offset(&mut self, offset: Vec2<i32>);
    fn grid(&self) -> Option<i32>;
    fn set_grid(&mut self, grid: Option<i32>);
    fn set_grid_color(&mut self, color: RGBA);
    fn set_dont_show_grid(&mut self, dont_show_grid: bool);
    fn set_selection_color(&mut self, color: RGBA);
    fn set_used_color(&mut self, color: RGBA);
    fn set_wheel_scale(&mut self, wheel_scale: f32);
    fn set_hover_color(&mut self, color: Option<RGBA>);
    fn set_scrollbar_ids(&mut self, hscrollbar: TheId, vscrollbar: TheId);
    fn set_icon_mode(&mut self, icon_mode: bool);

    fn set_associated_layout(&mut self, id: TheId);

    fn selection(&self) -> FxHashSet<(i32, i32)>;
    fn selection_as_dim(&self) -> TheDim;
    fn selection_sorted(&self) -> Vec<(i32, i32)>;
    fn selection_as_sequence(&self) -> TheRGBARegionSequence;
    fn selection_as_tile(&self) -> TheRGBATile;
    fn set_selection(&mut self, selection: FxHashSet<(i32, i32)>);
    fn set_used(&mut self, selection: FxHashSet<(i32, i32)>);
    fn set_mode(&mut self, mode: TheRGBAViewMode);
    fn set_rectangular_selection(&mut self, rectangular_selection: bool);

    fn set_supports_external_zoom(&mut self, zoom: bool);

    fn set_show_transparency(&mut self, show: bool);
    fn show_transparency(&self) -> bool;
    fn set_transparency_color(&mut self, color: RGBA);
}

impl TheRGBAViewTrait for TheRGBAView {
    fn set_rectangular_selection(&mut self, rectangular_selection: bool) {
        self.rectangular_selection = rectangular_selection;
    }
    fn get_grid_location(&self, coord: Vec2<i32>) -> Option<(i32, i32)> {
        if let Some(grid) = self.grid {
            let (source_x, source_y) = self.screen_to_buffer(coord);

            // Use floor for consistent grid location regardless of where in the pixel you click
            let source_x = source_x.floor() as i32;
            let source_y = source_y.floor() as i32;

            if source_x >= 0
                && source_x < self.buffer.dim().width
                && source_y >= 0
                && source_y < self.buffer.dim().height
            {
                let grid_x = source_x / grid;
                let grid_y = source_y / grid;

                if grid_x * grid < self.buffer.dim().width
                    && grid_y * grid < self.buffer.dim().height
                {
                    return Some((grid_x, grid_y));
                }
            }
        }
        None
    }

    fn get_grid_location_f(&self, coord: Vec2<i32>) -> Option<(f32, f32)> {
        if let Some(grid) = self.grid {
            let (source_x, source_y) = self.screen_to_buffer(coord);

            if source_x >= 0.0
                && source_x < self.buffer.dim().width as f32
                && source_y >= 0.0
                && source_y < self.buffer.dim().height as f32
            {
                let grid_x = source_x / grid as f32;
                let grid_y = source_y / grid as f32;

                return Some((grid_x, grid_y));
            }
        }
        None
    }

    fn float_pos(&self) -> Vec2<f32> {
        self.float_pos
    }

    fn buffer(&self) -> &TheRGBABuffer {
        &self.buffer
    }
    fn buffer_mut(&mut self) -> &mut TheRGBABuffer {
        &mut self.buffer
    }
    fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        self.buffer = buffer;
        self.is_dirty = true;
    }
    fn set_background(&mut self, color: RGBA) {
        self.background = color;
    }
    fn set_scrollbar_ids(&mut self, hscrollbar: TheId, vscrollbar: TheId) {
        self.hscrollbar = hscrollbar;
        self.vscrollbar = vscrollbar;
    }
    fn zoom(&self) -> f32 {
        self.zoom
    }
    fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
        self.is_dirty = true;
    }
    fn visible_rect(&mut self) -> TheDim {
        TheDim::new(
            (self.scroll_offset.x as f32 / self.zoom()) as i32,
            ((self.scroll_offset.y as f32) / self.zoom()) as i32,
            self.dim.width,
            self.dim.height,
        )
    }
    fn set_scroll_offset(&mut self, offset: Vec2<i32>) {
        self.scroll_offset = offset;
    }
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }
    fn grid(&self) -> Option<i32> {
        self.grid
    }
    fn set_grid(&mut self, grid: Option<i32>) {
        self.grid = grid;
    }
    fn set_grid_color(&mut self, color: RGBA) {
        self.grid_color = color;
    }
    fn set_dont_show_grid(&mut self, dont_show_grid: bool) {
        self.dont_show_grid = dont_show_grid;
        self.is_dirty = true;
    }
    fn set_selection_color(&mut self, color: RGBA) {
        self.selection_color = color;
        self.is_dirty = true;
    }
    fn set_used_color(&mut self, color: RGBA) {
        self.used_color = color;
        self.is_dirty = true;
    }
    fn set_wheel_scale(&mut self, wheel_scale: f32) {
        self.wheel_scale = wheel_scale;
    }
    fn set_hover_color(&mut self, color: Option<RGBA>) {
        self.hover_color = color;
        self.is_dirty = true;
    }
    fn set_icon_mode(&mut self, icon_mode: bool) {
        self.icon_mode = icon_mode;
    }
    fn selection(&self) -> FxHashSet<(i32, i32)> {
        self.selected.clone()
    }
    fn selection_as_dim(&self) -> TheDim {
        let (mut min_x, mut min_y) = (i32::MAX, i32::MAX);
        let (mut max_x, mut max_y) = (i32::MIN, i32::MIN);

        for &(x, y) in &self.selected {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;

        TheDim::new(min_x, min_y, width, height)
    }
    fn selection_sorted(&self) -> Vec<(i32, i32)> {
        let mut vec: Vec<(i32, i32)> = self.selected.clone().into_iter().collect();
        vec.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        vec
    }
    fn selection_as_sequence(&self) -> TheRGBARegionSequence {
        let mut sequence = TheRGBARegionSequence::new();
        let sorted = self.selection_sorted();
        if let Some(grid) = self.grid {
            for s in sorted {
                sequence.regions.push(TheRGBARegion::new(
                    (s.0 * grid) as usize,
                    (s.1 * grid) as usize,
                    grid as usize,
                    grid as usize,
                ))
            }
        }
        sequence
    }
    fn selection_as_tile(&self) -> TheRGBATile {
        let sequence = self.selection_as_sequence();
        let buffer = self.buffer.extract_sequence(&sequence);
        let mut tile = TheRGBATile::new();
        tile.buffer = buffer;
        tile
    }
    fn set_selection(&mut self, selection: FxHashSet<(i32, i32)>) {
        self.selected = selection;
        self.is_dirty = true;
    }
    fn set_used(&mut self, used: FxHashSet<(i32, i32)>) {
        self.used = used;
        self.is_dirty = true;
    }
    fn set_mode(&mut self, mode: TheRGBAViewMode) {
        self.mode = mode;
    }

    fn set_supports_external_zoom(&mut self, zoom: bool) {
        self.supports_external_zoom = zoom;
    }

    fn set_show_transparency(&mut self, show: bool) {
        if self.show_transparency != show {
            self.show_transparency = show;
            self.is_dirty = true;
        }
    }

    fn show_transparency(&self) -> bool {
        self.show_transparency
    }

    fn set_transparency_color(&mut self, color: RGBA) {
        self.transparency_color = color;
        if self.show_transparency {
            self.is_dirty = true;
        }
    }
}
