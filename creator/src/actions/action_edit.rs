use crate::prelude::*;

/// A complete editor display optionally owned by the currently selected action.
///
/// The display is deliberately data-oriented. Persistent parameters remain on the action while
/// hover, drag, pan, and zoom state live in [`ActionEditSession`]. Additional full-display editors
/// (walls, paths, curves, and so on) can be added without teaching the map or Prefab editors about
/// the action which requested them.
#[derive(Clone, Debug, PartialEq)]
pub enum ActionEditDisplay {
    Profile2D(ActionProfile2D),
}

/// Editable side profile used by revolve and future path/profile based actions.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionProfile2D {
    pub title: String,
    pub points: Vec<Vec2<f32>>,
    pub presets: Vec<ActionProfilePreset>,
    pub axis_x: f32,
    pub grid_step: f32,
    pub minimum_points: usize,
    pub mirror_preview: bool,
}

impl ActionProfile2D {
    pub fn new(title: impl Into<String>, points: Vec<Vec2<f32>>) -> Self {
        Self {
            title: title.into(),
            points,
            presets: Vec::new(),
            axis_x: 0.0,
            grid_step: 0.25,
            minimum_points: 2,
            mirror_preview: true,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.len() >= self.minimum_points
            && self.grid_step.is_finite()
            && self.grid_step > 0.0
            && self.axis_x.is_finite()
            && Self::points_are_valid(&self.points, self.minimum_points, self.axis_x)
            && self.presets.iter().all(|preset| {
                Self::points_are_valid(&preset.points, self.minimum_points, self.axis_x)
            })
    }

    fn points_are_valid(points: &[Vec2<f32>], minimum_points: usize, axis_x: f32) -> bool {
        points.len() >= minimum_points
            && points
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite() && point.x >= axis_x)
    }

    pub fn dimensions(&self) -> ActionProfileDimensions {
        let min_y = self
            .points
            .iter()
            .map(|point| point.y)
            .reduce(f32::min)
            .unwrap_or(0.0);
        let max_y = self
            .points
            .iter()
            .map(|point| point.y)
            .reduce(f32::max)
            .unwrap_or(0.0);
        let radius = self
            .points
            .iter()
            .map(|point| (point.x - self.axis_x).max(0.0))
            .fold(0.0, f32::max);
        ActionProfileDimensions {
            min_y,
            max_y,
            radius,
            diameter: radius * 2.0,
            height: max_y - min_y,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionProfilePreset {
    pub name: String,
    pub points: Vec<Vec2<f32>>,
}

impl ActionProfilePreset {
    pub fn new(name: impl Into<String>, points: Vec<Vec2<f32>>) -> Self {
        Self {
            name: name.into(),
            points,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionProfileDimensions {
    pub min_y: f32,
    pub max_y: f32,
    pub radius: f32,
    pub diameter: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
struct ProfileTransform {
    origin: Vec2<f32>,
    scale: f32,
}

impl ProfileTransform {
    fn world_to_screen(self, point: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(
            self.origin.x + point.x * self.scale,
            self.origin.y - point.y * self.scale,
        )
    }

    fn screen_to_world(self, point: Vec2<i32>) -> Vec2<f32> {
        Vec2::new(
            (point.x as f32 - self.origin.x) / self.scale,
            (self.origin.y - point.y as f32) / self.scale,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionEditSessionResult {
    Ignored,
    Handled,
    DisplayChanged(ActionEditDisplay),
    GridSubdivisionChanged(f32),
    Commit,
    Cancel,
}

/// Shared transient host for an action-owned full editor display.
#[derive(Clone, Debug)]
pub struct ActionEditSession {
    pub action_id: Uuid,
    pub project_context: ProjectContext,
    original: ActionEditDisplay,
    working: ActionEditDisplay,
    selected_point: Option<usize>,
    hovered_point: Option<usize>,
    dragging: bool,
    dragging_profile_scale: bool,
    profile_scale: f32,
    zoom: f32,
    pan: Vec2<f32>,
    grid_subdivisions: Option<f32>,
    hover_coord: Option<Vec2<i32>>,
    last_view_width: i32,
    last_transform: Option<ProfileTransform>,
}

impl ActionEditSession {
    const GRID_SUBDIVISIONS: [f32; 6] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
    const GRID_BUTTON_X: i32 = 86;
    const GRID_BUTTON_Y: i32 = 10;
    const GRID_BUTTON_SIZE: i32 = 22;
    const SCALE_TRACK_X: i32 = 374;
    const SCALE_TRACK_Y: i32 = 13;
    const SCALE_TRACK_WIDTH: i32 = 140;
    const SCALE_TRACK_HEIGHT: i32 = 16;
    const MIN_PROFILE_SCALE: f32 = 0.125;
    const MAX_PROFILE_SCALE: f32 = 8.0;
    const PRESET_BUTTON_X: i32 = 14;
    const PRESET_BUTTON_Y: i32 = 72;
    const PRESET_BUTTON_WIDTH: i32 = 76;
    const PRESET_BUTTON_HEIGHT: i32 = 24;

    pub fn new(
        action_id: Uuid,
        project_context: ProjectContext,
        display: ActionEditDisplay,
    ) -> Self {
        Self {
            action_id,
            project_context,
            original: display.clone(),
            working: display,
            selected_point: None,
            hovered_point: None,
            dragging: false,
            dragging_profile_scale: false,
            profile_scale: 1.0,
            zoom: 1.0,
            pan: Vec2::zero(),
            grid_subdivisions: None,
            hover_coord: None,
            last_view_width: 0,
            last_transform: None,
        }
    }

    pub fn original_display(&self) -> &ActionEditDisplay {
        &self.original
    }

    pub fn working_display(&self) -> &ActionEditDisplay {
        &self.working
    }

    pub fn is_for(&self, action_id: Uuid, project_context: ProjectContext) -> bool {
        self.action_id == action_id && self.project_context == project_context
    }

    fn profile(&self) -> &ActionProfile2D {
        match &self.working {
            ActionEditDisplay::Profile2D(profile) => profile,
        }
    }

    fn profile_mut(&mut self) -> &mut ActionProfile2D {
        match &mut self.working {
            ActionEditDisplay::Profile2D(profile) => profile,
        }
    }

    fn original_profile(&self) -> &ActionProfile2D {
        match &self.original {
            ActionEditDisplay::Profile2D(profile) => profile,
        }
    }

    pub fn set_grid_subdivisions(&mut self, subdivisions: f32) {
        self.grid_subdivisions = Some(subdivisions);
    }

    fn effective_grid_step(&self) -> f32 {
        self.grid_subdivisions
            .map(ServerContext::edit_grid_step)
            .unwrap_or_else(|| self.profile().grid_step)
            .max(0.0001)
    }

    fn grid_button_rect(index: usize) -> TheDim {
        TheDim::rect(
            Self::GRID_BUTTON_X + index as i32 * Self::GRID_BUTTON_SIZE,
            Self::GRID_BUTTON_Y,
            Self::GRID_BUTTON_SIZE,
            Self::GRID_BUTTON_SIZE,
        )
    }

    fn grid_subdivision_at(coord: Vec2<i32>) -> Option<f32> {
        Self::GRID_SUBDIVISIONS
            .iter()
            .enumerate()
            .find_map(|(index, subdivision)| {
                Self::grid_button_rect(index)
                    .contains(coord)
                    .then_some(*subdivision)
            })
    }

    fn preset_button_rect(index: usize) -> TheDim {
        TheDim::rect(
            Self::PRESET_BUTTON_X,
            Self::PRESET_BUTTON_Y + index as i32 * (Self::PRESET_BUTTON_HEIGHT + 4),
            Self::PRESET_BUTTON_WIDTH,
            Self::PRESET_BUTTON_HEIGHT,
        )
    }

    fn preset_at(&self, coord: Vec2<i32>) -> Option<usize> {
        self.profile()
            .presets
            .iter()
            .enumerate()
            .find_map(|(index, _)| {
                Self::preset_button_rect(index)
                    .contains(coord)
                    .then_some(index)
            })
    }

    fn apply_profile_preset(&mut self, index: usize) -> bool {
        let Some(points) = self
            .profile()
            .presets
            .get(index)
            .map(|preset| preset.points.clone())
        else {
            return false;
        };
        if self.profile().points == points {
            return false;
        }
        self.profile_mut().points = points;
        self.profile_scale = 1.0;
        self.selected_point = None;
        self.hovered_point = None;
        self.dragging = false;
        self.dragging_profile_scale = false;
        true
    }

    fn scale_track_rect() -> TheDim {
        TheDim::rect(
            Self::SCALE_TRACK_X,
            Self::SCALE_TRACK_Y,
            Self::SCALE_TRACK_WIDTH,
            Self::SCALE_TRACK_HEIGHT,
        )
    }

    fn scale_from_slider_x(x: i32) -> f32 {
        let normalized =
            ((x - Self::SCALE_TRACK_X) as f32 / Self::SCALE_TRACK_WIDTH as f32).clamp(0.0, 1.0);
        // A logarithmic slider gives equal room to shrinking and enlarging around 100%.
        2.0_f32.powf((normalized - 0.5) * 6.0)
    }

    fn slider_x_from_scale(scale: f32) -> i32 {
        let normalized = (scale
            .clamp(Self::MIN_PROFILE_SCALE, Self::MAX_PROFILE_SCALE)
            .log2()
            / 6.0
            + 0.5)
            .clamp(0.0, 1.0);
        Self::SCALE_TRACK_X + (normalized * Self::SCALE_TRACK_WIDTH as f32).round() as i32
    }

    fn set_profile_scale(&mut self, scale: f32) -> bool {
        let scale = scale.clamp(Self::MIN_PROFILE_SCALE, Self::MAX_PROFILE_SCALE);
        if (scale - self.profile_scale).abs() <= f32::EPSILON {
            return false;
        }
        let ratio = scale / self.profile_scale;
        let dimensions = self.profile().dimensions();
        let axis_x = self.profile().axis_x;
        for point in &mut self.profile_mut().points {
            point.x = axis_x + (point.x - axis_x) * ratio;
            point.y = dimensions.min_y + (point.y - dimensions.min_y) * ratio;
        }
        self.profile_scale = scale;
        true
    }

    fn scale_profile_from_slider(&mut self, x: i32) -> bool {
        self.set_profile_scale(Self::scale_from_slider_x(x))
    }

    fn profile_transform(&self, width: i32, height: i32) -> ProfileTransform {
        let profile = self.profile();
        // Fit once against the session's original profile. This stable world scale makes a whole
        // profile resize visible instead of immediately auto-fitting it back to the same size.
        let fit_profile = self.original_profile();
        let grid_step = self.effective_grid_step();
        // Keep the auto-fit profile clear of the shared snap bar, summary, and dimension lines.
        let padding = 78.0;
        let usable_width = (width as f32 - padding * 2.0).max(32.0);
        let usable_height = (height as f32 - padding * 2.0).max(32.0);

        let fit_dimensions = fit_profile.dimensions();
        let max_radius = fit_dimensions.radius.max(grid_step);
        let min_y = fit_dimensions.min_y;
        let max_y = fit_dimensions.max_y;
        let profile_width = if profile.mirror_preview {
            max_radius * 2.0
        } else {
            max_radius
        }
        .max(grid_step);
        let profile_height = (max_y - min_y).max(grid_step);
        let fit_scale = (usable_width / profile_width)
            .min(usable_height / profile_height)
            .clamp(4.0, 640.0);
        let scale = (fit_scale * self.zoom).clamp(2.0, 1600.0);
        let center_y = (min_y + max_y) * 0.5;

        ProfileTransform {
            origin: Vec2::new(
                width as f32 * 0.5 - profile.axis_x * scale + self.pan.x,
                height as f32 * 0.5 + center_y * scale + self.pan.y,
            ),
            scale,
        }
    }

    fn nearest_point(&self, coord: Vec2<i32>, max_distance: f32) -> Option<usize> {
        let transform = self.last_transform?;
        let coord = Vec2::new(coord.x as f32, coord.y as f32);
        self.profile()
            .points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| {
                let distance = (transform.world_to_screen(*point) - coord).magnitude();
                (distance <= max_distance).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    fn nearest_segment(&self, coord: Vec2<i32>, max_distance: f32) -> Option<usize> {
        let transform = self.last_transform?;
        let coord = Vec2::new(coord.x as f32, coord.y as f32);
        self.profile()
            .points
            .windows(2)
            .enumerate()
            .filter_map(|(index, segment)| {
                let a = transform.world_to_screen(segment[0]);
                let b = transform.world_to_screen(segment[1]);
                let ab = b - a;
                let length_squared = ab.magnitude_squared();
                if length_squared <= f32::EPSILON {
                    return None;
                }
                let t = ((coord - a).dot(ab) / length_squared).clamp(0.0, 1.0);
                let distance = (a + ab * t - coord).magnitude();
                (distance <= max_distance).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    fn snapped_profile_point(&self, coord: Vec2<i32>) -> Option<Vec2<f32>> {
        let transform = self.last_transform?;
        let profile = self.profile();
        let step = self.effective_grid_step();
        let point = transform.screen_to_world(coord);
        Some(Vec2::new(
            ((point.x - profile.axis_x) / step).round() * step + profile.axis_x,
            (point.y / step).round() * step,
        ))
    }

    fn move_selected_point(&mut self, coord: Vec2<i32>) -> bool {
        let Some(index) = self.selected_point else {
            return false;
        };
        let axis_x = self.profile().axis_x;
        let Some(mut point) = self.snapped_profile_point(coord) else {
            return false;
        };
        point.x = point.x.max(axis_x);
        if self.profile().points.get(index) == Some(&point) {
            return false;
        }
        self.profile_mut().points[index] = point;
        true
    }

    fn insert_point(&mut self, segment_index: usize, coord: Vec2<i32>) -> bool {
        let axis_x = self.profile().axis_x;
        let Some(mut point) = self.snapped_profile_point(coord) else {
            return false;
        };
        point.x = point.x.max(axis_x);
        let insert_at = segment_index + 1;
        self.profile_mut().points.insert(insert_at, point);
        self.selected_point = Some(insert_at);
        self.dragging = true;
        true
    }

    fn delete_selected_point(&mut self) -> bool {
        let Some(index) = self.selected_point else {
            return false;
        };
        if self.profile().points.len() <= self.profile().minimum_points {
            return false;
        }
        self.profile_mut().points.remove(index);
        self.selected_point = None;
        self.hovered_point = None;
        self.dragging = false;
        true
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        render_view_name: &str,
    ) -> ActionEditSessionResult {
        self.handle_event_with_modifiers(event, render_view_name, false)
    }

    pub fn handle_event_with_modifiers(
        &mut self,
        event: &TheEvent,
        render_view_name: &str,
        command_zoom: bool,
    ) -> ActionEditSessionResult {
        let is_target = |id: &TheId| id.name == render_view_name;
        match event {
            TheEvent::RenderViewClicked(id, coord) if is_target(id) => {
                if let Some(index) = self.preset_at(*coord) {
                    return if self.apply_profile_preset(index) {
                        ActionEditSessionResult::DisplayChanged(self.working.clone())
                    } else {
                        ActionEditSessionResult::Handled
                    };
                }
                if self.last_view_width >= 580 && Self::scale_track_rect().contains(*coord) {
                    self.dragging_profile_scale = true;
                    self.dragging = false;
                    return if self.scale_profile_from_slider(coord.x) {
                        ActionEditSessionResult::DisplayChanged(self.working.clone())
                    } else {
                        ActionEditSessionResult::Handled
                    };
                }
                if let Some(subdivision) = Self::grid_subdivision_at(*coord) {
                    self.set_grid_subdivisions(subdivision);
                    return ActionEditSessionResult::GridSubdivisionChanged(subdivision);
                }
                if let Some(index) = self.nearest_point(*coord, 12.0) {
                    self.selected_point = Some(index);
                    self.dragging = true;
                    return ActionEditSessionResult::Handled;
                }
                if let Some(segment) = self.nearest_segment(*coord, 10.0)
                    && self.insert_point(segment, *coord)
                {
                    return ActionEditSessionResult::DisplayChanged(self.working.clone());
                }
                self.selected_point = None;
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewDragged(id, coord) if is_target(id) => {
                if self.dragging_profile_scale && self.scale_profile_from_slider(coord.x) {
                    ActionEditSessionResult::DisplayChanged(self.working.clone())
                } else if self.dragging && self.move_selected_point(*coord) {
                    ActionEditSessionResult::DisplayChanged(self.working.clone())
                } else {
                    ActionEditSessionResult::Handled
                }
            }
            TheEvent::RenderViewUp(id, _) if is_target(id) => {
                self.dragging = false;
                self.dragging_profile_scale = false;
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewHoverChanged(id, coord) if is_target(id) => {
                self.hover_coord = Some(*coord);
                self.hovered_point = self.nearest_point(*coord, 12.0);
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewLostHover(id) if is_target(id) => {
                self.hover_coord = None;
                self.hovered_point = None;
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewScrollBy(id, delta) if is_target(id) => {
                self.pan += Vec2::new(delta.x as f32, delta.y as f32);
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewPreciseScrollBy(id, delta) if is_target(id) => {
                if command_zoom {
                    self.zoom = (self.zoom * (delta.y as f32 * 0.025).exp()).clamp(0.2, 8.0);
                } else {
                    let delta = Vec2::new(delta.x as f32, delta.y as f32);
                    #[cfg(target_os = "macos")]
                    let delta = -delta;
                    self.pan += delta;
                }
                ActionEditSessionResult::Handled
            }
            TheEvent::RenderViewZoomBy(id, amount) if is_target(id) => {
                self.zoom = (self.zoom * (*amount * 2.5).exp()).clamp(0.2, 8.0);
                ActionEditSessionResult::Handled
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) => {
                if self.delete_selected_point() {
                    ActionEditSessionResult::DisplayChanged(self.working.clone())
                } else {
                    ActionEditSessionResult::Handled
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Return)) => {
                ActionEditSessionResult::Commit
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)) => {
                ActionEditSessionResult::Cancel
            }
            _ => ActionEditSessionResult::Ignored,
        }
    }

    fn fill_square(buffer: &mut TheRGBABuffer, center: Vec2<f32>, radius: i32, color: [u8; 4]) {
        let cx = center.x.round() as i32;
        let cy = center.y.round() as i32;
        for y in cy - radius..=cy + radius {
            for x in cx - radius..=cx + radius {
                buffer.set_pixel(x, y, &color);
            }
        }
    }

    fn draw_text(
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: TheDim,
        text: &str,
        color: [u8; 4],
        align: TheHorizontalAlign,
    ) {
        if rect.x < 0 || rect.y < 0 || rect.width <= 0 || rect.height <= 0 {
            return;
        }
        let stride = buffer.stride();
        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &rect.to_buffer_utuple(),
            stride,
            text,
            TheFontSettings {
                size: 12.0,
                ..Default::default()
            },
            &color,
            align,
            TheVerticalAlign::Center,
        );
    }

    fn measurement_precision(step: f32) -> usize {
        if step >= 1.0 {
            2
        } else if step >= 0.1 {
            3
        } else if step >= 0.01 {
            5
        } else {
            6
        }
    }

    fn grid_step_label(subdivisions: f32) -> String {
        let subdivision = subdivisions.round().clamp(1.0, 32.0) as usize;
        if subdivision == 1 {
            "1".to_string()
        } else {
            format!("1/{subdivision}")
        }
    }

    fn draw_grid_controls(&self, buffer: &mut TheRGBABuffer, ctx: &mut TheContext) {
        let active = self
            .grid_subdivisions
            .unwrap_or_else(|| 1.0 / self.profile().grid_step.max(0.0001));
        for (index, subdivision) in Self::GRID_SUBDIVISIONS.iter().copied().enumerate() {
            let rect = Self::grid_button_rect(index);
            let selected = (subdivision - active).abs() < 0.1;
            let is_hovered = self.hover_coord.is_some_and(|coord| rect.contains(coord));
            let fill = if selected {
                [68, 68, 74, 245]
            } else if is_hovered {
                [53, 57, 66, 245]
            } else {
                [31, 35, 43, 235]
            };
            let stride = buffer.stride();
            ctx.draw
                .rect(buffer.pixels_mut(), &rect.to_buffer_utuple(), stride, &fill);
            buffer.draw_rect_outline(&rect, &[83, 89, 101, 255]);
            if selected {
                buffer.draw_horizontal_line(
                    rect.x + 3,
                    rect.x + rect.width - 4,
                    rect.y + rect.height - 3,
                    [232, 190, 92, 255],
                );
            }
            Self::draw_text(
                buffer,
                ctx,
                rect,
                &(index + 1).to_string(),
                if selected || is_hovered {
                    [240, 240, 244, 255]
                } else {
                    [177, 181, 190, 255]
                },
                TheHorizontalAlign::Center,
            );
        }

        let label_x =
            Self::GRID_BUTTON_X + Self::GRID_SUBDIVISIONS.len() as i32 * Self::GRID_BUTTON_SIZE + 8;
        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect(label_x, Self::GRID_BUTTON_Y, 86, Self::GRID_BUTTON_SIZE),
            &format!("SNAP {}", Self::grid_step_label(active)),
            [211, 214, 221, 255],
            TheHorizontalAlign::Left,
        );
    }

    fn draw_scale_control(&self, buffer: &mut TheRGBABuffer, ctx: &mut TheContext) {
        if buffer.dim().width < 580 {
            return;
        }
        let track = Self::scale_track_rect();
        let hovered = self.hover_coord.is_some_and(|coord| track.contains(coord));
        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect(track.x - 52, track.y - 3, 48, track.height + 6),
            "SCALE",
            if hovered || self.dragging_profile_scale {
                [240, 240, 244, 255]
            } else {
                [177, 181, 190, 255]
            },
            TheHorizontalAlign::Right,
        );

        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &track.to_buffer_utuple(),
            stride,
            &[31, 35, 43, 235],
        );
        buffer.draw_rect_outline(&track, &[83, 89, 101, 255]);
        let center_x = Self::slider_x_from_scale(1.0);
        let handle_x = Self::slider_x_from_scale(self.profile_scale);
        buffer.draw_vertical_line(
            center_x,
            track.y + 3,
            track.y + track.height - 4,
            [91, 118, 165, 255],
        );
        buffer.draw_horizontal_line(
            center_x.min(handle_x),
            center_x.max(handle_x),
            track.y + track.height / 2,
            [232, 190, 92, 255],
        );
        buffer.draw_vertical_line(
            handle_x,
            track.y - 2,
            track.y + track.height + 1,
            if self.dragging_profile_scale {
                [255, 224, 132, 255]
            } else {
                [232, 190, 92, 255]
            },
        );
        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect(track.x + track.width + 6, track.y - 3, 56, track.height + 6),
            &format!("{:>3.0}%", self.profile_scale * 100.0),
            [211, 214, 221, 255],
            TheHorizontalAlign::Left,
        );
    }

    fn draw_preset_controls(&self, buffer: &mut TheRGBABuffer, ctx: &mut TheContext) {
        for (index, preset) in self.profile().presets.iter().enumerate() {
            let rect = Self::preset_button_rect(index);
            if rect.y + rect.height >= buffer.dim().height {
                break;
            }
            let selected = preset.points == self.profile().points;
            let hovered = self.hover_coord.is_some_and(|coord| rect.contains(coord));
            let fill = if selected {
                [68, 68, 74, 245]
            } else if hovered {
                [53, 57, 66, 245]
            } else {
                [31, 35, 43, 235]
            };
            let stride = buffer.stride();
            ctx.draw
                .rect(buffer.pixels_mut(), &rect.to_buffer_utuple(), stride, &fill);
            buffer.draw_rect_outline(&rect, &[83, 89, 101, 255]);
            if selected {
                buffer.draw_vertical_line(
                    rect.x + 2,
                    rect.y + 3,
                    rect.y + rect.height - 4,
                    [232, 190, 92, 255],
                );
            }
            Self::draw_text(
                buffer,
                ctx,
                rect,
                &preset.name.to_uppercase(),
                if selected || hovered {
                    [240, 240, 244, 255]
                } else {
                    [177, 181, 190, 255]
                },
                TheHorizontalAlign::Center,
            );
        }
    }

    fn draw_dimensions(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        transform: ProfileTransform,
    ) {
        let profile = self.profile();
        let dimensions = profile.dimensions();
        let dim = *buffer.dim();
        let axis = profile.axis_x;
        let top_y = transform
            .world_to_screen(Vec2::new(axis, dimensions.max_y))
            .y
            .round() as i32;
        let bottom_y = transform
            .world_to_screen(Vec2::new(axis, dimensions.min_y))
            .y
            .round() as i32;
        let right_x = transform
            .world_to_screen(Vec2::new(axis + dimensions.radius, dimensions.min_y))
            .x
            .round() as i32;
        let left_x = transform
            .world_to_screen(Vec2::new(axis - dimensions.radius, dimensions.min_y))
            .x
            .round() as i32;
        let axis_x = transform
            .world_to_screen(Vec2::new(axis, dimensions.min_y))
            .x
            .round() as i32;
        let color = [129, 151, 184, 255];

        let height_x = (right_x + 28).min(dim.width - 20).max(axis_x + 12);
        buffer.draw_vertical_line(height_x, top_y, bottom_y, color);
        buffer.draw_horizontal_line(height_x - 5, height_x + 5, top_y, color);
        buffer.draw_horizontal_line(height_x - 5, height_x + 5, bottom_y, color);

        let diameter_y = (bottom_y + 28).min(dim.height - 18);
        buffer.draw_horizontal_line(left_x, right_x, diameter_y, color);
        buffer.draw_vertical_line(left_x, diameter_y - 5, diameter_y + 5, color);
        buffer.draw_vertical_line(right_x, diameter_y - 5, diameter_y + 5, color);

        let precision = Self::measurement_precision(self.effective_grid_step());
        let summary = format!(
            "R {:.*}    D {:.*}    H {:.*}",
            precision,
            dimensions.radius,
            precision,
            dimensions.diameter,
            precision,
            dimensions.height
        );
        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect(14, 38, dim.width - 28, 24),
            &summary,
            [204, 213, 226, 255],
            TheHorizontalAlign::Center,
        );

        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect((left_x + right_x) / 2 - 46, diameter_y - 22, 92, 18),
            &format!("D {:.*}", precision, dimensions.diameter),
            color,
            TheHorizontalAlign::Center,
        );
        Self::draw_text(
            buffer,
            ctx,
            TheDim::rect(
                (height_x + 6).min(dim.width - 68),
                (top_y + bottom_y) / 2 - 9,
                62,
                18,
            ),
            &format!("H {:.*}", precision, dimensions.height),
            color,
            TheHorizontalAlign::Left,
        );
    }

    pub fn draw(&mut self, buffer: &mut TheRGBABuffer, ctx: &mut TheContext) {
        let dim = *buffer.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }
        self.last_view_width = dim.width;
        buffer.fill([20, 23, 29, 255]);
        let transform = self.profile_transform(dim.width, dim.height);
        self.last_transform = Some(transform);
        let profile = self.profile();

        let mut visible_step = self.effective_grid_step();
        while visible_step * transform.scale < 20.0 {
            visible_step *= 2.0;
        }
        let left = transform.screen_to_world(Vec2::new(0, dim.height)).x;
        let right = transform.screen_to_world(Vec2::new(dim.width, 0)).x;
        let bottom = transform.screen_to_world(Vec2::new(0, dim.height)).y;
        let top = transform.screen_to_world(Vec2::new(dim.width, 0)).y;
        let grid_color = [42, 48, 59, 255];
        let minor_grid_color = [32, 37, 46, 255];

        let x_start = (left / visible_step).floor() as i32;
        let x_end = (right / visible_step).ceil() as i32;
        for index in x_start..=x_end {
            let x = transform
                .world_to_screen(Vec2::new(index as f32 * visible_step, 0.0))
                .x
                .round() as i32;
            buffer.draw_vertical_line(
                x,
                0,
                dim.height - 1,
                if index % 2 == 0 {
                    grid_color
                } else {
                    minor_grid_color
                },
            );
        }
        let y_start = (bottom / visible_step).floor() as i32;
        let y_end = (top / visible_step).ceil() as i32;
        for index in y_start..=y_end {
            let y = transform
                .world_to_screen(Vec2::new(0.0, index as f32 * visible_step))
                .y
                .round() as i32;
            buffer.draw_horizontal_line(
                0,
                dim.width - 1,
                y,
                if index % 2 == 0 {
                    grid_color
                } else {
                    minor_grid_color
                },
            );
        }

        let axis_screen = transform
            .world_to_screen(Vec2::new(profile.axis_x, 0.0))
            .x
            .round() as i32;
        buffer.draw_vertical_line(axis_screen, 0, dim.height - 1, [91, 118, 165, 255]);

        if profile.mirror_preview {
            for segment in profile.points.windows(2) {
                let a = transform.world_to_screen(Vec2::new(
                    profile.axis_x - (segment[0].x - profile.axis_x),
                    segment[0].y,
                ));
                let b = transform.world_to_screen(Vec2::new(
                    profile.axis_x - (segment[1].x - profile.axis_x),
                    segment[1].y,
                ));
                buffer.draw_line(
                    a.x.round() as i32,
                    a.y.round() as i32,
                    b.x.round() as i32,
                    b.y.round() as i32,
                    [67, 88, 114, 255],
                );
            }
        }

        for segment in profile.points.windows(2) {
            let a = transform.world_to_screen(segment[0]);
            let b = transform.world_to_screen(segment[1]);
            buffer.draw_line(
                a.x.round() as i32,
                a.y.round() as i32,
                b.x.round() as i32,
                b.y.round() as i32,
                [232, 190, 92, 255],
            );
        }
        for (index, point) in profile.points.iter().enumerate() {
            let screen = transform.world_to_screen(*point);
            let color = if self.selected_point == Some(index) {
                [255, 224, 132, 255]
            } else if self.hovered_point == Some(index) {
                [255, 242, 192, 255]
            } else {
                [222, 166, 64, 255]
            };
            Self::fill_square(buffer, screen, 4, color);
            buffer.draw_rect_outline(
                &TheDim::rect(
                    screen.x.round() as i32 - 5,
                    screen.y.round() as i32 - 5,
                    11,
                    11,
                ),
                &[37, 29, 14, 255],
            );
        }

        self.draw_grid_controls(buffer, ctx);
        self.draw_scale_control(buffer, ctx);
        self.draw_preset_controls(buffer, ctx);
        self.draw_dimensions(buffer, ctx, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_validation_rejects_points_behind_the_axis() {
        let profile =
            ActionProfile2D::new("Profile", vec![Vec2::new(0.0, 0.0), Vec2::new(-0.25, 1.0)]);
        assert!(!profile.is_valid());
    }

    #[test]
    fn session_keeps_original_and_working_displays_separate() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(
            Uuid::new_v4(),
            ProjectContext::Prefab(Uuid::new_v4()),
            display.clone(),
        );
        session.profile_mut().points[1].x = 2.0;

        assert_eq!(session.original_display(), &display);
        assert_ne!(session.working_display(), &display);
    }

    #[test]
    fn profile_display_renders_in_a_shared_rgba_buffer() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 2.0),
                Vec2::new(0.0, 2.0),
            ],
        ));
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);
        let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 240));
        let mut ctx = TheContext::new(320, 240, 1.0);
        session.draw(&mut buffer, &mut ctx);

        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .any(|pixel| pixel == [232, 190, 92, 255])
        );
    }

    #[test]
    fn one_session_routes_events_to_either_editor_view() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(
            Uuid::new_v4(),
            ProjectContext::Prefab(Uuid::new_v4()),
            display,
        );

        let regular = session.handle_event(
            &TheEvent::RenderViewUp(TheId::named("PolyView"), Vec2::zero()),
            "PrefabView",
        );
        let prefab = session.handle_event(
            &TheEvent::RenderViewUp(TheId::named("PrefabView"), Vec2::zero()),
            "PrefabView",
        );

        assert_eq!(regular, ActionEditSessionResult::Ignored);
        assert_eq!(prefab, ActionEditSessionResult::Handled);
    }

    #[test]
    fn profile_dimensions_report_revolved_size() {
        let profile = ActionProfile2D::new(
            "Profile",
            vec![
                Vec2::new(0.0, -0.5),
                Vec2::new(1.25, 0.0),
                Vec2::new(0.0, 2.5),
            ],
        );
        assert_eq!(
            profile.dimensions(),
            ActionProfileDimensions {
                min_y: -0.5,
                max_y: 2.5,
                radius: 1.25,
                diameter: 2.5,
                height: 3.0,
            }
        );
    }

    #[test]
    fn shared_grid_subdivision_overrides_the_action_fallback_step() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);
        session.set_grid_subdivisions(8.0);

        assert_eq!(session.effective_grid_step(), 0.125);
    }

    #[test]
    fn profile_grid_buttons_request_the_normal_editor_subdivision() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);
        let rect = ActionEditSession::grid_button_rect(4);
        let result = session.handle_event(
            &TheEvent::RenderViewClicked(
                TheId::named("PolyView"),
                Vec2::new(rect.x + 2, rect.y + 2),
            ),
            "PolyView",
        );

        assert_eq!(
            result,
            ActionEditSessionResult::GridSubdivisionChanged(16.0)
        );
    }

    #[test]
    fn uniform_scale_keeps_the_axis_and_profile_bottom_anchored() {
        let mut profile = ActionProfile2D::new(
            "Profile",
            vec![
                Vec2::new(0.5, -1.0),
                Vec2::new(1.5, -1.0),
                Vec2::new(1.5, 2.0),
                Vec2::new(0.5, 2.0),
            ],
        );
        profile.axis_x = 0.5;
        let display = ActionEditDisplay::Profile2D(profile);
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);

        assert!(session.set_profile_scale(0.5));
        let dimensions = session.profile().dimensions();
        assert!((dimensions.radius - 0.5).abs() < 1e-6);
        assert!((dimensions.height - 1.5).abs() < 1e-6);
        assert_eq!(session.profile().points[0], Vec2::new(0.5, -1.0));
        assert_eq!(session.profile().points[1], Vec2::new(1.0, -1.0));
        assert_eq!(session.profile().points[3], Vec2::new(0.5, 0.5));
    }

    #[test]
    fn scale_slider_is_logarithmic_around_one_hundred_percent() {
        let center = ActionEditSession::slider_x_from_scale(1.0);
        assert_eq!(center, ActionEditSession::SCALE_TRACK_X + 70);
        assert!((ActionEditSession::scale_from_slider_x(center) - 1.0).abs() < 1e-6);
        assert!(
            (ActionEditSession::scale_from_slider_x(ActionEditSession::SCALE_TRACK_X)
                - ActionEditSession::MIN_PROFILE_SCALE)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn profile_preset_replaces_points_and_resets_scale() {
        let mut profile =
            ActionProfile2D::new("Profile", vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)]);
        let preset_points = vec![Vec2::new(0.0, 0.0), Vec2::new(0.5, 2.0)];
        profile.presets = vec![ActionProfilePreset::new("Tall", preset_points.clone())];
        let display = ActionEditDisplay::Profile2D(profile);
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);
        session.set_profile_scale(2.0);

        assert!(session.apply_profile_preset(0));
        assert_eq!(session.profile().points, preset_points);
        assert_eq!(session.profile_scale, 1.0);
    }

    #[test]
    fn command_precise_scroll_zooms_instead_of_panning() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);

        let result = session.handle_event_with_modifiers(
            &TheEvent::RenderViewPreciseScrollBy(TheId::named("PolyView"), Vec2::new(0, 10)),
            "PolyView",
            true,
        );

        assert_eq!(result, ActionEditSessionResult::Handled);
        assert!(session.zoom > 1.0);
        assert_eq!(session.pan, Vec2::zero());
    }

    #[test]
    fn pinch_has_a_useful_zoom_scale() {
        let display = ActionEditDisplay::Profile2D(ActionProfile2D::new(
            "Profile",
            vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
        ));
        let mut session = ActionEditSession::new(Uuid::new_v4(), ProjectContext::Unknown, display);
        session.handle_event(
            &TheEvent::RenderViewZoomBy(TheId::named("PolyView"), 0.1),
            "PolyView",
        );

        assert!(session.zoom > 1.25);
    }
}
