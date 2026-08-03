use crate::{
    ast::{
        BinaryOperator, ColorRange, ColorSource, Colorize, CombineMode, CoordinateChannel, Domain,
        FieldDefinition, FractalKind, GeometryChannel, GeometryFeature, HeightOperation, IdSource,
        MaterialOutput, MaterialRecipe, NoiseKind, Output, PaletteMode, PatternChannel,
        PatternDefinition, PatternKind, Recipe, ScalarSource, UnaryOperator, WrapMode,
    },
    palette::{PaletteError, PaletteModel},
};
use std::{error::Error, fmt};
use theframework::prelude::{TheColor, ThePalette};

const MAX_RENDER_DIMENSION: u32 = 16_384;

enum ColorRamp {
    Palette(Vec<usize>),
    Rgba(Vec<[u8; 4]>),
}

#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    pub seed_offset: u64,
}

#[derive(Clone, Debug)]
pub struct RenderedFrame {
    pub rgba: Vec<u8>,
    pub palette_indices: Vec<u16>,
    pub coverage: Vec<u8>,
    pub height: Vec<u8>,
    pub time: f32,
}

#[derive(Clone, Debug)]
pub struct RenderedRecipe {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub fps: f32,
    pub looping: bool,
    pub frames: Vec<RenderedFrame>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedMaterialFrame {
    pub rgba: Vec<u8>,
    pub palette_indices: Vec<u16>,
    pub material: Vec<[f32; 4]>,
    pub normal_height: Vec<u8>,
    pub time: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedMaterial {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub fps: f32,
    pub looping: bool,
    pub normal_strength: f32,
    pub frames: Vec<RenderedMaterialFrame>,
}

/// One frame requested by a consumer-neutral procedural surface.
///
/// The surface describes where and when to evaluate a recipe without carrying
/// Tile-specific concepts such as coverage, grid coordinates, or geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSurfaceFrame {
    pub time: f32,
}

/// Affine mapping from normalized surface pixels to Recipe coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSurfaceMapping {
    pub origin: [f32; 2],
    pub u_axis: [f32; 2],
    pub v_axis: [f32; 2],
}

impl Default for RenderSurfaceMapping {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0],
            u_axis: [1.0, 0.0],
            v_axis: [0.0, 1.0],
        }
    }
}

impl RenderSurfaceMapping {
    pub fn map(self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + self.u_axis[0] * uv[0] + self.v_axis[0] * uv[1],
            self.origin[1] + self.u_axis[1] * uv[0] + self.v_axis[1] * uv[1],
        ]
    }
}

/// A flat procedural evaluation target that can be owned by any consumer.
///
/// Tiles adapt to this type today. Avatars, UI elements, and other procedural
/// consumers can use it directly without pretending to be Tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSurface {
    pub width: u32,
    pub height: u32,
    pub mapping: RenderSurfaceMapping,
    pub fps: f32,
    pub looping: bool,
    pub frames: Vec<RenderSurfaceFrame>,
}

impl From<&RenderedRecipe> for RenderSurface {
    fn from(recipe: &RenderedRecipe) -> Self {
        Self {
            width: recipe.width,
            height: recipe.height,
            mapping: RenderSurfaceMapping::default(),
            fps: recipe.fps,
            looping: recipe.looping,
            frames: recipe
                .frames
                .iter()
                .map(|frame| RenderSurfaceFrame { time: frame.time })
                .collect(),
        }
    }
}

/// Material channels rendered for a consumer-neutral procedural surface.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedSurfaceMaterial {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub looping: bool,
    pub normal_strength: f32,
    pub frames: Vec<RenderedMaterialFrame>,
}

impl RenderedSurfaceMaterial {
    fn into_tiled(self, input: &RenderedRecipe) -> RenderedMaterial {
        RenderedMaterial {
            name: self.name,
            width: self.width,
            height: self.height,
            tile_width: input.tile_width,
            tile_height: input.tile_height,
            grid_width: input.grid_width,
            grid_height: input.grid_height,
            fps: self.fps,
            looping: self.looping,
            normal_strength: self.normal_strength,
            frames: self.frames,
        }
    }
}

impl RenderedRecipe {
    pub fn tile_rgba(&self, frame: usize, tile_x: u32, tile_y: u32) -> Option<Vec<u8>> {
        if tile_x >= self.grid_width || tile_y >= self.grid_height {
            return None;
        }
        let frame = self.frames.get(frame)?;
        let mut tile = vec![0_u8; (self.tile_width * self.tile_height * 4) as usize];
        let origin_x = tile_x * self.tile_width;
        let origin_y = tile_y * self.tile_height;
        for y in 0..self.tile_height {
            let source_start = (((origin_y + y) * self.width + origin_x) * 4) as usize;
            let source_end = source_start + (self.tile_width * 4) as usize;
            let target_start = (y * self.tile_width * 4) as usize;
            tile[target_start..target_start + (self.tile_width * 4) as usize]
                .copy_from_slice(&frame.rgba[source_start..source_end]);
        }
        Some(tile)
    }

    pub fn tile_height_values(&self, frame: usize, tile_x: u32, tile_y: u32) -> Option<Vec<u8>> {
        if tile_x >= self.grid_width || tile_y >= self.grid_height {
            return None;
        }
        let frame = self.frames.get(frame)?;
        let mut tile = vec![0_u8; (self.tile_width * self.tile_height) as usize];
        let origin_x = tile_x * self.tile_width;
        let origin_y = tile_y * self.tile_height;
        for y in 0..self.tile_height {
            let source_start = ((origin_y + y) * self.width + origin_x) as usize;
            let source_end = source_start + self.tile_width as usize;
            let target_start = (y * self.tile_width) as usize;
            tile[target_start..target_start + self.tile_width as usize]
                .copy_from_slice(&frame.height[source_start..source_end]);
        }
        Some(tile)
    }
}

impl RenderedMaterial {
    pub fn tile_rgba(&self, frame: usize, tile_x: u32, tile_y: u32) -> Option<Vec<u8>> {
        let frame = self.frames.get(frame)?;
        copy_tile_bytes(
            &frame.rgba,
            self.width,
            self.tile_width,
            self.tile_height,
            self.grid_width,
            self.grid_height,
            tile_x,
            tile_y,
            4,
        )
    }

    pub fn tile_material(&self, frame: usize, tile_x: u32, tile_y: u32) -> Option<Vec<[f32; 4]>> {
        if tile_x >= self.grid_width || tile_y >= self.grid_height {
            return None;
        }
        let frame = self.frames.get(frame)?;
        let mut tile = vec![[0.0; 4]; (self.tile_width * self.tile_height) as usize];
        let origin_x = tile_x * self.tile_width;
        let origin_y = tile_y * self.tile_height;
        for y in 0..self.tile_height {
            let source_start = ((origin_y + y) * self.width + origin_x) as usize;
            let target_start = (y * self.tile_width) as usize;
            tile[target_start..target_start + self.tile_width as usize].copy_from_slice(
                &frame.material[source_start..source_start + self.tile_width as usize],
            );
        }
        Some(tile)
    }

    pub fn tile_normal_height(&self, frame: usize, tile_x: u32, tile_y: u32) -> Option<Vec<u8>> {
        let frame = self.frames.get(frame)?;
        copy_tile_bytes(
            &frame.normal_height,
            self.width,
            self.tile_width,
            self.tile_height,
            self.grid_width,
            self.grid_height,
            tile_x,
            tile_y,
            1,
        )
    }

    pub fn blend_layer(
        &mut self,
        layer: &RenderedMaterial,
        masks: &[Vec<f32>],
    ) -> Result<(), RenderError> {
        if self.width != layer.width
            || self.height != layer.height
            || self.frames.len() != layer.frames.len()
            || masks.len() != self.frames.len()
        {
            return Err(RenderError::Dimensions(
                "material layer dimensions or frame counts do not match".to_string(),
            ));
        }
        let pixel_count = (self.width * self.height) as usize;
        let base_normal_strength = self.normal_strength;
        let layer_normal_strength = layer.normal_strength;
        for (frame_index, (base, layer)) in self.frames.iter_mut().zip(&layer.frames).enumerate() {
            let mask = &masks[frame_index];
            if mask.len() != pixel_count {
                return Err(RenderError::Dimensions(
                    "material layer mask dimensions do not match".to_string(),
                ));
            }
            for (index, factor) in mask.iter().copied().enumerate() {
                let factor = factor.clamp(0.0, 1.0);
                let base_color = srgba8_to_linear([
                    base.rgba[index * 4],
                    base.rgba[index * 4 + 1],
                    base.rgba[index * 4 + 2],
                    base.rgba[index * 4 + 3],
                ]);
                let layer_color = srgba8_to_linear([
                    layer.rgba[index * 4],
                    layer.rgba[index * 4 + 1],
                    layer.rgba[index * 4 + 2],
                    layer.rgba[index * 4 + 3],
                ]);
                let mixed = linear_to_srgba8([
                    lerp(base_color[0], layer_color[0], factor),
                    lerp(base_color[1], layer_color[1], factor),
                    lerp(base_color[2], layer_color[2], factor),
                    lerp(base_color[3], layer_color[3], factor),
                ]);
                base.rgba[index * 4..index * 4 + 4].copy_from_slice(&mixed);
                base.palette_indices[index] = if factor <= 0.0 {
                    base.palette_indices[index]
                } else if factor >= 1.0 {
                    layer.palette_indices[index]
                } else {
                    u16::MAX
                };
                for channel in 0..4 {
                    base.material[index][channel] = lerp(
                        base.material[index][channel],
                        layer.material[index][channel],
                        factor,
                    );
                }
                let base_micro =
                    0.5 + (base.normal_height[index] as f32 / 255.0 - 0.5) * base_normal_strength;
                let layer_micro =
                    0.5 + (layer.normal_height[index] as f32 / 255.0 - 0.5) * layer_normal_strength;
                base.normal_height[index] =
                    (lerp(base_micro, layer_micro, factor).clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        self.normal_strength = 1.0;
        Ok(())
    }
}

#[derive(Debug)]
pub enum RenderError {
    Palette(PaletteError),
    Dimensions(String),
    Evaluation(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Palette(error) => write!(f, "{error}"),
            Self::Dimensions(message) | Self::Evaluation(message) => write!(f, "{message}"),
        }
    }
}

impl Error for RenderError {}

impl From<PaletteError> for RenderError {
    fn from(value: PaletteError) -> Self {
        Self::Palette(value)
    }
}

pub struct RecipeRenderer {
    palette: PaletteModel,
}

impl RecipeRenderer {
    pub fn new(palette: &ThePalette) -> Result<Self, RenderError> {
        Ok(Self {
            palette: PaletteModel::new(palette)?,
        })
    }

    pub fn from_palette_model(palette: PaletteModel) -> Self {
        Self { palette }
    }

    /// Creates a renderer for tiles that do not declare `Colorize`.
    ///
    /// Their output bypasses palette mapping and is emitted directly as
    /// grayscale. A palette is still required for colored tiles and material
    /// color previews.
    pub fn grayscale() -> Self {
        let palette = ThePalette::new(vec![
            Some(TheColor::from_u8(0, 0, 0, 255)),
            Some(TheColor::from_u8(255, 255, 255, 255)),
        ]);
        Self::new(&palette).expect("the built-in grayscale palette is not empty")
    }

    pub fn palette_model(&self) -> &PaletteModel {
        &self.palette
    }

    pub fn render(
        &self,
        recipe: &Recipe,
        options: &RenderOptions,
    ) -> Result<RenderedRecipe, RenderError> {
        let width = recipe.size[0]
            .checked_mul(recipe.coverage[0])
            .ok_or_else(|| RenderError::Dimensions("render width overflows".to_string()))?;
        let height = recipe.size[1]
            .checked_mul(recipe.coverage[1])
            .ok_or_else(|| RenderError::Dimensions("render height overflows".to_string()))?;
        if width == 0 || height == 0 {
            return Err(RenderError::Dimensions(
                "render dimensions must be greater than zero".to_string(),
            ));
        }
        if width > MAX_RENDER_DIMENSION || height > MAX_RENDER_DIMENSION {
            return Err(RenderError::Dimensions(format!(
                "render dimensions {width}x{height} exceed {MAX_RENDER_DIMENSION}"
            )));
        }

        let frame_count = recipe.animation.frames.max(1);
        let mut frames = Vec::with_capacity(frame_count as usize);
        for frame_index in 0..frame_count {
            let time = frame_index as f32 / frame_count as f32;
            frames.push(self.render_frame(recipe, width, height, time, options.seed_offset)?);
        }

        Ok(RenderedRecipe {
            name: recipe.name.clone(),
            width,
            height,
            tile_width: recipe.size[0],
            tile_height: recipe.size[1],
            grid_width: recipe.coverage[0],
            grid_height: recipe.coverage[1],
            fps: recipe.animation.fps,
            looping: recipe.animation.looping,
            frames,
        })
    }

    pub fn render_scalar_field(
        &self,
        recipe: &Recipe,
        source: &ScalarSource,
        space: &Domain,
        options: &RenderOptions,
    ) -> Result<Vec<Vec<f32>>, RenderError> {
        let width = recipe.size[0]
            .checked_mul(recipe.coverage[0])
            .ok_or_else(|| RenderError::Dimensions("render width overflows".to_string()))?;
        let height = recipe.size[1]
            .checked_mul(recipe.coverage[1])
            .ok_or_else(|| RenderError::Dimensions("render height overflows".to_string()))?;
        let evaluator = Evaluator {
            recipe,
            seed: mix64(recipe.seed ^ options.seed_offset),
        };
        let frame_count = recipe.animation.frames.max(1);
        let mut frames = Vec::with_capacity(frame_count as usize);
        for frame_index in 0..frame_count {
            let time = frame_index as f32 / frame_count as f32;
            let mut values = vec![0.0; (width * height) as usize];
            for y in 0..height {
                for x in 0..width {
                    let context = EvalContext {
                        uv: [
                            wrap_coordinate((x as f32 + 0.5) / width as f32, recipe.wrap),
                            wrap_coordinate((y as f32 + 0.5) / height as f32, recipe.wrap),
                        ],
                        time,
                        current_id: None,
                        input_height: 0.0,
                        pattern_bindings: [None; 8],
                    };
                    let context = evaluator.context_in_domain(space, context, &mut Vec::new())?;
                    values[(y * width + x) as usize] = evaluator
                        .scalar(source, context, &mut Vec::new())?
                        .clamp(0.0, 1.0);
                }
            }
            frames.push(values);
        }
        Ok(frames)
    }

    pub fn render_material(
        &self,
        material: &MaterialRecipe,
        input: &RenderedRecipe,
        options: &RenderOptions,
    ) -> Result<RenderedMaterial, RenderError> {
        self.render_material_internal(material, input, options, false, None, [1.0, 1.0])
    }

    /// Renders a material on a consumer-neutral surface.
    ///
    /// Coordinates are normalized across the complete surface. This is the
    /// reusable entry point for consumers that do not need Tile-local spaces.
    pub fn render_material_on_surface(
        &self,
        material: &MaterialRecipe,
        surface: &RenderSurface,
        options: &RenderOptions,
    ) -> Result<RenderedSurfaceMaterial, RenderError> {
        self.render_material_surface_internal(material, surface, options, false, None, [1.0, 1.0])
    }

    /// Renders a material in a coordinate space supplied by a tile.
    ///
    /// Pattern-local spaces reset material coordinates for every pattern unit
    /// and expose that unit's stable identity as `Id` to the complete material.
    pub fn render_material_in_space(
        &self,
        material: &MaterialRecipe,
        input: &RenderedRecipe,
        tile: &Recipe,
        space: &Domain,
        tiling: [f32; 2],
        options: &RenderOptions,
    ) -> Result<RenderedMaterial, RenderError> {
        self.render_material_internal(material, input, options, false, Some((tile, space)), tiling)
    }

    /// Renders a standalone material preview. An optional material `Output`
    /// overrides only the preview image and is ignored by `render_material`.
    pub fn render_material_preview(
        &self,
        material: &MaterialRecipe,
        input: &RenderedRecipe,
        options: &RenderOptions,
    ) -> Result<RenderedMaterial, RenderError> {
        self.render_material_internal(material, input, options, true, None, [1.0, 1.0])
    }

    fn render_material_internal(
        &self,
        material: &MaterialRecipe,
        input: &RenderedRecipe,
        options: &RenderOptions,
        use_debug_output: bool,
        tile_space: Option<(&Recipe, &Domain)>,
        tiling: [f32; 2],
    ) -> Result<RenderedMaterial, RenderError> {
        let surface = RenderSurface::from(input);
        self.render_material_surface_internal(
            material,
            &surface,
            options,
            use_debug_output,
            tile_space,
            tiling,
        )
        .map(|rendered| rendered.into_tiled(input))
    }

    fn render_material_surface_internal(
        &self,
        material: &MaterialRecipe,
        surface: &RenderSurface,
        options: &RenderOptions,
        use_debug_output: bool,
        tile_space: Option<(&Recipe, &Domain)>,
        tiling: [f32; 2],
    ) -> Result<RenderedSurfaceMaterial, RenderError> {
        let evaluation_recipe = Recipe {
            name: material.name.clone(),
            wrap: material.wrap,
            seed: material.seed,
            fields: material.fields.clone(),
            patterns: material.patterns.clone(),
            ..Recipe::default()
        };
        let evaluator = Evaluator {
            recipe: &evaluation_recipe,
            seed: mix64(material.seed ^ options.seed_offset),
        };
        let tile_evaluator = tile_space.map(|(tile, _)| Evaluator {
            recipe: tile,
            seed: mix64(tile.seed ^ options.seed_offset),
        });
        let pixel_count = (surface.width * surface.height) as usize;
        let mut frames = Vec::with_capacity(surface.frames.len());
        for surface_frame in &surface.frames {
            let mut rgba = vec![0_u8; pixel_count * 4];
            let mut palette_indices = vec![u16::MAX; pixel_count];
            let mut material_values = vec![[0.0_f32; 4]; pixel_count];
            let mut normal_height = vec![128_u8; pixel_count];
            for y in 0..surface.height {
                for x in 0..surface.width {
                    let index = (y * surface.width + x) as usize;
                    let global_uv = [
                        (x as f32 + 0.5) / surface.width as f32,
                        (y as f32 + 0.5) / surface.height as f32,
                    ];
                    let surface_uv = surface.mapping.map(global_uv);
                    let mut context = EvalContext {
                        uv: [
                            wrap_coordinate(surface_uv[0] * tiling[0], material.wrap),
                            wrap_coordinate(surface_uv[1] * tiling[1], material.wrap),
                        ],
                        time: surface_frame.time,
                        current_id: Some(0),
                        input_height: 0.0,
                        pattern_bindings: [None; 8],
                    };
                    if let (Some((tile, space)), Some(tile_evaluator)) =
                        (tile_space, tile_evaluator.as_ref())
                        && let Domain::PatternLocal(_) = space
                    {
                        let tile_context = EvalContext {
                            uv: global_uv.map(|coordinate| wrap_coordinate(coordinate, tile.wrap)),
                            time: surface_frame.time,
                            current_id: None,
                            input_height: 0.0,
                            pattern_bindings: [None; 8],
                        };
                        let local = tile_evaluator.context_in_domain(
                            space,
                            tile_context,
                            &mut Vec::new(),
                        )?;
                        context.uv = [
                            wrap_coordinate(local.uv[0] * tiling[0], material.wrap),
                            wrap_coordinate(local.uv[1] * tiling[1], material.wrap),
                        ];
                        context.current_id = Some(local.current_id.unwrap_or(0));
                    }
                    let color = self.evaluate_material_color(
                        material,
                        &material.surface.color,
                        &evaluator,
                        context,
                        &mut Vec::new(),
                    )?;
                    let (mut pixel, mut palette_index) =
                        self.finish_material_color(color, material.surface.palette);
                    if use_debug_output {
                        match &material.output {
                            Some(MaterialOutput::Value { source, space }) => {
                                let context =
                                    evaluator.context_in_domain(space, context, &mut Vec::new())?;
                                let value = evaluator
                                    .scalar(source, context, &mut Vec::new())?
                                    .clamp(0.0, 1.0);
                                let gray = (value * 255.0).round() as u8;
                                pixel = [gray, gray, gray, 255];
                                palette_index = u16::MAX;
                            }
                            Some(MaterialOutput::Color { source, space }) => {
                                let context =
                                    evaluator.context_in_domain(space, context, &mut Vec::new())?;
                                let color = self.evaluate_material_color(
                                    material,
                                    source,
                                    &evaluator,
                                    context,
                                    &mut Vec::new(),
                                )?;
                                (pixel, palette_index) =
                                    self.finish_material_color(color, material.surface.palette);
                            }
                            None => {}
                        }
                    }
                    rgba[index * 4..index * 4 + 4].copy_from_slice(&pixel);
                    palette_indices[index] = palette_index;
                    for (channel, source) in [
                        &material.surface.roughness,
                        &material.surface.metallic,
                        &material.surface.opacity,
                        &material.surface.emissive,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        material_values[index][channel] = evaluator
                            .scalar(source, context, &mut Vec::new())?
                            .clamp(0.0, 1.0);
                    }
                    if let Some(normal) = &material.surface.normal {
                        let normal = evaluator
                            .scalar(normal, context, &mut Vec::new())?
                            .clamp(0.0, 1.0);
                        normal_height[index] = (normal * 255.0).round() as u8;
                    }
                }
            }
            frames.push(RenderedMaterialFrame {
                rgba,
                palette_indices,
                material: material_values,
                normal_height,
                time: surface_frame.time,
            });
        }

        Ok(RenderedSurfaceMaterial {
            name: material.name.clone(),
            width: surface.width,
            height: surface.height,
            fps: surface.fps,
            looping: surface.looping,
            normal_strength: if material.surface.normal.is_some() {
                material.surface.normal_strength
            } else {
                0.0
            },
            frames,
        })
    }

    fn evaluate_material_color(
        &self,
        material: &MaterialRecipe,
        source: &ColorSource,
        evaluator: &Evaluator<'_>,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<[f32; 4], RenderError> {
        match source {
            ColorSource::Exact(rgba) => Ok(srgba8_to_linear(*rgba)),
            ColorSource::Nearest(rgba) => {
                let authored = rgba.map(|component| component as f32 / 255.0);
                let palette_index = self.palette.closest(authored);
                Ok(srgba8_to_linear(self.palette.rgba(palette_index)))
            }
            ColorSource::Reference(name) => {
                let key = format!("color:{}", name.to_ascii_lowercase());
                enter(stack, &key)?;
                let result = material
                    .colors
                    .iter()
                    .find(|color| color.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| RenderError::Evaluation(format!("unknown color '{name}'")))
                    .and_then(|color| {
                        self.evaluate_material_color(
                            material,
                            &color.source,
                            evaluator,
                            context,
                            stack,
                        )
                    });
                stack.pop();
                result
            }
            ColorSource::Mix { a, b, factor } => {
                let a = self.evaluate_material_color(material, a, evaluator, context, stack)?;
                let b = self.evaluate_material_color(material, b, evaluator, context, stack)?;
                let factor = evaluator
                    .scalar(factor, context, &mut Vec::new())?
                    .clamp(0.0, 1.0);
                Ok([
                    lerp(a[0], b[0], factor),
                    lerp(a[1], b[1], factor),
                    lerp(a[2], b[2], factor),
                    lerp(a[3], b[3], factor),
                ])
            }
        }
    }

    fn finish_material_color(&self, linear: [f32; 4], palette_mode: PaletteMode) -> ([u8; 4], u16) {
        let rgba = linear_to_srgba8(linear);
        if palette_mode == PaletteMode::Strict {
            let index = self
                .palette
                .closest(rgba.map(|component| component as f32 / 255.0));
            (
                self.palette.rgba(index),
                self.palette.source_index(index) as u16,
            )
        } else {
            (rgba, u16::MAX)
        }
    }

    fn colorize_values(
        &self,
        colorize: &Colorize,
        values: &[f32],
        width: u32,
    ) -> (Vec<u8>, Vec<u16>) {
        let (range_min, range_max) = match colorize.range {
            ColorRange::Auto => values
                .iter()
                .copied()
                .fold((f32::MAX, f32::MIN), |(min, max), value| {
                    (min.min(value), max.max(value))
                }),
            ColorRange::Fixed(range) => (range[0], range[1]),
        };
        let range_width = (range_max - range_min).abs().max(0.000_01);
        let steps = colorize.steps.clamp(2, 256) as usize;
        let ramp = if let Some(base) = colorize.base {
            match colorize.palette {
                PaletteMode::Strict => ColorRamp::Palette(self.palette.resolve_anchored_ramp(
                    base,
                    steps,
                    colorize.brightness,
                    colorize.saturation,
                    colorize.hue,
                )),
                PaletteMode::BaseOnly => ColorRamp::Rgba(self.palette.resolve_base_only_ramp(
                    base,
                    steps,
                    colorize.brightness,
                    colorize.saturation,
                    colorize.hue,
                )),
            }
        } else {
            let ramp_positions = (0..steps)
                .map(|index| {
                    let factor = index as f32 / (steps - 1) as f32;
                    lerp(colorize.ramp_range[0], colorize.ramp_range[1], factor)
                })
                .collect::<Vec<_>>();
            ColorRamp::Palette(self.palette.resolve_ramp(
                colorize.family,
                &ramp_positions,
                colorize.saturation_range,
            ))
        };
        let mut rgba = vec![0_u8; values.len() * 4];
        let mut palette_indices = vec![u16::MAX; values.len()];
        for (index, value) in values.iter().copied().enumerate() {
            let mut normalized = ((value - range_min) / range_width).clamp(0.0, 0.999_999);
            if colorize.dither {
                let x = index as u32 % width;
                let y = index as u32 / width;
                normalized =
                    (normalized + (bayer4(x, y) - 0.5) / steps as f32).clamp(0.0, 0.999_999);
            }
            let slot = (normalized * steps as f32).floor() as usize;
            match &ramp {
                ColorRamp::Palette(ramp) => {
                    let palette_index = ramp[slot.min(ramp.len() - 1)];
                    rgba[index * 4..index * 4 + 4]
                        .copy_from_slice(&self.palette.rgba(palette_index));
                    palette_indices[index] = self.palette.source_index(palette_index) as u16;
                }
                ColorRamp::Rgba(ramp) => {
                    rgba[index * 4..index * 4 + 4].copy_from_slice(&ramp[slot.min(ramp.len() - 1)]);
                }
            }
        }
        (rgba, palette_indices)
    }

    fn render_frame(
        &self,
        recipe: &Recipe,
        width: u32,
        height: u32,
        time: f32,
        seed_offset: u64,
    ) -> Result<RenderedFrame, RenderError> {
        let pixel_count = (width * height) as usize;
        let mut scalar_height = vec![0.0_f32; pixel_count];
        let evaluator = Evaluator {
            recipe,
            seed: mix64(recipe.seed ^ seed_offset),
        };
        let pixelate = recipe.pixelate.max(1);

        for y in 0..height {
            for x in 0..width {
                let sample_x = (x / pixelate) * pixelate + pixelate / 2;
                let sample_y = (y / pixelate) * pixelate + pixelate / 2;
                let raw_u = (sample_x.min(width - 1) as f32 + 0.5) / width as f32;
                let raw_v = (sample_y.min(height - 1) as f32 + 0.5) / height as f32;
                let context = EvalContext {
                    uv: [
                        wrap_coordinate(raw_u, recipe.wrap),
                        wrap_coordinate(raw_v, recipe.wrap),
                    ],
                    time,
                    current_id: None,
                    input_height: 0.0,
                    pattern_bindings: [None; 8],
                };
                scalar_height[(y * width + x) as usize] = evaluator
                    .output_scalar(&recipe.output, context, &mut Vec::new())?
                    .clamp(0.0, 1.0);
            }
        }

        let (rgba, palette_indices) = recipe.colorize.as_ref().map_or_else(
            || grayscale_values(&scalar_height),
            |colorize| self.colorize_values(colorize, &scalar_height, width),
        );
        let coverage = vec![255_u8; pixel_count];
        let height_values = scalar_height
            .into_iter()
            .map(|value| (value * 255.0).round() as u8)
            .collect();

        Ok(RenderedFrame {
            rgba,
            palette_indices,
            coverage,
            height: height_values,
            time,
        })
    }
}

#[derive(Clone, Copy)]
struct EvalContext {
    uv: [f32; 2],
    time: f32,
    current_id: Option<u64>,
    input_height: f32,
    pattern_bindings: [Option<PatternBinding>; 8],
}

#[derive(Clone, Copy)]
struct PatternBinding {
    pattern_index: usize,
    id: u64,
    local: [f32; 2],
}

impl EvalContext {
    fn at(self, uv: [f32; 2]) -> Self {
        Self { uv, ..self }
    }

    fn with_id(self, current_id: u64) -> Self {
        Self {
            current_id: Some(current_id),
            ..self
        }
    }

    fn with_pattern(mut self, pattern_index: usize, id: u64, local: [f32; 2]) -> Self {
        let binding = PatternBinding {
            pattern_index,
            id,
            local,
        };
        if let Some(slot) = self.pattern_bindings.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(binding);
        } else {
            self.pattern_bindings[self.pattern_bindings.len() - 1] = Some(binding);
        }
        self
    }

    fn pattern_binding(self, pattern_index: usize) -> Option<PatternBinding> {
        self.pattern_bindings
            .iter()
            .rev()
            .flatten()
            .find(|binding| binding.pattern_index == pattern_index)
            .copied()
    }
}

#[derive(Clone, Copy, Debug)]
struct PatternSample {
    height: f32,
    edge: f32,
    center: f32,
    id: u64,
    local: [f32; 2],
}

struct Evaluator<'a> {
    recipe: &'a Recipe,
    seed: u64,
}

impl Evaluator<'_> {
    fn output_scalar(
        &self,
        output: &Output,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<f32, RenderError> {
        let context = self.context_in_domain(&output.space, context, stack)?;
        self.scalar(&output.height, context, stack)
    }

    fn context_in_domain(
        &self,
        domain: &Domain,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<EvalContext, RenderError> {
        Ok(match domain {
            Domain::Global => context,
            Domain::PatternLocal(name) => {
                let pattern_index = self.pattern_index(name)?;
                let binding = self.pattern_coordinates(pattern_index, context, stack)?;
                context
                    .at(binding.local)
                    .with_pattern(pattern_index, binding.id, binding.local)
                    .with_id(binding.id)
            }
        })
    }

    fn scalar(
        &self,
        source: &ScalarSource,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<f32, RenderError> {
        match source {
            ScalarSource::Constant(value) => Ok(*value),
            ScalarSource::Coordinate(channel) => Ok(match channel {
                CoordinateChannel::U => context.uv[0],
                CoordinateChannel::V => context.uv[1],
                CoordinateChannel::Radius => (context.uv[0] - 0.5).hypot(context.uv[1] - 0.5),
                CoordinateChannel::Angle => {
                    ((context.uv[1] - 0.5).atan2(context.uv[0] - 0.5) / std::f32::consts::TAU + 1.0)
                        .rem_euclid(1.0)
                }
            }),
            ScalarSource::InputHeight => Ok(context.input_height),
            ScalarSource::Field(name) => self.field(name, context, stack),
            ScalarSource::Pattern { name, channel } => {
                let sample = self.pattern(name, context, stack)?;
                Ok(match channel {
                    PatternChannel::Height => sample.height,
                    PatternChannel::Edge => sample.edge,
                    PatternChannel::Center => sample.center,
                })
            }
            ScalarSource::Geometry { name, channel } => match channel {
                GeometryChannel::Distance => self.geometry_distance(name, context),
            },
            ScalarSource::RandomId { id, min, max, seed } => {
                let id = self.id(id, context, stack)?;
                Ok(lerp(
                    *min,
                    *max,
                    random01(mix64(self.seed ^ id ^ seed.rotate_left(23))),
                ))
            }
            ScalarSource::Wave {
                min,
                max,
                cycles,
                phase,
            } => {
                let angle = (context.time * cycles + phase) * std::f32::consts::TAU;
                Ok(lerp(*min, *max, angle.sin() * 0.5 + 0.5))
            }
            ScalarSource::Unary { op, source } => {
                let source = self.scalar(source, context, stack)?;
                Ok(match op {
                    UnaryOperator::Negate => -source,
                    UnaryOperator::Abs => source.abs(),
                    UnaryOperator::Invert => 1.0 - source,
                    UnaryOperator::Sin => (source * std::f32::consts::TAU).sin(),
                    UnaryOperator::Cos => (source * std::f32::consts::TAU).cos(),
                    UnaryOperator::Fract => source.rem_euclid(1.0),
                    UnaryOperator::Sqrt => source.max(0.0).sqrt(),
                })
            }
            ScalarSource::Binary { op, left, right } => {
                let left = self.scalar(left, context, stack)?;
                let right = self.scalar(right, context, stack)?;
                Ok(match op {
                    BinaryOperator::Add => left + right,
                    BinaryOperator::Subtract => left - right,
                    BinaryOperator::Multiply => left * right,
                    BinaryOperator::Divide => {
                        if right.abs() <= 0.000_001 {
                            0.0
                        } else {
                            left / right
                        }
                    }
                    BinaryOperator::Min => left.min(right),
                    BinaryOperator::Max => left.max(right),
                    BinaryOperator::Pow => left.max(0.0).powf(right),
                })
            }
            ScalarSource::Clamp { source, min, max } => {
                let source = self.scalar(source, context, stack)?;
                let min = self.scalar(min, context, stack)?;
                let max = self.scalar(max, context, stack)?;
                Ok(source.clamp(min.min(max), min.max(max)))
            }
            ScalarSource::Mix { a, b, factor } => {
                let a = self.scalar(a, context, stack)?;
                let b = self.scalar(b, context, stack)?;
                let factor = self.scalar(factor, context, stack)?;
                Ok(lerp(a, b, factor.clamp(0.0, 1.0)))
            }
            ScalarSource::Smoothstep { min, max, source } => {
                let min = self.scalar(min, context, stack)?;
                let max = self.scalar(max, context, stack)?;
                let source = self.scalar(source, context, stack)?;
                let factor = ((source - min) / (max - min).abs().max(0.000_001)).clamp(0.0, 1.0);
                Ok(smoothstep(factor))
            }
        }
    }

    fn geometry_distance(&self, name: &str, context: EvalContext) -> Result<f32, RenderError> {
        let feature = self
            .recipe
            .geometry
            .iter()
            .find(|feature| match feature {
                GeometryFeature::Box(geometry_box) => geometry_box.name.eq_ignore_ascii_case(name),
            })
            .ok_or_else(|| RenderError::Evaluation(format!("unknown Geometry feature '{name}'")))?;

        match feature {
            GeometryFeature::Box(geometry_box) => {
                let placement_x =
                    (context.uv[0] * self.recipe.coverage[0].max(1) as f32).rem_euclid(1.0);
                let placement_y = context.uv[1] * self.recipe.coverage[1].max(1) as f32;
                let mut distance = f32::INFINITY;
                for repeat_x in 0..geometry_box.repeat[0] {
                    for repeat_y in 0..geometry_box.repeat[1] {
                        let min = [
                            geometry_box.position[0] + repeat_x as f32 * geometry_box.spacing[0],
                            geometry_box.position[1] + repeat_y as f32 * geometry_box.spacing[1],
                        ];
                        let center = [
                            min[0] + geometry_box.size[0] * 0.5,
                            min[1] + geometry_box.size[1] * 0.5,
                        ];
                        let half = [geometry_box.size[0] * 0.5, geometry_box.size[1] * 0.5];
                        let q = [
                            (placement_x - center[0]).abs() - half[0],
                            (placement_y - center[1]).abs() - half[1],
                        ];
                        let outside = q[0].max(0.0).hypot(q[1].max(0.0));
                        let inside = q[0].max(q[1]).min(0.0);
                        distance = distance.min(outside + inside);
                    }
                }
                Ok(distance)
            }
        }
    }

    fn id(
        &self,
        source: &IdSource,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<u64, RenderError> {
        match source {
            IdSource::Current => context.current_id.ok_or_else(|| {
                RenderError::Evaluation(
                    "Id is only available while evaluating a pattern unit".to_string(),
                )
            }),
            IdSource::Pattern(name) => {
                let pattern_index = self.pattern_index(name)?;
                if let Some(binding) = context.pattern_binding(pattern_index) {
                    Ok(binding.id)
                } else {
                    Ok(self.pattern_coordinates(pattern_index, context, stack)?.id)
                }
            }
        }
    }

    fn field(
        &self,
        name: &str,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<f32, RenderError> {
        let key = format!("field:{}", name.to_ascii_lowercase());
        enter(stack, &key)?;
        let result = self
            .recipe
            .fields
            .iter()
            .find(|field| field.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| RenderError::Evaluation(format!("unknown field '{name}'")))
            .and_then(|definition| match definition {
                FieldDefinition::Noise(noise) => {
                    let (uv, domain_id) = self.domain(&noise.domain, context, stack)?;
                    let key_id = noise
                        .key
                        .as_ref()
                        .map(|source| match source {
                            IdSource::Current => Ok(context.current_id.unwrap_or(0)),
                            IdSource::Pattern(_) => self.id(source, context, stack),
                        })
                        .transpose()?
                        .unwrap_or(0);
                    let seed = mix64(
                        self.seed ^ noise.seed ^ mix64(domain_id) ^ mix64(key_id.rotate_left(11)),
                    );
                    Ok(fractal_noise(
                        uv,
                        noise.kind,
                        noise.fractal,
                        noise.scale,
                        noise.octaves,
                        noise.persistence,
                        seed,
                    ))
                }
                FieldDefinition::Height(height) => {
                    let mut value = self.scalar(&height.source, context, stack)?;
                    for operation in &height.operations {
                        value = self.height_operation(value, operation, context, stack)?;
                    }
                    Ok(value)
                }
                FieldDefinition::Value(value) => self.scalar(&value.source, context, stack),
            });
        stack.pop();
        result
    }

    fn height_operation(
        &self,
        value: f32,
        operation: &HeightOperation,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<f32, RenderError> {
        match operation {
            HeightOperation::Shape {
                contrast,
                bias,
                plateau,
                rim,
            } => {
                let contrast = self.scalar(contrast, context, stack)?.clamp(0.1, 8.0);
                let bias = self.scalar(bias, context, stack)?.clamp(-1.0, 1.0);
                let plateau = self.scalar(plateau, context, stack)?.clamp(0.0, 4.0);
                let rim = self.scalar(rim, context, stack)?.clamp(0.0, 4.0);
                Ok(shape_height(value, contrast, bias, plateau, rim))
            }
            HeightOperation::Combine {
                mode,
                source,
                amount,
            } => {
                let source = self.scalar(source, context, stack)?;
                let amount = self.scalar(amount, context, stack)?;
                Ok(match mode {
                    CombineMode::Add => value + source * amount,
                    CombineMode::Subtract => value - source * amount,
                    CombineMode::Multiply => value * lerp(1.0, source, amount.clamp(0.0, 1.0)),
                    CombineMode::Min => lerp(value, value.min(source), amount.clamp(0.0, 1.0)),
                    CombineMode::Max => lerp(value, value.max(source), amount.clamp(0.0, 1.0)),
                })
            }
            HeightOperation::Clamp { min, max } => {
                let min = self.scalar(min, context, stack)?;
                let max = self.scalar(max, context, stack)?;
                Ok(value.clamp(min.min(max), min.max(max)))
            }
            HeightOperation::Remap { from, to } => {
                let factor =
                    ((value - from[0]) / (from[1] - from[0]).abs().max(0.000_01)).clamp(0.0, 1.0);
                Ok(lerp(to[0], to[1], factor))
            }
            HeightOperation::Terrace { steps, smoothness } => {
                let segments = steps.saturating_sub(1).max(1) as f32;
                let terraced = (value.clamp(0.0, 1.0) * segments).round() / segments;
                Ok(lerp(terraced, value, smoothness.clamp(0.0, 1.0)))
            }
            HeightOperation::Invert => Ok(1.0 - value),
        }
    }

    fn pattern(
        &self,
        name: &str,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<PatternSample, RenderError> {
        let pattern_index = self.pattern_index(name)?;
        self.pattern_by_index(pattern_index, context, stack)
    }

    fn pattern_index(&self, name: &str) -> Result<usize, RenderError> {
        self.recipe
            .patterns
            .iter()
            .position(|pattern| pattern.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| RenderError::Evaluation(format!("unknown pattern '{name}'")))
    }

    fn pattern_by_index(
        &self,
        pattern_index: usize,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<PatternSample, RenderError> {
        let pattern = &self.recipe.patterns[pattern_index];
        let key = format!("pattern:{}", pattern.name.to_ascii_lowercase());
        enter(stack, &key)?;
        let result = self.evaluate_pattern(pattern_index, pattern, context, stack);
        stack.pop();
        result
    }

    fn evaluate_pattern(
        &self,
        pattern_index: usize,
        pattern: &PatternDefinition,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<PatternSample, RenderError> {
        let (uv, seed) = self.pattern_space(pattern, context, stack)?;
        match &pattern.kind {
            PatternKind::Bricks {
                columns,
                rows,
                stagger,
                gap,
                rounding,
                rotation,
                size_variation,
                falloff,
                seed: local_seed,
            } => {
                let cell = brick_cell(uv, *columns, *rows, *stagger, mix64(seed ^ local_seed));
                let unit_context = context.with_id(cell.id);
                let gap = self.scalar(gap, unit_context, stack)?.clamp(0.0, 0.95);
                let rounding = self
                    .scalar(rounding, unit_context, stack)?
                    .abs()
                    .clamp(0.0, 0.49);
                let rotation = self.scalar(rotation, unit_context, stack)?;
                let falloff = self.scalar(falloff, unit_context, stack)?.clamp(0.1, 8.0);
                let rotated = brick_rotated_local(cell, rotation);
                let (bevel, perturb, perturb_amount) = self.pattern_modifiers(
                    pattern_index,
                    pattern,
                    cell.id,
                    rotated,
                    context,
                    stack,
                )?;
                Ok(brick_pattern(
                    cell,
                    gap,
                    bevel,
                    rounding,
                    rotation,
                    *size_variation,
                    perturb,
                    perturb_amount,
                    falloff,
                ))
            }
            PatternKind::Voronoi {
                cells,
                jitter,
                falloff,
                seed: local_seed,
            } => {
                let local_seed = mix64(seed ^ local_seed);
                let base =
                    voronoi_pattern(uv, *cells, *jitter, *falloff, local_seed, 0.08, 0.5, 0.0);
                let (bevel, perturb, perturb_amount) = self.pattern_modifiers(
                    pattern_index,
                    pattern,
                    base.id,
                    base.local,
                    context,
                    stack,
                )?;
                Ok(voronoi_pattern(
                    uv,
                    *cells,
                    *jitter,
                    *falloff,
                    local_seed,
                    bevel,
                    perturb,
                    perturb_amount,
                ))
            }
            PatternKind::Discs {
                cells,
                jitter,
                radius,
                falloff,
                seed: local_seed,
            } => {
                let jitter = self.scalar(jitter, context, stack)?.clamp(0.0, 1.0);
                let radius = self.scalar(radius, context, stack)?.clamp(0.01, 2.0);
                let falloff = self.scalar(falloff, context, stack)?.clamp(0.1, 8.0);
                let local_seed = mix64(seed ^ local_seed);
                let base = disc_pattern(
                    uv, *cells, jitter, radius, falloff, local_seed, 0.08, 0.5, 0.0,
                );
                let (bevel, perturb, perturb_amount) = self.pattern_modifiers(
                    pattern_index,
                    pattern,
                    base.id,
                    base.local,
                    context,
                    stack,
                )?;
                Ok(disc_pattern(
                    uv,
                    *cells,
                    jitter,
                    radius,
                    falloff,
                    local_seed,
                    bevel,
                    perturb,
                    perturb_amount,
                ))
            }
        }
    }

    fn pattern_coordinates(
        &self,
        pattern_index: usize,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<PatternBinding, RenderError> {
        if let Some(binding) = context.pattern_binding(pattern_index) {
            return Ok(binding);
        }

        let pattern = &self.recipe.patterns[pattern_index];
        let key = format!("placement:{}", pattern.name.to_ascii_lowercase());
        enter(stack, &key)?;
        let result = (|| {
            let (uv, seed) = self.pattern_space(pattern, context, stack)?;
            let (id, local) = match &pattern.kind {
                PatternKind::Bricks {
                    columns,
                    rows,
                    stagger,
                    rotation,
                    seed: local_seed,
                    ..
                } => {
                    let cell = brick_cell(uv, *columns, *rows, *stagger, mix64(seed ^ local_seed));
                    let unit_context = context
                        .with_pattern(pattern_index, cell.id, cell.local)
                        .with_id(cell.id);
                    let rotation = self.scalar(rotation, unit_context, stack)?;
                    (cell.id, brick_rotated_local(cell, rotation))
                }
                PatternKind::Voronoi {
                    cells,
                    jitter,
                    seed: local_seed,
                    ..
                } => {
                    let local_seed = mix64(seed ^ local_seed);
                    let sample =
                        voronoi_pattern(uv, *cells, *jitter, 1.0, local_seed, 0.08, 0.5, 0.0);
                    (sample.id, sample.local)
                }
                PatternKind::Discs {
                    cells,
                    jitter,
                    seed: local_seed,
                    ..
                } => {
                    let jitter = self.scalar(jitter, context, stack)?.clamp(0.0, 1.0);
                    let local_seed = mix64(seed ^ local_seed);
                    let sample =
                        disc_pattern(uv, *cells, jitter, 1.0, 1.0, local_seed, 0.08, 0.5, 0.0);
                    (sample.id, sample.local)
                }
            };
            Ok(PatternBinding {
                pattern_index,
                id,
                local,
            })
        })();
        stack.pop();
        result
    }

    fn pattern_space(
        &self,
        pattern: &PatternDefinition,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<([f32; 2], u64), RenderError> {
        let (mut uv, domain_id) = self.domain(&pattern.domain, context, stack)?;
        let key_id = pattern
            .key
            .as_ref()
            .map(|source| self.id(source, context, stack))
            .transpose()?
            .unwrap_or(0);
        let seed = mix64(
            self.seed
                ^ mix64(domain_id)
                ^ mix64(key_id.rotate_left(13))
                ^ hash_bytes(pattern.name.as_bytes()),
        );
        if let Some(warp) = &pattern.warp {
            let first = self.scalar(&warp.source, context, stack)?;
            let shifted = context.at([
                wrap_coordinate(context.uv[0] + 0.173, self.recipe.wrap),
                wrap_coordinate(context.uv[1] + 0.317, self.recipe.wrap),
            ]);
            let second = self.scalar(&warp.source, shifted, stack)?;
            uv[0] += (first - 0.5) * 2.0 * warp.amount;
            uv[1] += (second - 0.5) * 2.0 * warp.amount;
        }
        Ok((
            [
                wrap_coordinate(uv[0], WrapMode::Repeat),
                wrap_coordinate(uv[1], WrapMode::Repeat),
            ],
            seed,
        ))
    }

    fn pattern_modifiers(
        &self,
        pattern_index: usize,
        pattern: &PatternDefinition,
        unit_id: u64,
        local: [f32; 2],
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<(f32, f32, f32), RenderError> {
        let unit_context = context
            .with_pattern(pattern_index, unit_id, local)
            .with_id(unit_id);
        let bevel = self
            .scalar(&pattern.bevel, unit_context, stack)?
            .abs()
            .clamp(0.0, 1.0);
        let Some(perturb) = &pattern.perturb else {
            return Ok((bevel, 0.5, 0.0));
        };
        let amount = self
            .scalar(&perturb.amount, unit_context, stack)?
            .abs()
            .clamp(0.0, 0.5);
        let value = self.scalar(&perturb.source, unit_context.at(local), stack)?;
        Ok((bevel, value, amount))
    }

    fn domain(
        &self,
        domain: &Domain,
        context: EvalContext,
        stack: &mut Vec<String>,
    ) -> Result<([f32; 2], u64), RenderError> {
        match domain {
            Domain::Global => Ok((context.uv, 0)),
            Domain::PatternLocal(name) => {
                let pattern_index = self.pattern_index(name)?;
                if let Some(binding) = context.pattern_binding(pattern_index) {
                    Ok((binding.local, binding.id))
                } else {
                    let binding = self.pattern_coordinates(pattern_index, context, stack)?;
                    Ok((binding.local, binding.id))
                }
            }
        }
    }
}

fn enter(stack: &mut Vec<String>, key: &str) -> Result<(), RenderError> {
    if stack.iter().any(|entry| entry == key) {
        return Err(RenderError::Evaluation(format!(
            "cyclic field dependency involving '{key}'"
        )));
    }
    stack.push(key.to_string());
    Ok(())
}

#[derive(Clone, Copy)]
struct BrickCell {
    local: [f32; 2],
    id: u64,
}

fn brick_cell(uv: [f32; 2], columns: u32, rows: u32, stagger: f32, seed: u64) -> BrickCell {
    let columns = columns.max(1);
    let rows = rows.max(1);
    let scaled_y = uv[1].rem_euclid(1.0) * rows as f32;
    let row = scaled_y.floor() as i32;
    let offset = if row.rem_euclid(2) == 1 { stagger } else { 0.0 };
    let scaled_x = uv[0].rem_euclid(1.0) * columns as f32 + offset;
    let column = scaled_x.floor() as i32;
    let local = [scaled_x.fract(), scaled_y.fract()];
    BrickCell {
        local,
        id: hash_cell(
            column.rem_euclid(columns as i32),
            row.rem_euclid(rows as i32),
            seed,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn brick_pattern(
    cell: BrickCell,
    gap: f32,
    bevel: f32,
    rounding: f32,
    rotation_degrees: f32,
    size_variation: [f32; 2],
    perturb: f32,
    perturb_amount: f32,
    falloff: f32,
) -> PatternSample {
    let local = brick_rotated_local(cell, rotation_degrees);
    let rotated = [local[0] - 0.5, local[1] - 0.5];
    let base_half = (0.5 - gap * 0.5).max(0.001);
    let variation_x =
        (random01(mix64(cell.id ^ 0x7369_7a65)) * 2.0 - 1.0) * size_variation[0].clamp(0.0, 0.45);
    let variation_y =
        (random01(mix64(cell.id ^ 0x7661_7279)) * 2.0 - 1.0) * size_variation[1].clamp(0.0, 0.45);
    let half_x = (base_half * (1.0 + variation_x)).clamp(0.02, 0.5);
    let half_y = (base_half * (1.0 + variation_y)).clamp(0.02, 0.5);
    let radius = rounding.min(half_x.min(half_y));
    let qx = rotated[0].abs() - (half_x - radius);
    let qy = rotated[1].abs() - (half_y - radius);
    let base_distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
    let signed_distance = base_distance + (perturb - 0.5) * 2.0 * perturb_amount.clamp(0.0, 0.5);
    let inside = signed_distance <= 0.0;
    let height = if inside {
        let bevel_width = half_x.min(half_y) * bevel.clamp(0.0, 1.0);
        if bevel_width <= f32::EPSILON {
            1.0
        } else {
            ((-signed_distance) / bevel_width)
                .clamp(0.0, 1.0)
                .powf(falloff)
        }
    } else {
        0.0
    };
    let dx = (rotated[0].abs() / half_x).clamp(0.0, 1.0);
    let dy = (rotated[1].abs() / half_y).clamp(0.0, 1.0);
    PatternSample {
        height,
        edge: if inside { 1.0 - height } else { 1.0 },
        center: (1.0 - (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2).clamp(0.0, 1.0),
        id: cell.id,
        local,
    }
}

fn brick_rotated_local(cell: BrickCell, rotation_degrees: f32) -> [f32; 2] {
    let centered = [cell.local[0] - 0.5, cell.local[1] - 0.5];
    let radians = -rotation_degrees.to_radians();
    let cosine = radians.cos();
    let sine = radians.sin();
    [
        centered[0] * cosine - centered[1] * sine + 0.5,
        centered[0] * sine + centered[1] * cosine + 0.5,
    ]
}

fn voronoi_pattern(
    uv: [f32; 2],
    cells: [u32; 2],
    jitter: f32,
    falloff: f32,
    seed: u64,
    bevel: f32,
    perturb: f32,
    perturb_amount: f32,
) -> PatternSample {
    let cells_x = cells[0].max(1) as i32;
    let cells_y = cells[1].max(1) as i32;
    let x = uv[0].rem_euclid(1.0) * cells_x as f32;
    let y = uv[1].rem_euclid(1.0) * cells_y as f32;
    let cell_x = x.floor() as i32;
    let cell_y = y.floor() as i32;
    let frac_x = x.fract();
    let frac_y = y.fract();
    let mut min_distance = f32::MAX;
    let mut second_distance = f32::MAX;
    let mut nearest = (cell_x, cell_y);
    let mut nearest_delta = [0.0, 0.0];
    for oy in -1..=1 {
        for ox in -1..=1 {
            let candidate_x = cell_x + ox;
            let candidate_y = cell_y + oy;
            let wrapped_x = candidate_x.rem_euclid(cells_x);
            let wrapped_y = candidate_y.rem_euclid(cells_y);
            let candidate_id = hash_cell(wrapped_x, wrapped_y, seed);
            let point_x = 0.5 + (random01(candidate_id) - 0.5) * jitter;
            let point_y = 0.5 + (random01(mix64(candidate_id ^ 1)) - 0.5) * jitter;
            let dx = ox as f32 + point_x - frac_x;
            let dy = oy as f32 + point_y - frac_y;
            let distance = dx.hypot(dy);
            if distance < min_distance {
                second_distance = min_distance;
                min_distance = distance;
                nearest = (wrapped_x, wrapped_y);
                nearest_delta = [-dx, -dy];
            } else if distance < second_distance {
                second_distance = distance;
            }
        }
    }
    let boundary_depth = (second_distance - min_distance) * 0.5;
    let perturb_offset = (perturb - 0.5) * 2.0 * perturb_amount.clamp(0.0, 0.5);
    let profile_depth = boundary_depth - perturb_offset;
    let bevel_width = second_distance * 0.5 * bevel.clamp(0.0, 1.0);
    let height = if profile_depth < 0.0 {
        0.0
    } else if bevel_width <= f32::EPSILON {
        1.0
    } else {
        (profile_depth / bevel_width).clamp(0.0, 1.0).powf(falloff)
    };
    PatternSample {
        height,
        edge: 1.0 - height,
        center: (1.0 - min_distance / std::f32::consts::SQRT_2).clamp(0.0, 1.0),
        id: hash_cell(nearest.0, nearest.1, seed),
        local: [
            (nearest_delta[0] + 0.5).rem_euclid(1.0),
            (nearest_delta[1] + 0.5).rem_euclid(1.0),
        ],
    }
}

fn disc_pattern(
    uv: [f32; 2],
    cells: [u32; 2],
    jitter: f32,
    radius: f32,
    falloff: f32,
    seed: u64,
    bevel: f32,
    perturb: f32,
    perturb_amount: f32,
) -> PatternSample {
    let cells_x = cells[0].max(1) as i32;
    let cells_y = cells[1].max(1) as i32;
    let x = uv[0].rem_euclid(1.0) * cells_x as f32;
    let y = uv[1].rem_euclid(1.0) * cells_y as f32;
    let cell_x = x.floor() as i32;
    let cell_y = y.floor() as i32;
    let frac_x = x.fract();
    let frac_y = y.fract();
    let mut min_distance = f32::MAX;
    let mut nearest = (cell_x, cell_y);
    let mut nearest_delta = [0.0, 0.0];
    for oy in -1..=1 {
        for ox in -1..=1 {
            let candidate_x = cell_x + ox;
            let candidate_y = cell_y + oy;
            let wrapped_x = candidate_x.rem_euclid(cells_x);
            let wrapped_y = candidate_y.rem_euclid(cells_y);
            let candidate_id = hash_cell(wrapped_x, wrapped_y, seed);
            let point_x = 0.5 + (random01(candidate_id) - 0.5) * jitter;
            let point_y = 0.5 + (random01(mix64(candidate_id ^ 2)) - 0.5) * jitter;
            let dx = ox as f32 + point_x - frac_x;
            let dy = oy as f32 + point_y - frac_y;
            let distance = dx.hypot(dy);
            if distance < min_distance {
                min_distance = distance;
                nearest = (wrapped_x, wrapped_y);
                nearest_delta = [-dx, -dy];
            }
        }
    }
    let radius = (radius * 0.5).max(0.000_1);
    let perturb_offset = (perturb - 0.5) * 2.0 * perturb_amount.clamp(0.0, 0.5);
    let signed_distance = min_distance - radius + perturb_offset;
    let bevel_width = radius * bevel.clamp(0.0, 1.0);
    let height = if signed_distance > 0.0 {
        0.0
    } else if bevel_width <= f32::EPSILON {
        1.0
    } else {
        (-signed_distance / bevel_width)
            .clamp(0.0, 1.0)
            .powf(falloff)
    };
    PatternSample {
        height,
        edge: 1.0 - height,
        center: height,
        id: hash_cell(nearest.0, nearest.1, seed),
        local: [
            (nearest_delta[0] + 0.5).rem_euclid(1.0),
            (nearest_delta[1] + 0.5).rem_euclid(1.0),
        ],
    }
}

fn shape_height(mut value: f32, contrast: f32, bias: f32, plateau: f32, rim: f32) -> f32 {
    value = ((value - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
    if bias < 0.0 {
        value = value.powf(1.0 + -bias * 3.0);
    } else if bias > 0.0 {
        value = 1.0 - (1.0 - value).powf(1.0 + bias * 3.0);
    }
    if rim > 0.0 {
        let shoulder = (4.0 * value * (1.0 - value)).clamp(0.0, 1.0);
        value = (value - shoulder * rim * 0.18).clamp(0.0, 1.0);
    }
    if plateau > 0.0 {
        let top = (1.0 - plateau * 0.18).clamp(0.35, 0.999);
        if value > top {
            let factor = ((value - top) / (1.0 - top).max(0.000_1)).clamp(0.0, 1.0);
            let curve = smoothstep(factor);
            let flatten = (0.2 / (1.0 + plateau * 1.2)).clamp(0.03, 0.2);
            value = top + curve * (1.0 - top) * flatten;
        }
    }
    value
}

fn fractal_noise(
    uv: [f32; 2],
    kind: NoiseKind,
    fractal: FractalKind,
    scale: [f32; 2],
    octaves: u32,
    persistence: f32,
    seed: u64,
) -> f32 {
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut weight = 0.0;
    for octave in 0..octaves.max(1) {
        let frequency = 1_u32.checked_shl(octave.min(16)).unwrap_or(u32::MAX);
        let lattice = [
            ((scale[0] * frequency as f32).round() as u32).max(1),
            ((scale[1] * frequency as f32).round() as u32).max(1),
        ];
        let value = match kind {
            NoiseKind::Value => periodic_value_noise(uv, lattice, mix64(seed ^ octave as u64)),
            NoiseKind::Gradient => {
                periodic_gradient_noise(uv, lattice, mix64(seed ^ octave as u64))
            }
        };
        let signed = value * 2.0 - 1.0;
        let value = match fractal {
            FractalKind::Fbm => value,
            FractalKind::Ridged => 1.0 - signed.abs(),
            FractalKind::Billow => signed.abs(),
            FractalKind::Turbulence => signed,
        };
        total += value * amplitude;
        weight += amplitude;
        amplitude *= persistence;
    }
    let value = total / weight.max(0.000_1);
    match fractal {
        FractalKind::Turbulence => value.abs().clamp(0.0, 1.0),
        _ => value.clamp(0.0, 1.0),
    }
}

fn periodic_gradient_noise(uv: [f32; 2], scale: [u32; 2], seed: u64) -> f32 {
    let scale_x = scale[0].max(1) as i32;
    let scale_y = scale[1].max(1) as i32;
    let x = uv[0].rem_euclid(1.0) * scale_x as f32;
    let y = uv[1].rem_euclid(1.0) * scale_y as f32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x.fract();
    let fy = y.fract();
    let tx = smoothstep(fx);
    let ty = smoothstep(fy);
    let dot = |sample_x: i32, sample_y: i32, dx: f32, dy: f32| {
        let hash = hash_cell(
            sample_x.rem_euclid(scale_x),
            sample_y.rem_euclid(scale_y),
            seed,
        );
        let angle = random01(hash) * std::f32::consts::TAU;
        angle.cos() * dx + angle.sin() * dy
    };
    let top = lerp(dot(x0, y0, fx, fy), dot(x0 + 1, y0, fx - 1.0, fy), tx);
    let bottom = lerp(
        dot(x0, y0 + 1, fx, fy - 1.0),
        dot(x0 + 1, y0 + 1, fx - 1.0, fy - 1.0),
        tx,
    );
    (lerp(top, bottom, ty) * 0.707_106_77 + 0.5).clamp(0.0, 1.0)
}

fn copy_tile_bytes(
    source: &[u8],
    source_width: u32,
    tile_width: u32,
    tile_height: u32,
    grid_width: u32,
    grid_height: u32,
    tile_x: u32,
    tile_y: u32,
    channels: u32,
) -> Option<Vec<u8>> {
    if tile_x >= grid_width || tile_y >= grid_height {
        return None;
    }
    let mut tile = vec![0_u8; (tile_width * tile_height * channels) as usize];
    let origin_x = tile_x * tile_width;
    let origin_y = tile_y * tile_height;
    for y in 0..tile_height {
        let source_start = (((origin_y + y) * source_width + origin_x) * channels) as usize;
        let row_len = (tile_width * channels) as usize;
        let target_start = (y * tile_width * channels) as usize;
        tile[target_start..target_start + row_len]
            .copy_from_slice(&source[source_start..source_start + row_len]);
    }
    Some(tile)
}

fn periodic_value_noise(uv: [f32; 2], scale: [u32; 2], seed: u64) -> f32 {
    let scale_x = scale[0].max(1) as i32;
    let scale_y = scale[1].max(1) as i32;
    let x = uv[0].rem_euclid(1.0) * scale_x as f32;
    let y = uv[1].rem_euclid(1.0) * scale_y as f32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = smoothstep(x.fract());
    let ty = smoothstep(y.fract());
    let sample = |sample_x: i32, sample_y: i32| {
        random01(hash_cell(
            sample_x.rem_euclid(scale_x),
            sample_y.rem_euclid(scale_y),
            seed,
        ))
    };
    let top = lerp(sample(x0, y0), sample(x0 + 1, y0), tx);
    let bottom = lerp(sample(x0, y0 + 1), sample(x0 + 1, y0 + 1), tx);
    lerp(top, bottom, ty)
}

fn wrap_coordinate(value: f32, mode: WrapMode) -> f32 {
    match mode {
        WrapMode::Clamp => value.clamp(0.0, 1.0),
        WrapMode::Repeat => value.rem_euclid(1.0),
        WrapMode::Mirror => {
            let value = value.rem_euclid(2.0);
            if value <= 1.0 { value } else { 2.0 - value }
        }
    }
}

fn grayscale_values(values: &[f32]) -> (Vec<u8>, Vec<u16>) {
    let mut rgba = Vec::with_capacity(values.len() * 4);
    for value in values {
        let value = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        rgba.extend_from_slice(&[value, value, value, 255]);
    }
    (rgba, vec![u16::MAX; values.len()])
}

fn bayer4(x: u32, y: u32) -> f32 {
    const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (BAYER[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0
}

fn hash_cell(x: i32, y: i32, seed: u64) -> u64 {
    mix64(seed ^ (x as i64 as u64).rotate_left(17) ^ (y as i64 as u64).rotate_left(43))
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x1000_0000_01b3)
    })
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn lerp(a: f32, b: f32, value: f32) -> f32 {
    a + (b - a) * value
}

fn srgba8_to_linear(rgba: [u8; 4]) -> [f32; 4] {
    let channel = |value: u8| {
        let value = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    [
        channel(rgba[0]),
        channel(rgba[1]),
        channel(rgba[2]),
        rgba[3] as f32 / 255.0,
    ]
}

fn linear_to_srgba8(rgba: [f32; 4]) -> [u8; 4] {
    let channel = |value: f32| {
        let value = value.clamp(0.0, 1.0);
        let value = if value <= 0.003_130_8 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (value * 255.0).round() as u8
    };
    [
        channel(rgba[0]),
        channel(rgba[1]),
        channel(rgba[2]),
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn random01(seed: u64) -> f32 {
    ((seed >> 40) as u32) as f32 / ((1_u32 << 24) - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_material_document, parse_recipe};
    use theframework::prelude::TheColor;

    fn test_palette() -> ThePalette {
        ThePalette::new(vec![
            Some(TheColor::from_u8(20, 25, 28, 255)),
            Some(TheColor::from_u8(61, 51, 45, 255)),
            Some(TheColor::from_u8(110, 75, 52, 255)),
            Some(TheColor::from_u8(170, 125, 80, 255)),
            Some(TheColor::from_u8(220, 195, 150, 255)),
            Some(TheColor::from_u8(225, 230, 225, 255)),
        ])
    }

    fn recipe() -> Recipe {
        parse_recipe(include_str!("../examples/bricks.recipe")).unwrap()
    }

    fn colored_recipe() -> Recipe {
        parse_recipe(include_str!("../examples/stones.recipe")).unwrap()
    }

    #[test]
    fn height_first_render_is_deterministic_and_palette_pure() {
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let first = renderer
            .render(&colored_recipe(), &RenderOptions::default())
            .unwrap();
        let second = renderer
            .render(&colored_recipe(), &RenderOptions::default())
            .unwrap();
        assert_eq!(first.frames[0].height, second.frames[0].height);
        assert_eq!(first.frames[0].rgba, second.frames[0].rgba);
        assert!(
            first.frames[0]
                .palette_indices
                .iter()
                .all(|index| *index < 6)
        );
    }

    #[test]
    fn palette_swap_changes_only_colorization() {
        let warm = RecipeRenderer::new(&test_palette())
            .unwrap()
            .render(&colored_recipe(), &RenderOptions::default())
            .unwrap();
        let cool_palette = ThePalette::new(vec![
            Some(TheColor::from_u8(4, 8, 18, 255)),
            Some(TheColor::from_u8(20, 55, 90, 255)),
            Some(TheColor::from_u8(50, 110, 160, 255)),
            Some(TheColor::from_u8(150, 210, 235, 255)),
        ]);
        let cool = RecipeRenderer::new(&cool_palette)
            .unwrap()
            .render(&colored_recipe(), &RenderOptions::default())
            .unwrap();
        assert_eq!(warm.frames[0].height, cool.frames[0].height);
        assert_eq!(warm.frames[0].coverage, cool.frames[0].coverage);
        assert_ne!(warm.frames[0].rgba, cool.frames[0].rgba);
    }

    #[test]
    fn missing_colorize_renders_the_heightmap_as_grayscale() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(4, 1)

    Height Surface
        source = U
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::new(&test_palette())
            .unwrap()
            .render(&recipe, &RenderOptions::default())
            .unwrap();
        let frame = &rendered.frames[0];

        assert_eq!(frame.height, vec![32, 96, 159, 223]);
        assert_eq!(
            frame.rgba,
            vec![
                32, 32, 32, 255, 96, 96, 96, 255, 159, 159, 159, 255, 223, 223, 223, 255,
            ]
        );
        assert!(frame.palette_indices.iter().all(|index| *index == u16::MAX));
    }

    #[test]
    fn local_disc_kerbs_change_the_brick_heightmap() {
        let with_kerbs = recipe();
        let mut without_kerbs = with_kerbs.clone();
        let wall = without_kerbs
            .patterns
            .iter_mut()
            .find(|pattern| pattern.name == "Wall")
            .unwrap();
        wall.perturb = None;
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let detailed = renderer
            .render(&with_kerbs, &RenderOptions::default())
            .unwrap();
        let plain = renderer
            .render(&without_kerbs, &RenderOptions::default())
            .unwrap();

        assert_ne!(detailed.frames[0].height, plain.frames[0].height);
    }

    #[test]
    fn wave_animates_the_height_source() {
        let animated = parse_recipe(
            r#"
Tile
    Animation
        frames = 4
        fps = 8

    Height Surface
        source = Wave(0.2, 0.8, 1)

    Colorize
        source = Surface

    Output
        height = Surface
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::new(&test_palette())
            .unwrap()
            .render(&animated, &RenderOptions::default())
            .unwrap();
        assert_eq!(rendered.frames.len(), 4);
        assert_eq!(rendered.frames[2].time, 0.5);
        assert_ne!(rendered.frames[0].height, rendered.frames[1].height);
    }

    #[test]
    fn nested_scalar_functions_drive_height() {
        let expression_recipe = parse_recipe(
            r#"
Tile
    size = I2(4, 4)

    Height Surface
        source = Clamp(Pow((1 + 2 * 3) / 7, 2), 0, 1)

    Colorize
        source = Surface

    Output
        height = Surface
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::new(&test_palette())
            .unwrap()
            .render(&expression_recipe, &RenderOptions::default())
            .unwrap();
        assert!(rendered.frames[0].height.iter().all(|value| *value == 255));
    }

    #[test]
    fn material_render_uses_its_own_fields_and_outputs_direct_channels() {
        let tile = parse_recipe(
            r#"
Tile
    size = I2(4, 4)

    Height Surface
        source = U

    Colorize
        source = Surface

    Output
        height = Surface
"#,
        )
        .unwrap();
        let material = parse_material_document(
            r#"
Material test
    Color Dark
        base = #201810

    Color Light
        base = #a08060

    Color Tone
        source = Mix(Dark, Light, U)

    Surface
        color = Tone
        roughness = U
        metallic = 0.25
        opacity = 0.75
        emission = V
        normal = U
        normal_strength = 0.6
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let tile = renderer.render(&tile, &RenderOptions::default()).unwrap();
        let material = renderer
            .render_material(&material, &tile, &RenderOptions::default())
            .unwrap();

        assert_eq!(material.frames[0].material.len(), 16);
        assert_eq!(material.normal_strength, 0.6);
        assert!(
            material.frames[0]
                .material
                .iter()
                .all(|value| value[1] == 0.25 && value[2] == 0.75)
        );
        assert_ne!(
            material.frames[0].material[0][0],
            material.frames[0].material[3][0]
        );
        assert_ne!(
            &material.frames[0].rgba[0..4],
            &material.frames[0].rgba[12..16]
        );
    }

    #[test]
    fn tiled_material_wrapper_matches_consumer_neutral_surface_render() {
        let tile = parse_recipe(
            r#"
Tile
    size = I2(4, 3)
    coverage = I2(2, 1)

    Height Surface
        source = U

    Animation
        frames = 3
        fps = 9
        looping = true

    Output
        height = Surface
"#,
        )
        .unwrap();
        let material = parse_material_document(
            r#"
Material test
    Noise Detail
        scale = F2(3, 2)

    Color Dark
        exact = #201810

    Color Light
        exact = #a08060

    Color Tone
        source = Mix(Dark, Light, Detail)

    Surface
        color = Tone
        roughness = U
        metallic = V
        opacity = 0.75
        emission = Detail
        normal = Detail
        normal_strength = 0.6
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let rendered_tile = renderer.render(&tile, &RenderOptions::default()).unwrap();

        let tiled = renderer
            .render_material(&material, &rendered_tile, &RenderOptions::default())
            .unwrap();
        let surface = RenderSurface::from(&rendered_tile);
        let neutral = renderer
            .render_material_on_surface(&material, &surface, &RenderOptions::default())
            .unwrap();

        assert_eq!(neutral.name, tiled.name);
        assert_eq!(neutral.width, tiled.width);
        assert_eq!(neutral.height, tiled.height);
        assert_eq!(neutral.fps, tiled.fps);
        assert_eq!(neutral.looping, tiled.looping);
        assert_eq!(neutral.normal_strength, tiled.normal_strength);
        assert_eq!(neutral.frames, tiled.frames);
        assert_eq!(tiled.tile_width, rendered_tile.tile_width);
        assert_eq!(tiled.tile_height, rendered_tile.tile_height);
        assert_eq!(tiled.grid_width, rendered_tile.grid_width);
        assert_eq!(tiled.grid_height, rendered_tile.grid_height);
    }

    #[test]
    fn pattern_local_material_space_resets_uv_and_exposes_unit_id() {
        let tile = parse_recipe(
            r#"
Tile
    size = I2(8, 1)

    Pattern Planks
        Bricks
            columns = 2
            rows = 1
            gap = 0.0
            bevel = 0.0

    Output
        height = Planks.height
"#,
        )
        .unwrap();
        let material = parse_material_document(
            r#"
Material test
    Color Dark
        exact = #101010

    Color Light
        exact = #f0f0f0

    Color UnitColor
        source = Mix(Dark, Light, Random(Id, 0.1, 0.9, 23))

    Surface
        color = UnitColor
        palette = BaseOnly
        roughness = Random(Id, 0.1, 0.9, 29)
        metallic = U
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let rendered_tile = renderer.render(&tile, &RenderOptions::default()).unwrap();

        let global = renderer
            .render_material(&material, &rendered_tile, &RenderOptions::default())
            .unwrap();
        assert!(
            global.frames[0]
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel == &global.frames[0].rgba[0..4])
        );

        let local = renderer
            .render_material_in_space(
                &material,
                &rendered_tile,
                &tile,
                &Domain::PatternLocal("Planks".to_string()),
                [0.5, 2.0],
                &RenderOptions::default(),
            )
            .unwrap();
        let frame = &local.frames[0];

        assert_ne!(&frame.rgba[0..4], &frame.rgba[16..20]);
        assert_ne!(frame.material[0][0], frame.material[4][0]);
        assert!((frame.material[0][1] - frame.material[4][1]).abs() < 0.000_001);
        assert!(frame.material[0][1] < 0.1);
    }

    #[test]
    fn material_debug_height_overrides_only_the_preview_color() {
        let tile = parse_recipe(
            r#"
Tile
    size = I2(4, 1)

    Output
        height = 0.5
"#,
        )
        .unwrap();
        let material = parse_material_document(
            r#"
Material test
    Noise Debug
        scale = F2(2, 2)

    Surface
        color = #804020
        roughness = 0.7

    Output
        value = Debug
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let renderer = RecipeRenderer::new(&test_palette()).unwrap();
        let tile = renderer.render(&tile, &RenderOptions::default()).unwrap();
        let runtime = renderer
            .render_material(&material, &tile, &RenderOptions::default())
            .unwrap();
        let preview = renderer
            .render_material_preview(&material, &tile, &RenderOptions::default())
            .unwrap();

        assert!(
            runtime.frames[0]
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel[0] != pixel[1])
        );
        assert!(
            preview.frames[0]
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2])
        );
        assert_eq!(runtime.frames[0].material, preview.frames[0].material);
    }

    #[test]
    fn stable_id_random_is_constant_within_a_pattern_unit() {
        let evaluator = Evaluator {
            recipe: &recipe(),
            seed: 42,
        };
        let source = ScalarSource::RandomId {
            id: IdSource::Pattern("Wall".to_string()),
            min: 0.0,
            max: 1.0,
            seed: 9,
        };
        let a = evaluator
            .scalar(
                &source,
                EvalContext {
                    uv: [0.150, 0.150],
                    time: 0.0,
                    current_id: None,
                    input_height: 0.0,
                    pattern_bindings: [None; 8],
                },
                &mut Vec::new(),
            )
            .unwrap();
        let b = evaluator
            .scalar(
                &source,
                EvalContext {
                    uv: [0.155, 0.155],
                    time: 0.0,
                    current_id: None,
                    input_height: 0.0,
                    pattern_bindings: [None; 8],
                },
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn current_id_drives_per_unit_sdf_parameters() {
        let recipe = recipe();
        let evaluator = Evaluator {
            recipe: &recipe,
            seed: 42,
        };
        let source = ScalarSource::RandomId {
            id: IdSource::Current,
            min: 0.05,
            max: 0.15,
            seed: 13,
        };
        let context = EvalContext {
            uv: [0.25, 0.25],
            time: 0.0,
            current_id: Some(41),
            input_height: 0.0,
            pattern_bindings: [None; 8],
        };
        let first = evaluator.scalar(&source, context, &mut Vec::new()).unwrap();
        let second = evaluator
            .scalar(
                &source,
                EvalContext {
                    current_id: Some(42),
                    ..context
                },
                &mut Vec::new(),
            )
            .unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn procedural_generators_are_periodic() {
        let seed = 982_451_653;
        let noise = periodic_value_noise([0.173, 0.619], [5, 7], seed);
        assert!((noise - periodic_value_noise([1.173, -0.381], [5, 7], seed)).abs() < 1e-5);
        let first = voronoi_pattern([0.173, 0.619], [6, 5], 0.8, 1.0, seed, 0.08, 0.5, 0.0);
        let repeated = voronoi_pattern([1.173, -0.381], [6, 5], 0.8, 1.0, seed, 0.08, 0.5, 0.0);
        assert!((first.height - repeated.height).abs() < 1e-5);
        assert_eq!(first.id, repeated.id);
    }

    #[test]
    fn brick_sdf_perturbation_does_not_move_the_unit() {
        let cell = BrickCell {
            local: [0.08, 0.5],
            id: 42,
        };
        let smooth = brick_pattern(cell, 0.08, 0.08, 0.1, 0.0, [0.0, 0.0], 0.5, 0.08, 1.0);
        let eroded = brick_pattern(cell, 0.08, 0.08, 0.1, 0.0, [0.0, 0.0], 1.0, 0.08, 1.0);

        assert_eq!(smooth.id, eroded.id);
        assert_eq!(smooth.local, eroded.local);
        assert_ne!(smooth.height, eroded.height);
    }

    #[test]
    fn pattern_perturb_changes_voronoi_and_discs_without_reassigning_units() {
        let seed = 42;
        let mut voronoi_changed = false;
        let mut discs_changed = false;
        for y in 0..32 {
            for x in 0..32 {
                let uv = [(x as f32 + 0.5) / 32.0, (y as f32 + 0.5) / 32.0];
                let base = voronoi_pattern(uv, [6, 5], 0.8, 1.0, seed, 0.08, 0.5, 0.0);
                let rough = voronoi_pattern(uv, [6, 5], 0.8, 1.0, seed, 0.08, 1.0, 0.05);
                assert_eq!(base.id, rough.id);
                assert_eq!(base.local, rough.local);
                voronoi_changed |= (base.height - rough.height).abs() > 0.001;

                let base = disc_pattern(uv, [6, 5], 0.8, 1.2, 1.0, seed, 0.08, 0.5, 0.0);
                let rough = disc_pattern(uv, [6, 5], 0.8, 1.2, 1.0, seed, 0.08, 1.0, 0.05);
                assert_eq!(base.id, rough.id);
                assert_eq!(base.local, rough.local);
                discs_changed |= (base.height - rough.height).abs() > 0.001;
            }
        }
        assert!(voronoi_changed);
        assert!(discs_changed);
    }

    #[test]
    fn common_bevel_changes_voronoi_and_disc_profiles() {
        let seed = 42;
        let mut voronoi_changed = false;
        let mut discs_changed = false;
        for y in 0..32 {
            for x in 0..32 {
                let uv = [(x as f32 + 0.5) / 32.0, (y as f32 + 0.5) / 32.0];
                let narrow = voronoi_pattern(uv, [4, 4], 0.8, 1.0, seed, 0.05, 0.5, 0.0);
                let wide = voronoi_pattern(uv, [4, 4], 0.8, 1.0, seed, 0.8, 0.5, 0.0);
                voronoi_changed |= (narrow.height - wide.height).abs() > 0.001;

                let narrow = disc_pattern(uv, [4, 4], 0.8, 1.2, 1.0, seed, 0.05, 0.5, 0.0);
                let wide = disc_pattern(uv, [4, 4], 0.8, 1.2, 1.0, seed, 0.8, 0.5, 0.0);
                discs_changed |= (narrow.height - wide.height).abs() > 0.001;
            }
        }

        assert!(voronoi_changed);
        assert!(discs_changed);
    }

    #[test]
    fn common_pattern_modifiers_render_for_every_generator() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(16, 16)

    Noise ShapeNoise
        scale = F2(3.0, 3.0)
        octaves = 2

    Pattern Masonry
        Bricks
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Pattern Cells
        Voronoi
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Pattern Dots
        Discs
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Output
        height = (Masonry.height + Cells.height + Dots.height) / 3.0
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::grayscale()
            .render(&recipe, &RenderOptions::default())
            .unwrap();

        assert_eq!(rendered.frames[0].height.len(), 16 * 16);
        assert!(
            rendered.frames[0]
                .height
                .windows(2)
                .any(|values| values[0] != values[1])
        );
    }

    #[test]
    fn output_space_previews_the_exact_pattern_local_keyed_noise() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(32, 32)

    Noise LocalWarp
        key = Id
        scale = F2(2.0, 2.0)
        octaves = 2
        seed = 7

    Pattern Wall
        Bricks
            columns = 4
            rows = 4
            perturb = LocalWarp
            perturb_amount = 0.05

    Output
        height = LocalWarp
        space = Wall.local
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::grayscale()
            .render(&recipe, &RenderOptions::default())
            .unwrap();
        let height = &rendered.frames[0].height;

        let min = height.iter().copied().min().unwrap();
        let max = height.iter().copied().max().unwrap();
        assert!(max - min > 40);
    }

    #[test]
    fn child_pattern_can_use_the_active_parent_local_space_for_perturbation() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(32, 32)

    Pattern Wall
        Bricks
            columns = 4
            rows = 4
            perturb = Kerbs.height
            perturb_amount = 0.08

    Pattern Kerbs
        Discs
            space = Wall.local
            cells = I2(3, 3)
            radius = 0.25 + Random(Wall.id, 0.0, 0.15, 5)

    Height Surface
        source = Wall.height

        Subtract
            source = Kerbs.height
            amount = 0.12

    Output
        height = Surface
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::grayscale()
            .render(&recipe, &RenderOptions::default())
            .unwrap();
        let height = &rendered.frames[0].height;

        assert!(height.windows(2).any(|values| values[0] != values[1]));
    }

    #[test]
    fn active_pattern_binding_does_not_hide_a_real_height_cycle() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(4, 4)

    Pattern Wall
        Bricks
            perturb = Wall.height
            perturb_amount = 0.08

    Output
        height = Wall.height
"#,
        )
        .unwrap();
        let error = RecipeRenderer::grayscale()
            .render(&recipe, &RenderOptions::default())
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("cyclic field dependency involving 'pattern:wall'")
        );
    }

    #[test]
    fn global_output_previews_keyed_noise_with_the_zero_key() {
        let recipe = parse_recipe(
            r#"
Tile
    size = I2(32, 32)

    Noise LocalWarp
        key = Id
        scale = F2(2.0, 2.0)
        octaves = 2
        seed = 7

    Output
        height = LocalWarp
"#,
        )
        .unwrap();
        let rendered = RecipeRenderer::grayscale()
            .render(&recipe, &RenderOptions::default())
            .unwrap();
        let height = &rendered.frames[0].height;

        let min = height.iter().copied().min().unwrap();
        let max = height.iter().copied().max().unwrap();
        assert!(max - min > 40);
    }

    #[test]
    fn zero_gap_does_not_force_a_minimum_brick_inset() {
        let cell = BrickCell {
            local: [0.0005, 0.5],
            id: 42,
        };
        let touching = brick_pattern(cell, 0.0, 0.0, 0.0, 0.0, [0.0, 0.0], 0.5, 0.0, 1.0);
        let separated = brick_pattern(cell, 0.01, 0.0, 0.0, 0.0, [0.0, 0.0], 0.5, 0.0, 1.0);

        assert!(touching.height > 0.0);
        assert_eq!(separated.height, 0.0);
    }

    #[test]
    fn brick_bevel_is_bounded_to_the_edge() {
        let cell = BrickCell {
            local: [0.1, 0.5],
            id: 42,
        };
        let sample = brick_pattern(cell, 0.0, 0.08, 0.0, 0.0, [0.0, 0.0], 0.5, 0.0, 1.0);

        assert_eq!(sample.height, 1.0);
    }

    #[test]
    fn slices_coordinated_output_into_tiles() {
        let rendered = RecipeRenderer::new(&test_palette())
            .unwrap()
            .render(&recipe(), &RenderOptions::default())
            .unwrap();
        assert_eq!(rendered.tile_rgba(0, 3, 3).unwrap().len(), 64 * 64 * 4);
        let height = rendered.tile_height_values(0, 3, 3).unwrap();
        assert!(height.iter().any(|value| *value > 180));
        assert!(height.iter().any(|value| *value < 30));
        assert!(rendered.tile_rgba(0, 4, 0).is_none());
    }
}
