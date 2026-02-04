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
        let scene = Scene::from_static(
            vec![Batch2D::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![
                Batch3D::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0)
                    .source(PixelSource::StaticTileIndex(0))
                    .cull_mode(CullMode::Off)
                    // Metallic material which is based on half of the
                    // saturation of the pixel color
                    .material(Material::new(
                        MaterialRole::Metallic,
                        MaterialModifier::Saturation,
                        0.6,
                        0.0,
                    ))
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
        .rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            60,         // Tile size
            &self.assets,
        );

        let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
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
