use scenevm::{Atom, GeoId, Light, Poly2D, Poly3D, RenderMode, SceneVM};
use theframework::prelude::*;
use uuid::Uuid;
use vek::Mat4;
/// Helper function to pack material properties into the unified u32 format
///
/// ## Format
/// - Bits 0-3:   Roughness (0-15, maps to 0.0-1.0)
/// - Bits 4-7:   Metallic  (0-15, maps to 0.0-1.0)
/// - Bits 8-11:  Opacity   (0-15, maps to 0.0-1.0)
/// - Bits 12-15: Emissive  (0-15, maps to 0.0-1.0)
/// - Bits 16-23: Normal X (0-255, maps to -1.0 to +1.0, typically 128 = 0.0)
/// - Bits 24-31: Normal Y (0-255, maps to -1.0 to +1.0, typically 128 = 0.0)
///
/// # Arguments
/// * `roughness` - 0.0 to 1.0
/// * `metallic` - 0.0 to 1.0
/// * `opacity` - 0.0 to 1.0
/// * `emissive` - 0.0 to 1.0
/// * `normal_x` - -1.0 to +1.0 (optional, default 0.0)
/// * `normal_y` - -1.0 to +1.0 (optional, default 0.0)
fn pack_material(
    roughness: f32,
    metallic: f32,
    opacity: f32,
    emissive: f32,
    normal_x: Option<f32>,
    normal_y: Option<f32>,
) -> [u8; 4] {
    // Clamp and quantize to 4 bits (0-15)
    let r = (roughness.clamp(0.0, 1.0) * 15.0).round() as u8;
    let m = (metallic.clamp(0.0, 1.0) * 15.0).round() as u8;
    let o = (opacity.clamp(0.0, 1.0) * 15.0).round() as u8;
    let e = (emissive.clamp(0.0, 1.0) * 15.0).round() as u8;

    // Pack into lower 16 bits
    let mat_lo = r | (m << 4);
    let mat_hi = o | (e << 4);

    // Pack normals into upper 16 bits (convert -1..1 to 0..255)
    let nx = normal_x.unwrap_or(0.0);
    let ny = normal_y.unwrap_or(0.0);
    let norm_x = ((nx.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;
    let norm_y = ((ny.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;

    [mat_lo, mat_hi, norm_x, norm_y]
}

pub struct Circle {
    vm: SceneVM,

    matrix: Mat4<f32>,
}

impl TheTrait for Circle {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            vm: SceneVM::new(100, 100),
            matrix: Mat4::identity(),
        }
    }

    // #[cfg(not(target_arch = "wasm32"))]
    fn init(&mut self, _ctx: &mut TheContext) {
        let tile_id = Uuid::new_v4();
        let overlay_tile = Uuid::new_v4();

        // if let Some((data, width, height)) = self
        //     .vm
        //     .load_image_rgba(std::path::Path::new("images/logo.png"))
        // {
        //     self.vm.execute(Atom::AddTile {
        //         id: tile_id,
        //         width: width,
        //         height: height,
        //         frames: vec![data],
        //         material_frames: None,
        //     });
        //     self.vm.execute(Atom::BuildAtlas);
        // }

        // self.vm.set_layer_activity_logging(true);
        self.vm.execute(Atom::SetBackground(Vec4::zero()));
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: tile_id,
            color: [128, 128, 128, 255],
            material: pack_material(
                0.0,  // Very low roughness (very shiny)
                0.0,  // Non-metallic
                1.0,  // Semi-transparent (30% opacity)
                0.0,  // No emission
                None, // Default normal X (0.0)
                None, // Default normal Y (0.0)
            ),
        });
        self.vm.execute(Atom::AddSolid {
            id: overlay_tile,
            color: [255, 80, 80, 160],
        });
        self.vm.execute(Atom::BuildAtlas);

        self.vm.execute(Atom::AddPoly3D {
            poly: Poly3D::cube(GeoId::Unknown(0), tile_id, Vec3::zero(), 2.0),
        });

        self.vm.execute(Atom::AddLight {
            id: GeoId::Light(0),
            light: Light::new_pointlight(Vec3::new(0.0, 1.0, -4.0))
                .with_color(Vec3::new(1.0, 0.95, 0.9))
                .with_intensity(150.0)
                .with_radius(12.0)
                .with_end_distance(18.0),
        });

        // Render Settings
        self.vm.execute(Atom::SetGP5(Vec4::new(
            8.0, // AO Samples
            0.5, // AO radius
            1.0, // Bump Strength
            8.0, // Max transparency bounces
        )));
        // Add a little ambient so the cube is visible even if the light misses
        self.vm.execute(Atom::SetGP3(Vec4::new(
            0.6,  // Ambient R (linear)
            0.6,  // Ambient G
            0.7,  // Ambient B
            0.15, // Ambient strength
        )));
        // Sky tint for reflections/background
        self.vm
            .execute(Atom::SetGP0(Vec4::new(0.1, 0.15, 0.2, 1.0)));

        // Enable PBR reflections
        // gp6: x: Max shadow distance, y: Max sky distance, z: Max shadow steps, w: Reflection samples
        self.vm.execute(Atom::SetGP6(Vec4::new(
            10.0, // Max shadow distance
            50.0, // Max sky distance
            2.0,  // Max shadow steps (for transparent shadows)
            16.0, // Reflection samples (4 for good quality reflections)
        )));

        self.vm.execute(Atom::SetRenderMode(RenderMode::Compute3D));

        // VM1: 2D overlay gets its own layer so it can draw on top without clearing.
        let overlay_index = self.vm.add_vm_layer();
        self.vm.set_active_vm(overlay_index);

        self.vm.execute(Atom::AddPoly {
            poly: Poly2D::poly(
                GeoId::Unknown(0),
                overlay_tile,
                vec![[40.0, 40.0], [160.0, 40.0], [160.0, 160.0], [40.0, 160.0]],
                vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                vec![(0, 1, 2), (0, 2, 3)],
            ),
        });

        // Add a line strip
        self.vm.execute(Atom::AddLineStrip2D {
            id: GeoId::Linedef(1),
            tile_id: overlay_tile,
            points: vec![[200.0, 60.0], [240.0, 120.0], [280.0, 180.0]],
            width: 2.0,
        });

        // Switch back so subsequent commands keep targeting the primary 3D VM.
        self.vm.set_active_vm(0);

        // self.vm.execute(Atom::SetCamera3D {
        //     camera: Camera3D::iso(),
        // });
    }

    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        // Rotate a bit every frame to see the cube spinning (angles in radians per frame)
        let rot = Mat4::<f32>::rotation_y(0.02) * Mat4::<f32>::rotation_x(0.01);
        self.matrix = rot * self.matrix;
        self.vm.execute(Atom::SetTransform3D(self.matrix));

        self.vm
            .render_frame(pixels, ctx.width as u32, ctx.height as u32);
    }

    /// Touch down event
    fn touch_down(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Touch up event
    fn touch_up(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }
}
