use rusterix::prelude::*;
use std::path::Path;
use std::time::Instant;
use theframework::*;
use vek::{Vec2, Vec3, Vec4};

fn main() {
    let cube = Cube::new();
    let app = TheApp::new();

    () = app.run(Box::new(cube));
}

// This example uses static draw calls into rusterix, bypassing the game engine API.
pub struct Cube {
    camera: D3OrbitCamera,
    scene: Scene,
    assets: Assets,
    start_time: Instant,
}

impl TheTrait for Cube {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut scene = Scene::from_static(
            vec![Batch2D::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![
                Batch3D::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0)
                    .source(PixelSource::StaticTileIndex(0))
                    .cull_mode(CullMode::Off)
                    .ambient_color(Vec3::broadcast(0.3))
                    .shader(0)
                    .with_computed_normals(),
            ],
        )
        .lights(vec![
            Light::new(LightType::Point)
                .with_intensity(1.0)
                .with_color([1.0, 1.0, 0.95])
                .compile(),
        ])
        .background(Box::new(VGrayGradientShader::new()));

        scene.add_shader(
            r#"
            fn shade() {
                // Procedural wood: concentric growth rings warped by turbulence + fine grain.
                // Only the .x channel of textures is used (value channel).
                let t = time * 0.0;

                // Move and scale domain; center the rings roughly in the middle of each face.
                let uv2 = uv / 3.0 - vec2(1.5);

                // fBm turbulence (zero-mean) to warp the rings
                let n1 = sample(uv2 + vec2(t, 0.0), "fbm_perlin");   // [0,1]
                let n2 = sample(uv2 * 2.0 + vec2(0.0, t*0.7), "fbm_perlin");
                let turb = 0.65 * n1 + 0.35 * n2;                       // [0,1]
                let turb_zm = (turb - 0.5) * 2.0;                       // [-1,1]

                // Radial distance from center (log cross-section look)
                let r = length(uv2);

                // Warp rings by turbulence (phase modulation)
                let ring_freq = 10.0;            // number of rings
                let ring_warp = 0.22;            // strength of warp
                let rings = r + ring_warp * turb_zm;
                let waves = sin(rings * ring_freq);

                // Map sine to ring mask; sharpen valleys to make rings thinner
                let rings_mask = pow(1.0 - abs(waves), 3.0);

                // Fine longitudinal grain: high-frequency value noise stretched along X
                let grain_uv = vec2(uv2.x * 8.0, uv2.y * 40.0);
                let g = sample(grain_uv + vec2(0.0, t*0.5), "value");
                let grain = (g - 0.5) * 2.0;     // zero-mean

                // Base wood hues
                let base_light = vec3(0.72, 0.52, 0.32);
                let base_dark  = vec3(0.45, 0.30, 0.16);

                // Mix light/dark by ring mask
                color = mix(base_light, base_dark, rings_mask);

                // Apply subtle anisotropic grain as a multiplicative zero-mean factor
                color *= (1.0 + 0.06 * grain);

                // Optional pore streaks (cathedrals): directional bands along Y with slight turbulence
                let band = uv2.y + 0.15 * turb_zm;
                let cathedral = pow(1.0 - abs(sin(band * 6.0)), 4.0);
                color = mix(color, color * 0.9, cathedral * 0.2);

                // Roughness varies: pores are rougher, rings smoother
                roughness = 0.6 + cathedral * 0.3;

                // 16 Colors
                //let color_steps = 16.0;
                //color = floor(color * color_steps) / color_steps;
            }
        "#,
        );

        let assets = Assets::default().textures(vec![Tile::from_texture(Texture::from_image(
            Path::new("images/logo.png"),
        ))]);

        let mut camera = D3OrbitCamera::new();
        camera.set_parameter_f32("distance", 1.5);

        Self {
            camera,
            scene,
            start_time: Instant::now(),
            assets,
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        // Animate light in circle around Y-axis
        let elapsed = self.start_time.elapsed().as_secs_f32() * 1.5;
        self.scene.lights[0].position = Vec3::new(2.0 * elapsed.cos(), 0.8, 2.0 * elapsed.sin());

        // Set it up
        Rasterizer::setup(
            None,
            self.camera.view_matrix(),
            self.camera
                .projection_matrix(ctx.width as f32, ctx.height as f32),
        )
        .ambient(Vec4::broadcast(0.1))
        .time(elapsed)
        .rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            80,         // Tile size
            &self.assets,
        );

        let _stop = get_time();
        println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Hover event
    fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        self.camera.set_parameter_vec2(
            "from_normalized",
            Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        );
        true
    }

    // Query if the widget needs a redraw, we redraw at max speed (which is not necessary)
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }

    fn window_title(&self) -> String {
        "Rusterix Cube Demo".to_string()
    }
}

fn get_time() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now() as u128
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let stop = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");
        stop.as_millis()
    }
}
