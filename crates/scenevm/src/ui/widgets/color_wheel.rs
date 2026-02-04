use uuid::Uuid;
use vek::{Vec3, Vec4};

use crate::ui::{
    drawable::Drawable,
    event::{UiAction, UiEvent, UiEventKind, UiEventOutcome},
    workspace::{UiView, ViewContext},
};
use crate::vm::{Atom, VM};

/// HSV Color Wheel widget for color selection
pub struct ColorWheel {
    id: String,
    tile_id: Uuid,
    render_id: Uuid,
    indicator_id: Uuid,
    pub rect: [f32; 4],        // [x, y, width, height]
    current_color: Vec4<f32>,  // RGBA
    current_hsv: Vec3<f32>,    // HSV (hue 0-360, sat 0-1, val 0-1)
    original_color: Vec4<f32>, // Color at start of drag (for undo)
    radius: f32,
    dragging: bool,
    active_pointer: Option<u32>,
}

impl ColorWheel {
    /// Create a new color wheel with the given rect and initial color
    pub fn new(rect: [f32; 4], initial_color: Vec4<f32>) -> Self {
        let size = rect[2].min(rect[3]);
        let radius = size / 2.0;
        let hsv = rgb_to_hsv(initial_color);

        Self {
            id: String::new(),
            tile_id: Uuid::new_v4(),
            render_id: Uuid::new_v4(),
            indicator_id: Uuid::new_v4(),
            rect,
            current_color: initial_color,
            current_hsv: hsv,
            original_color: initial_color,
            radius,
            dragging: false,
            active_pointer: None,
        }
    }

    /// Set a custom ID for this widget
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Get the current selected color
    pub fn color(&self) -> Vec4<f32> {
        self.current_color
    }

    /// Set the color
    pub fn set_color(&mut self, color: Vec4<f32>) {
        self.current_color = color;
        self.current_hsv = rgb_to_hsv(color);
    }

    /// Set the rectangle
    pub fn set_rect(&mut self, rect: [f32; 4]) {
        self.rect = rect;
        let size = rect[2].min(rect[3]);
        self.radius = size / 2.0;
    }

    /// Ensure the color wheel tile is registered in the atlas
    pub fn ensure_tile(&self, vm: &mut VM) {
        // Create a 2x1 tile with material data for color wheel
        // Color pixels (not used for color wheel, just placeholder)
        let color_pixels = vec![0u8, 0, 0, 255, 0, 0, 0, 255];

        // Material: texel0.r = widget_type (2=color wheel), texel0.b = 255 (style flag)
        let widget_type = 2u8; // 2 = color wheel
        let mat_tex0 = [widget_type, 0, 255, 0];
        let mat_tex1 = [widget_type, 0, 255, 0];
        let mut mat_pixels = Vec::from(mat_tex0);
        mat_pixels.extend_from_slice(&mat_tex1);

        vm.execute(Atom::AddTile {
            id: self.tile_id,
            width: 2,
            height: 1,
            frames: vec![color_pixels],
            material_frames: Some(vec![mat_pixels]),
        });
        vm.execute(Atom::BuildAtlas);
    }

    /// Get the tile ID for this color wheel
    pub fn tile_id(&self) -> Uuid {
        self.tile_id
    }

    /// Get current HSV value component for shader (0-1 range)
    pub fn hsv_value(&self) -> f32 {
        self.current_hsv.z
    }

    /// Update the color based on pointer position
    fn update_color_from_pos(&mut self, pos: [f32; 2]) -> bool {
        let rel_x = (pos[0] - self.rect[0]) / self.rect[2];
        let rel_y = (pos[1] - self.rect[1]) / self.rect[3];

        let u = rel_x.clamp(0.0, 1.0);
        let v = rel_y.clamp(0.0, 1.0);

        // Hue across X (mirrored in shader). Y mapping:
        // top half ramps saturation 0->1 at full value (white->vivid),
        // bottom half keeps sat=1 and ramps value 1->0 (vivid->black).
        let hue = (1.0 - u) * 360.0;
        let (sat, val) = if v < 0.5 {
            let t = v * 2.0;
            (t, 1.0)
        } else {
            let t = (v - 0.5) * 2.0;
            (1.0, 1.0 - t)
        };

        let new_hsv = Vec3::new(hue, sat, val);
        let delta = new_hsv - self.current_hsv;
        let max_delta = delta.x.abs().max(delta.y.abs()).max(delta.z.abs());
        let changed = max_delta > 1e-4;

        if changed {
            self.current_hsv = new_hsv;
            self.current_color = hsv_to_rgb(self.current_hsv);
        }

        changed
    }

    fn hit_test(&self, pos: [f32; 2]) -> bool {
        pos[0] >= self.rect[0]
            && pos[0] <= self.rect[0] + self.rect[2]
            && pos[1] >= self.rect[1]
            && pos[1] <= self.rect[1] + self.rect[3]
    }
}

impl UiView for ColorWheel {
    fn build(&mut self, ctx: &mut ViewContext) {
        // Create a single quad for the color wheel
        // Material texture will contain widget type (2.0 for color wheel)
        // The shader will read U.gp0.z for the HSV value

        ctx.push(Drawable::Quad {
            id: self.render_id,
            tile_id: self.tile_id,
            rect: self.rect,
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            layer: 10,
            tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
        });

        // Draw current color indicator (small circle at selected position)
        let indicator_x =
            self.rect[0] + (1.0 - (self.current_hsv.x / 360.0).clamp(0.0, 1.0)) * self.rect[2];
        // Inverse of the 2-band Y control for indicator placement.
        let indicator_y = self.rect[1]
            + if self.current_hsv.z < 1.0 - 1e-4 {
                // Bottom half: val ramps down 1->0
                (0.5 + (1.0 - self.current_hsv.z).clamp(0.0, 1.0) * 0.5) * self.rect[3]
            } else {
                // Top half: sat ramps up 0->1
                (self.current_hsv.y.clamp(0.0, 1.0) * 0.5) * self.rect[3]
            };
        let indicator_size = 12.0;

        ctx.push(Drawable::Rect {
            id: self.indicator_id,
            rect: [
                indicator_x - indicator_size / 2.0,
                indicator_y - indicator_size / 2.0,
                indicator_size,
                indicator_size,
            ],
            fill: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border: Vec4::new(0.0, 0.0, 0.0, 1.0),
            border_px: 2.0,
            radius_px: indicator_size / 2.0,
            layer: 11,
        });
    }

    fn handle_event(&mut self, event: &UiEvent) -> UiEventOutcome {
        match event.kind {
            UiEventKind::PointerDown => {
                if self.hit_test(event.pos) {
                    self.dragging = true;
                    self.active_pointer = Some(event.pointer_id);
                    // Store original color for undo
                    self.original_color = self.current_color;
                    if self.update_color_from_pos(event.pos) {
                        return UiEventOutcome::with_action(UiAction::ColorChanged(
                            self.id.clone(),
                            [
                                self.current_color.x,
                                self.current_color.y,
                                self.current_color.z,
                                self.current_color.w,
                            ],
                            [
                                self.original_color.x,
                                self.original_color.y,
                                self.original_color.z,
                                self.original_color.w,
                            ],
                            false, // Not final (drag started)
                        ));
                    }
                    return UiEventOutcome::dirty(); // Consume event even if color didn't change
                }
            }
            UiEventKind::PointerMove => {
                if self.dragging && self.active_pointer == Some(event.pointer_id) {
                    if self.update_color_from_pos(event.pos) {
                        return UiEventOutcome::with_action(UiAction::ColorChanged(
                            self.id.clone(),
                            [
                                self.current_color.x,
                                self.current_color.y,
                                self.current_color.z,
                                self.current_color.w,
                            ],
                            [
                                self.original_color.x,
                                self.original_color.y,
                                self.original_color.z,
                                self.original_color.w,
                            ],
                            false, // Not final (dragging)
                        ));
                    }
                    return UiEventOutcome::dirty(); // Consume event even if color didn't change
                }
            }
            UiEventKind::PointerUp => {
                if self.active_pointer == Some(event.pointer_id) {
                    self.dragging = false;
                    self.active_pointer = None;
                    // Send final ColorChanged event on mouse up
                    return UiEventOutcome::with_action(UiAction::ColorChanged(
                        self.id.clone(),
                        [
                            self.current_color.x,
                            self.current_color.y,
                            self.current_color.z,
                            self.current_color.w,
                        ],
                        [
                            self.original_color.x,
                            self.original_color.y,
                            self.original_color.z,
                            self.original_color.w,
                        ],
                        true, // Final (mouse released)
                    ));
                }
            }
            _ => {}
        }
        UiEventOutcome::none()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}

// HSV to RGB conversion
fn hsv_to_rgb(hsv: Vec3<f32>) -> Vec4<f32> {
    let h = hsv.x / 60.0;
    let s = hsv.y;
    let v = hsv.z;

    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 {
        (c, x, 0.0)
    } else if h < 2.0 {
        (x, c, 0.0)
    } else if h < 3.0 {
        (0.0, c, x)
    } else if h < 4.0 {
        (0.0, x, c)
    } else if h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Vec4::new(r + m, g + m, b + m, 1.0)
}

// RGB to HSV conversion
fn rgb_to_hsv(rgb: Vec4<f32>) -> Vec3<f32> {
    let r = rgb.x;
    let g = rgb.y;
    let b = rgb.z;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    let saturation = if max == 0.0 { 0.0 } else { delta / max };

    let value = max;

    Vec3::new(hue, saturation, value)
}
