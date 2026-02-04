use crate::prelude::*;
use maths_rs::prelude::*;
use rayon::prelude::*;

pub struct TheColorPicker {
    id: TheId,
    limiter: TheSizeLimiter,
    state: TheWidgetState,

    dim: TheDim,
    is_dirty: bool,

    background: Option<RGBA>,
    border: Option<RGBA>,

    color: Vec3f,

    h: f32,
    s: f32,
    l: f32,

    ls1_pt: Vec2f,
    ls2_pt: Vec2f,
    ls3_pt: Vec2f,

    hue_dot: Vec2f,
    sl_dot: Vec2f,

    dot_size: f32,
    inside: bool,

    continuous: bool,
}

impl TheWidget for TheColorPicker {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(vek::Vec2::new(200, 200));

        Self {
            id,
            limiter,
            state: TheWidgetState::None,

            dim: TheDim::zero(),
            is_dirty: false,

            background: None,
            border: None,

            color: Vec3f::zero(),

            h: 0.0,
            s: 0.0,
            l: 0.0,

            ls1_pt: Vec2f::zero(),
            ls2_pt: Vec2f::zero(),
            ls3_pt: Vec2f::zero(),

            hue_dot: Vec2f::zero(),
            sl_dot: Vec2f::zero(),

            dot_size: 0.0,
            inside: false,

            continuous: false,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                }
                ctx.ui.set_focus(self.id());
                self.calc_color(*coord, true);
                //ctx.ui.send(TheEvent::ValueChanged(self.id.clone(), TheValue::ColorObject(TheColor::from_vec3f(self.color))));
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                self.calc_color(*coord, false);
                if self.continuous {
                    ctx.ui.send(TheEvent::ValueChanged(
                        self.id.clone(),
                        TheValue::ColorObject(TheColor::from_vec3(vek::Vec3::new(
                            self.color.x,
                            self.color.y,
                            self.color.z,
                        ))),
                    ));
                }
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::MouseUp(coord) => {
                self.calc_color(*coord, false);
                ctx.ui.send(TheEvent::ValueChanged(
                    self.id.clone(),
                    TheValue::ColorObject(TheColor::from_vec3(vek::Vec3::new(
                        self.color.x,
                        self.color.y,
                        self.color.z,
                    ))),
                ));
                self.is_dirty = true;
                redraw = true;
                ctx.ui.clear_focus();
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

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        let stride: usize = buffer.stride();

        if !self.dim().is_valid() {
            return;
        }

        fn get_hue_color(pos: Vec2f) -> Vec3f {
            let theta = 3.0 + 3.0 * atan2(pos.x, pos.y) / std::f32::consts::PI;
            clamp(
                abs(fmod(theta + vec3f(0.0, 4.0, 2.0), vec3f(6.0, 6.0, 6.0)) - 3.0) - 1.0,
                Vec3f::zero(),
                Vec3f::one(),
            )
        }

        //style.draw_widget_border(buffer, self, &mut shrinker, ctx);

        let size = min(self.dim.width, self.dim.height);

        let mut b = TheRGBABuffer::new(TheDim::new(0, 0, size, size));

        let pixels = b.pixels_mut();

        let width = size as usize;

        if let Some(bc) = self.background {
            let stride = buffer.stride();
            ctx.draw.rect(
                buffer.pixels_mut(),
                &self.dim.to_buffer_utuple(),
                stride,
                &bc,
            );
        }

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as i32;
                    let y = /*height -*/ (i / width) as i32 - 1;

                    let xx = x as f32 / size as f32;
                    let yy = y as f32 / size as f32;

                    //let c;// = [xx, yy, 0.0, 1.0];

                    let mut uv = vec2f(2.0, -2.0) * (vec2f(xx, yy) - vec2f(0.5, 0.5));

                    let mut l = length(uv);

                    l = 1.0 - abs((l - 0.875) * 8.0);
                    l = clamp(l * size as f32 * 0.0625, 0.0, 1.0);

                    let t = l * get_hue_color(uv);
                    let mut col = vec4f(t.x, t.y, t.z, l);

                    if l < 0.75 {
                        uv /= 0.75;

                        let mut inhsl = vec3f(self.h, self.h, self.l); //rgb2hsl(data->color.xyz);
                        inhsl.x /= 360.0;

                        let angle = ((inhsl.x * 360.0) - 180.0) * std::f32::consts::PI / 180.0;
                        let mut mouse = vec2f(sin(angle), cos(angle));

                        let picked_hue_color = get_hue_color(mouse);

                        mouse = normalize(mouse);

                        let sat = 1.5 - (dot(uv, mouse) + 0.5); // [0.0,1.5]

                        if sat < 1.5 {
                            let h = sat / sqrt(3.0);
                            let om = cross(vec3f(mouse.x, mouse.y, 0.0), vec3f(0.0, 0.0, 1.0)).xy();

                            let lum = dot(uv, om);

                            if abs(lum) <= h {
                                l = clamp((h - abs(lum)) * size as f32 * 0.5, 0.0, 1.0)
                                    * clamp((1.5 - sat) / 1.5 * size as f32 * 0.5, 0.0, 1.0); // Fake antialiasing

                                let p = 0.5 * (lum + h) / h;

                                let r = l * lerp(picked_hue_color, vec3f(p, p, p), sat / 1.5);
                                col = vec4f(r.x, r.y, r.z, l);
                            }
                        }

                        //col.xyz = pickedHueColor;
                    }

                    let color = [
                        (col[0] * 255.0) as u8,
                        (col[1] * 255.0) as u8,
                        (col[2] * 255.0) as u8,
                        (col[3] * 255.0) as u8,
                    ];
                    pixel.copy_from_slice(&color);
                }
            });

        buffer.blend_into(self.dim.buffer_x, self.dim.buffer_y, &b);

        self.dot_size = (size as f32 / 20.0).max(5.0);
        self.compute_points();

        let mut r = self.dim.to_buffer_utuple();
        r.0 += self.hue_dot.x as usize;
        r.1 += self.hue_dot.y as usize;
        r.2 = self.dot_size as usize * 2;
        r.3 = self.dot_size as usize * 2;
        ctx.draw
            .circle(buffer.pixels_mut(), &r, stride, &BLACK, self.dot_size * 0.8);

        let mut r = self.dim.to_buffer_utuple();
        r.0 += self.sl_dot.x as usize;
        r.1 += self.sl_dot.y as usize;
        r.2 = self.dot_size as usize * 2;
        r.3 = self.dot_size as usize * 2;
        ctx.draw
            .circle(buffer.pixels_mut(), &r, stride, &BLACK, self.dot_size * 0.8);

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn value(&self) -> TheValue {
        TheValue::Float3(vek::Vec3::new(self.color.x, self.color.y, self.color.z))
    }

    #[allow(clippy::single_match)]
    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::ColorObject(color) => {
                let col = color.to_vec3();
                self.color = vec3f(col.x, col.y, col.z);
                let hsl = color.as_hsl();
                self.h = hsl.x * 360.0;
                self.s = hsl.y;
                self.l = hsl.z;
                self.is_dirty = true;
            }
            TheValue::Float3(color) => {
                self.color = vec3f(color.x, color.y, color.z);
                let hsl = TheColor::from_vec3(color).as_hsl();
                self.h = hsl.x * 360.0;
                self.s = hsl.y;
                self.l = hsl.z;
                self.is_dirty = true;
            }
            _ => {}
        }
    }
}

pub trait TheColorPickerTrait: TheWidget {
    fn set_background_color(&mut self, color: [u8; 4]);
    fn set_border_color(&mut self, color: [u8; 4]);

    fn set_color(&mut self, color: vek::Vec3<f32>);
    fn set_continuous(&mut self, continuous: bool);

    fn compute_points(&mut self);
    fn calc_color(&mut self, coord: vek::Vec2<i32>, new_op: bool);
    fn get_hue_at(&mut self, coord: vek::Vec2<i32>);
    fn get_sl_at(&mut self, coord: vek::Vec2<i32>);
}

impl TheColorPickerTrait for TheColorPicker {
    fn set_background_color(&mut self, color: [u8; 4]) {
        self.background = Some(color);
    }

    fn set_border_color(&mut self, color: [u8; 4]) {
        self.border = Some(color);
    }

    fn set_color(&mut self, color: vek::Vec3<f32>) {
        self.color = vec3f(color.x, color.y, color.z);
        let hsl = TheColor::from_vec3(color).as_hsl();
        self.h = hsl.x * 360.0;
        self.s = hsl.y;
        self.l = hsl.z;
    }

    fn set_continuous(&mut self, continuous: bool) {
        self.continuous = continuous;
    }

    fn calc_color(&mut self, co: vek::Vec2<i32>, new_op: bool) {
        let coord = vec2i(co.x, co.y);

        if new_op {
            let x = coord.x as f32;
            let y = coord.y as f32;

            let circle_size = min(self.dim.width, self.dim.height) as f32;
            let center = vec2f(circle_size / 2.0, circle_size / 2.0);

            let dist = distance(center, vec2f(x, y));
            self.inside = dist < (circle_size * 0.75) / 2.0;

            //print("calcColor", x, y, dist, insideOp)
        }

        if !self.inside {
            self.get_hue_at(co);
            self.compute_points();
        } else {
            self.get_sl_at(co);
            self.compute_points();
        }
    }

    fn get_sl_at(&mut self, co: vek::Vec2<i32>) {
        let coord = vec2i(co.x, co.y);

        let x = coord.x as f32 - self.dot_size / 1.0;
        let y = coord.y as f32 - self.dot_size / 1.0;

        // if x < 0 || x > rect.width { return }
        // if y < 0 || y > rect.width { return }

        fn sign_of(p1: Vec2f, p2: Vec2f, p3: Vec2f) -> f32 {
            (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x)
        }

        fn limit(v: f32) -> f32 {
            if v < 0.0 {
                return 0.0;
            }
            if v > 1.0 {
                return 1.0;
            }
            v
        }

        let mut ev = vec2f(x, y);

        let b1 = sign_of(ev, self.ls1_pt, self.ls2_pt) <= 0.0;
        let b2 = sign_of(ev, self.ls2_pt, self.ls3_pt) <= 0.0;
        let b3 = sign_of(ev, self.ls3_pt, self.ls1_pt) <= 0.0;

        let mut fail = false;
        // in this case coordinate axis is clockwise
        if b1 && b2 && b3 {
            // inside triangle
            ev -= self.ls1_pt;
        } else if b2 && b3 {
            let line = self.ls2_pt - self.ls1_pt;
            ev -= self.ls1_pt;
            ev = line * limit(dot(line, ev) / (length(line) * length(line)))
        } else if b1 && b2 {
            let line = self.ls3_pt - self.ls1_pt;
            ev -= self.ls1_pt;
            ev = line * limit(dot(line, ev) / (length(line) * length(line)))
        } else if b1 && b3 {
            let line = self.ls2_pt - self.ls3_pt;
            ev -= self.ls3_pt;
            ev = line * limit(dot(line, ev) / (length(line) * length(line)));
            ev += self.ls3_pt - self.ls1_pt;
        } else {
            fail = true
        }

        if !fail {
            let p3 = self.ls3_pt - self.ls1_pt;
            let side = length(p3);
            self.l = dot(ev, p3) / (side * side);
            if self.l > 0.01 && self.l < 0.99 {
                let up = ((self.ls3_pt + self.ls1_pt) * -0.5) + self.ls2_pt;
                let temp = if self.l < 0.5 { self.l } else { 1.0 - self.l };
                self.s = dot(ev, up) / length(up) / length(up) * 0.5 / temp;
            }
            let color = TheColor::from_hsl(self.h, self.s, self.l).to_vec3();
            self.color = vec3f(color.x, color.y, color.z);
        }
    }

    fn get_hue_at(&mut self, co: vek::Vec2<i32>) {
        let coord = vec2i(co.x, co.y);

        let x = coord.x as f32;
        let y = coord.y as f32;

        let hsv =
            TheColor::from_vec3(vek::Vec3::new(self.color.x, self.color.y, self.color.z)).as_hsl();

        let circle_size = min(self.dim.width, self.dim.height) as f32;

        let center = vec2f(circle_size / 2.0, circle_size / 2.0);
        let mut mouse = vec2f(x - center.x, y - center.y);

        mouse = normalize(mouse);

        mouse *= circle_size / 2.0 - (circle_size * 0.75) / 2.0;
        let v = center + mouse;
        let angle = atan2(v.x - center.x, v.y - center.y) * 180.0 / std::f32::consts::PI;
        let rgb = TheColor::from_hsl(angle + 180.0, hsv.y, hsv.z).to_vec3();
        self.h = angle + 180.0;
        self.color.x = rgb.x;
        self.color.y = rgb.y;
        self.color.z = rgb.z;
    }

    fn compute_points(&mut self) {
        let circle_size = min(self.dim.width, self.dim.height) as f32;

        let center = vec2f(circle_size / 2.0, circle_size / 2.0);
        let mut angle = (self.h - 180.0) * std::f32::consts::PI / 180.0;
        let mut dir = vec2f(sin(angle), cos(angle));

        dir = normalize(dir);

        let sub = self.dot_size * 1.4;

        let mut ldir = dir;
        ldir *= circle_size / 2.0 - self.dot_size * 1.4;

        self.hue_dot = center + ldir - self.dot_size;

        ldir = dir;
        ldir *= circle_size / 2.0 - sub * 2.0;

        self.ls2_pt = center + ldir - self.dot_size;

        angle = (self.h - 180.0 - 120.0) * std::f32::consts::PI / 180.0;
        dir = vec2f(sin(angle), cos(angle));
        dir = normalize(dir);
        dir *= circle_size / 2.0 - sub * 2.0;
        self.ls1_pt = center + dir - self.dot_size;

        angle = (self.h - 180.0 + 120.0) * std::f32::consts::PI / 180.0;
        dir = vec2f(sin(angle), cos(angle));
        dir = normalize(dir);
        dir *= circle_size / 2.0 - sub * 2.0;
        self.ls3_pt = center + dir - self.dot_size;

        let base = (self.ls3_pt - self.ls1_pt) * self.l;
        let up = ((self.ls3_pt + self.ls1_pt) * -0.5) + self.ls2_pt;
        let temp = (if self.l < 0.5 { self.l } else { 1.0 - self.l }) * 2.0 * self.s;
        self.sl_dot = base + up * temp + self.ls1_pt;
    }
}
