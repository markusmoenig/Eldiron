use scenevm::{Atom, GeoId, Light, Poly3D, RenderMode, SceneVM};
use theframework::prelude::*;
use uuid::Uuid;
use vek::{Vec3, Vec4};

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

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

pub struct CornellBox {
    vm: SceneVM,
    frame_index: u32,
    last_size: (u32, u32),
}

impl TheTrait for CornellBox {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            vm: SceneVM::new(100, 100),
            frame_index: 0,
            last_size: (0, 0),
        }
    }

    fn init(&mut self, _ctx: &mut TheContext) {
        if let Some(bytes) = Embedded::get("3d_body_pbr_raytraced.wgsl") {
            let source = std::str::from_utf8(bytes.data.as_ref())
                .unwrap_or("")
                .to_string();
            self.vm.execute(Atom::SetSource3D(source));
        }
        // Enable per-frame accumulation on the base VM layer.
        self.vm.vm.set_ping_pong_enabled(true);
        // Display layer: composite with linear-aware alpha so we don't double-gamma the path trace.
        self.vm
            .vm
            .set_blend_mode(scenevm::LayerBlendMode::AlphaLinear);

        // Disable debug logging
        self.vm.set_layer_activity_logging(false);

        // Create unique IDs for our materials
        let red_wall_id = Uuid::new_v4();
        let green_wall_id = Uuid::new_v4();
        let white_wall_id = Uuid::new_v4();
        let light_id = Uuid::new_v4();
        let cube1_id = Uuid::new_v4();
        let sphere_id = Uuid::new_v4();
        let metal_id = Uuid::new_v4();
        let glass_id = Uuid::new_v4();

        // Create solid color materials for walls with PBR properties
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: red_wall_id,
            color: [200, 50, 50, 255], // Red
            material: pack_material(
                0.8,  // Rough surface
                0.0,  // Non-metallic
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X
                None, // Default normal Y
            ),
        });
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: green_wall_id,
            color: [50, 200, 50, 255], // Green
            material: pack_material(
                0.8,  // Rough surface
                0.0,  // Non-metallic
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X
                None, // Default normal Y
            ),
        });
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: white_wall_id,
            color: [200, 200, 200, 255], // White
            material: pack_material(
                0.8,  // Rough surface
                0.0,  // Non-metallic
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X
                None, // Default normal Y
            ),
        });
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: light_id,
            color: [255, 255, 255, 255], // White light
            material: pack_material(
                0.5,  // Medium roughness
                0.0,  // Non-metallic
                1.0,  // Full opacity
                1.0,  // Full emission (it's a light!)
                None, // Default normal X
                None, // Default normal Y
            ),
        });
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: cube1_id,
            color: [180, 180, 220, 255], // Light blue
            material: pack_material(
                0.6,  // Medium-rough surface
                0.0,  // Non-metallic
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X
                None, // Default normal Y
            ),
        });
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: sphere_id,
            color: [220, 180, 180, 255], // Light pink
            material: pack_material(
                0.4,  // Slightly smooth
                0.0,  // Non-metallic
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X
                None, // Default normal Y
            ),
        });

        // Create solid materials with packed material properties
        self.vm.execute(Atom::AddSolidWithMaterial {
            id: metal_id,
            color: [220, 220, 240, 255], // Bright metallic silver for mirror
            material: pack_material(
                0.2,  // Slightly rough so GGX reflections blur; tweak to compare
                1.0,  // High metallic (strong reflections, but 10% diffuse so it's visible)
                1.0,  // Full opacity
                0.0,  // No emission
                None, // Default normal X (0.0)
                None, // Default normal Y (0.0)
            ),
        });

        self.vm.execute(Atom::AddSolidWithMaterial {
            id: glass_id,
            color: [200, 230, 255, 255], // Blue glass color (alpha in material, not here)
            material: pack_material(
                0.0,  // Very low roughness (very shiny)
                0.0,  // Non-metallic
                0.3,  // Semi-transparent (30% opacity)
                0.0,  // No emission
                None, // Default normal X (0.0)
                None, // Default normal Y (0.0)
            ),
        });

        self.vm.execute(Atom::BuildAtlas);

        // Cornell box dimensions
        let box_size = 10.0;
        let half_size = box_size / 2.0;

        // Create Cornell box walls as individual polygons (hollow box)

        // Back wall (white) - facing inward (positive Z)
        let back_wall = Poly3D::poly(
            GeoId::Unknown(0),
            white_wall_id,
            vec![
                [-half_size, -half_size, half_size, 1.0],
                [half_size, -half_size, half_size, 1.0],
                [half_size, half_size, half_size, 1.0],
                [-half_size, half_size, half_size, 1.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![(0, 1, 2), (0, 2, 3)],
        );
        self.vm.execute(Atom::AddPoly3D { poly: back_wall });

        // Left wall (red) - facing inward (positive X)
        let left_wall = Poly3D::poly(
            GeoId::Unknown(1),
            red_wall_id,
            vec![
                [-half_size, -half_size, -half_size, 1.0],
                [-half_size, -half_size, half_size, 1.0],
                [-half_size, half_size, half_size, 1.0],
                [-half_size, half_size, -half_size, 1.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![(0, 1, 2), (0, 2, 3)],
        );
        self.vm.execute(Atom::AddPoly3D { poly: left_wall });

        // Right wall (green) - facing inward (negative X)
        let right_wall = Poly3D::poly(
            GeoId::Unknown(2),
            green_wall_id,
            vec![
                [half_size, -half_size, half_size, 1.0],
                [half_size, -half_size, -half_size, 1.0],
                [half_size, half_size, -half_size, 1.0],
                [half_size, half_size, half_size, 1.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![(0, 1, 2), (0, 2, 3)],
        );
        self.vm.execute(Atom::AddPoly3D { poly: right_wall });

        // Floor (white) - facing inward (positive Y)
        let floor = Poly3D::poly(
            GeoId::Unknown(3),
            white_wall_id,
            vec![
                [-half_size, -half_size, -half_size, 1.0],
                [half_size, -half_size, -half_size, 1.0],
                [half_size, -half_size, half_size, 1.0],
                [-half_size, -half_size, half_size, 1.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![(0, 1, 2), (0, 2, 3)],
        );
        self.vm.execute(Atom::AddPoly3D { poly: floor });

        // Ceiling (white) - facing inward (negative Y)
        let ceiling = Poly3D::poly(
            GeoId::Unknown(4),
            white_wall_id,
            vec![
                [half_size, half_size, half_size, 1.0],
                [-half_size, half_size, half_size, 1.0],
                [-half_size, half_size, -half_size, 1.0],
                [half_size, half_size, -half_size, 1.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![(0, 1, 2), (0, 2, 3)],
        );
        self.vm.execute(Atom::AddPoly3D { poly: ceiling });

        // Add a cube and a sphere inside the Cornell box
        let cube_width = 3.0;
        let cube_height = 4.0;
        let cube_depth = 3.0;
        let sphere_radius = 1.5;

        // Cube (metallic, rotated 45 degrees)
        let cube_transform = vek::Mat4::rotation_y(std::f32::consts::FRAC_PI_4);
        let mut cube_poly = Poly3D::box_(
            GeoId::Unknown(5),
            metal_id,
            Vec3::new(-2.5, -half_size + cube_height / 2.0, -1.0),
            cube_width,
            cube_height,
            cube_depth,
        );
        cube_poly.vertices = cube_poly
            .vertices
            .iter()
            .map(|v| {
                let mut vec = vek::Vec4::from(*v);
                vec = cube_transform * vec;
                [vec.x, vec.y, vec.z, vec.w]
            })
            .collect();
        self.vm.execute(Atom::AddPoly3D { poly: cube_poly });

        // Sphere (glass)
        let sphere_poly = Poly3D::sphere(
            GeoId::Unknown(6),
            glass_id,
            Vec3::new(2.5, -half_size + sphere_radius, -1.5),
            sphere_radius,
            16, // stacks
            16, // slices
        );
        self.vm.execute(Atom::AddPoly3D { poly: sphere_poly });

        // Add area light at the top
        self.vm.execute(Atom::AddLight {
            id: GeoId::Light(0),
            light: Light::new_pointlight(Vec3::new(0.0, half_size - 1.0, 0.0))
                .with_color(Vec3::new(1.0, 1.0, 0.9)) // Slightly warm white
                .with_intensity(200.0)
                .with_radius(12.0),
        });

        // Add secondary fill light for better illumination
        self.vm.execute(Atom::AddLight {
            id: GeoId::Light(1),
            light: Light::new_pointlight(Vec3::new(0.0, 0.0, half_size - 2.0))
                .with_color(Vec3::new(0.9, 0.9, 1.0)) // Slightly cool white
                .with_intensity(50.0)
                .with_radius(6.0),
        });

        // Set up rendering

        // Set sky and background to dark gray so reflections have something to show
        self.vm
            .execute(Atom::SetGP0(Vec4::new(0.1, 0.1, 0.15, 1.0))); // Dark blue-gray sky for reflections

        self.vm
            .execute(Atom::SetBackground(Vec4::new(0.05, 0.05, 0.08, 1.0))); // Dark background

        // Add ambient light so surfaces aren't completely black
        // gp3: Ambient color (RGB) + ambient strength (w)
        self.vm.execute(Atom::SetGP3(Vec4::new(
            0.8, // Ambient R (linear space)
            0.8, // Ambient G
            0.8, // Ambient B
            0.3, // Ambient strength (30%)
        )));

        self.vm.execute(Atom::SetRenderMode(RenderMode::Compute3D));

        // Set up camera to look inside the Cornell box
        use scenevm::Camera3D;
        let camera = Camera3D::default()
            .look_at(
                Vec3::new(0.0, 0.0, -12.0), // Camera position inside the box, looking toward back wall
                Vec3::new(0.0, 0.0, 0.0),   // Look at center of box
                Vec3::new(0.0, 1.0, 0.0),   // Up vector
            )
            .with_perspective(60.0, 0.1, 100.0);

        self.vm.execute(Atom::SetCamera3D { camera });

        self.vm.execute(Atom::SetGP5(Vec4::new(
            8.0, // AO Samples
            0.5, // AO radius
            1.0, // Bump Strength
            8.0, // Max transparency bounces
        )));

        // Enable PBR reflections
        // gp6: x: Max shadow distance, y: Max sky distance, z: Max shadow steps, w: Reflection samples
        self.vm.execute(Atom::SetGP6(Vec4::new(
            10.0, // Max shadow distance
            50.0, // Max sky distance
            2.0,  // Max shadow steps (for transparent shadows)
            16.0, // Reflection samples (4 for good quality reflections)
        )));
    }

    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let size = (ctx.width as u32, ctx.height as u32);
        if size != self.last_size {
            self.frame_index = 0; // restart accumulation on resize
            self.last_size = size;
        }

        // Advance accumulation frame counter and pass to shader
        self.frame_index = self.frame_index.wrapping_add(1);
        self.vm
            .execute(Atom::SetAnimationCounter(self.frame_index as usize));

        // No rotation - keep the Cornell box stationary
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
