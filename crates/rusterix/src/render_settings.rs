use crate::value::{Value, ValueContainer};
use crate::value_toml::ValueTomlLoader;
use rustc_hash::FxHashMap;
use scenevm::{Atom, RenderMode as SceneVmRenderMode, SceneVM};
use vek::Vec4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    Compute,
    Raster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderQualityPreset {
    Low,
    Medium,
    High,
    Ultra,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeMode {
    OrderedDither,
    Uniform,
}

impl FadeMode {
    fn as_code(self) -> u32 {
        match self {
            FadeMode::OrderedDither => 0,
            FadeMode::Uniform => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingModel {
    Lambert,
    CookTorrance,
    Pbr,
}

impl LightingModel {
    fn as_code(self) -> u32 {
        match self {
            LightingModel::Lambert => 0,
            LightingModel::CookTorrance => 1,
            LightingModel::Pbr => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostToneMapper {
    None,
    Reinhard,
    Aces,
}

impl PostToneMapper {
    fn as_code(self) -> u32 {
        match self {
            PostToneMapper::None => 0,
            PostToneMapper::Reinhard => 1,
            PostToneMapper::Aces => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostTarget {
    Both,
    D2,
    D3,
}

#[derive(Debug, Clone)]
pub struct PostEffectSettings {
    pub name: String,
    pub enabled: bool,
    pub target: PostTarget,
    pub intensity: f32,
    pub distance_start: Option<f32>,
    pub distance_end: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct PostStackSettings {
    pub enabled: bool,
    pub effects: Vec<PostEffectSettings>,
}

/// PBR Render Settings for scenes
/// Corresponds to the uniform parameters (gp0-gp9) in the SceneVM PBR shader
#[derive(Debug, Clone)]
pub struct RenderSettings {
    /// Renderer backend for 2D path.
    pub backend_2d: RendererBackend,
    /// Renderer backend for 3D path.
    pub backend_3d: RendererBackend,
    /// User-facing quality preset.
    pub quality: RenderQualityPreset,
    /// Shared modular post stack config (2D/3D).
    pub post: PostStackSettings,

    /// Sky color (RGB) - set from TOML or dynamically by apply_hour()
    pub sky_color: [f32; 3],

    /// Sun color (RGB) - set from TOML or dynamically by apply_hour()
    pub sun_color: [f32; 3],

    /// Sun intensity (brightness multiplier)
    pub sun_intensity: f32,

    /// Sun direction (normalized vector) - set from TOML or dynamically by apply_hour()
    pub sun_direction: [f32; 3],

    /// Sun enabled
    pub sun_enabled: bool,

    /// Ambient color (RGB)
    pub ambient_color: [f32; 3],

    /// Ambient strength (0.0 to 1.0)
    pub ambient_strength: f32,

    /// Fog color (RGB)
    pub fog_color: [f32; 3],

    /// Fog density (0.0 = no fog, higher = denser)
    pub fog_density: f32,

    /// AO samples (number of rays)
    pub ao_samples: f32,

    /// AO radius
    pub ao_radius: f32,

    /// Bump strength (0.0-1.0)
    pub bump_strength: f32,

    /// Raster 3D MSAA sample count (0=off, 4=on).
    pub msaa_samples: u32,

    /// Max transparency bounces
    pub max_transparency_bounces: f32,

    /// Max shadow distance
    pub max_shadow_distance: f32,

    /// Max sky distance
    pub max_sky_distance: f32,

    /// Max shadow steps (for transparent shadows)
    pub max_shadow_steps: f32,

    /// Reflection samples (0 = disabled, higher = better quality)
    pub reflection_samples: f32,

    /// First-person texture transition start distance (world units).
    pub firstp_blur_near: f32,

    /// First-person texture transition end distance (world units).
    pub firstp_blur_far: f32,

    /// Raster 3D: enable/disable shadow map shading.
    pub raster_shadow_enabled: bool,
    /// Raster 3D: shadow contribution strength (0..1).
    pub raster_shadow_strength: f32,
    /// Raster 3D: shadow-map resolution in pixels.
    pub raster_shadow_resolution: f32,
    /// Raster 3D: depth bias to reduce acne/peter-panning.
    pub raster_shadow_bias: f32,
    /// Alpha fade mode used by raster path for geometry and billboards.
    pub fade_mode: FadeMode,
    /// Lighting model used by raster path.
    pub lighting_model: LightingModel,
    /// Avatar readability boost toggle for Raster 3D.
    pub avatar_highlight_enabled: bool,
    /// Avatar readability lift multiplier.
    pub avatar_highlight_lift: f32,
    /// Avatar readability ambient fill contribution.
    pub avatar_highlight_fill: f32,
    /// Avatar readability rim-light contribution.
    pub avatar_highlight_rim: f32,
    /// Enables generated marker ramp shading for avatars.
    pub avatar_shading_enabled: bool,
    /// Enables generated marker ramp shading for skin markers.
    pub avatar_skin_shading_enabled: bool,
    /// Post-processing enable toggle for final 3D output.
    pub post_enabled: bool,
    /// Tone mapper used in post step.
    pub post_tone_mapper: PostToneMapper,
    /// Exposure multiplier applied before tone mapping.
    pub post_exposure: f32,
    /// Output gamma.
    pub post_gamma: f32,
    /// Post saturation multiplier (1 = unchanged, 0 = grayscale).
    pub post_saturation: f32,
    /// Post luminance/brightness multiplier.
    pub post_luminance: f32,

    /// Target frame time in milliseconds for interpolation (default 30 FPS)
    pub frame_time_ms: f32,

    transitions: FxHashMap<SettingKey, Transition>,

    /// Daylight simulation settings
    pub simulation: DaylightSimulation,
}

/// Daylight simulation settings for time-of-day rendering
#[derive(Debug, Clone)]
pub struct DaylightSimulation {
    /// Enable procedural daylight simulation
    pub enabled: bool,

    /// Sky color at night
    pub night_sky_color: [f32; 3],

    /// Sky color at sunrise/sunset
    pub morning_sky_color: [f32; 3],

    /// Sky color at midday
    pub midday_sky_color: [f32; 3],

    /// Sky color in the evening
    pub evening_sky_color: [f32; 3],

    /// Sun color at night (moon light)
    pub night_sun_color: [f32; 3],

    /// Sun color at sunrise/sunset
    pub morning_sun_color: [f32; 3],

    /// Sun color at midday
    pub midday_sun_color: [f32; 3],

    /// Sun color in the evening
    pub evening_sun_color: [f32; 3],

    /// Sunrise time (0.0 - 24.0, e.g., 6.5 = 6:30 AM)
    pub sunrise_time: f32,

    /// Sunset time (0.0 - 24.0, e.g., 18.5 = 6:30 PM)
    pub sunset_time: f32,

    /// Duration in hours for each color transition window.
    /// Example: 0.5 = 30 in-game minutes.
    pub color_transition_duration_hours: f32,
}

impl Default for DaylightSimulation {
    fn default() -> Self {
        Self {
            enabled: false,
            night_sky_color: [0.02, 0.02, 0.05], // Very dark blue
            morning_sky_color: [1.0, 0.6, 0.4],  // Warm orange morning
            midday_sky_color: [0.529, 0.808, 0.922], // Clear blue
            evening_sky_color: [1.0, 0.5, 0.3],  // Warm orange evening
            night_sun_color: [0.1, 0.1, 0.15],   // Very dim bluish (moon)
            morning_sun_color: [1.0, 0.8, 0.6],  // Warm morning sun
            midday_sun_color: [1.0, 1.0, 0.95],  // Bright white sun
            evening_sun_color: [1.0, 0.7, 0.5],  // Warm evening sun
            sunrise_time: 6.0,
            sunset_time: 18.0,
            color_transition_duration_hours: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SettingKey {
    SkyColor,
    SunColor,
    SunIntensity,
    SunDirection,
    SunEnabled,
    AmbientColor,
    AmbientStrength,
    FogColor,
    FogDensity,
    AoSamples,
    AoRadius,
    BumpStrength,
    MaxTransparencyBounces,
    MaxShadowDistance,
    MaxSkyDistance,
    MaxShadowSteps,
    ReflectionSamples,
    FrameTimeMs,
}

#[derive(Debug, Clone)]
enum Transition {
    Float {
        start: f32,
        target: f32,
        duration: f32,
        elapsed: f32,
    },
    Vec3 {
        start: [f32; 3],
        target: [f32; 3],
        duration: f32,
        elapsed: f32,
    },
    Bool {
        start: bool,
        target: bool,
        duration: f32,
        elapsed: f32,
    },
}

#[derive(Debug, Clone)]
enum SettingValue {
    Float(f32),
    Vec3([f32; 3]),
    Bool(bool),
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            backend_2d: RendererBackend::Compute,
            backend_3d: RendererBackend::Raster,
            quality: RenderQualityPreset::Custom,
            post: PostStackSettings::default(),
            sky_color: [0.529, 0.808, 0.922], // #87CEEB
            sun_color: [1.0, 0.980, 0.804],   // #FFFACD
            sun_intensity: 1.0,
            sun_direction: [-0.5, -1.0, -0.3],
            sun_enabled: true,
            ambient_color: [0.8, 0.8, 0.8],
            ambient_strength: 0.3,
            fog_color: [0.502, 0.502, 0.502], // #808080
            fog_density: 0.0,
            ao_samples: 8.0,
            ao_radius: 0.5,
            bump_strength: 1.0,
            msaa_samples: 4,
            max_transparency_bounces: 8.0,
            max_shadow_distance: 10.0,
            max_sky_distance: 50.0,
            max_shadow_steps: 2.0,
            reflection_samples: 0.0,
            firstp_blur_near: 3.0,
            firstp_blur_far: 8.0,
            raster_shadow_enabled: true,
            raster_shadow_strength: 0.8,
            raster_shadow_resolution: 1024.0,
            raster_shadow_bias: 0.0015,
            fade_mode: FadeMode::OrderedDither,
            lighting_model: LightingModel::CookTorrance,
            avatar_highlight_enabled: true,
            avatar_highlight_lift: 1.12,
            avatar_highlight_fill: 0.20,
            avatar_highlight_rim: 0.18,
            avatar_shading_enabled: true,
            avatar_skin_shading_enabled: false,
            post_enabled: true,
            post_tone_mapper: PostToneMapper::Reinhard,
            post_exposure: 1.0,
            post_gamma: 2.2,
            post_saturation: 1.0,
            post_luminance: 1.0,
            frame_time_ms: 1000.0 / 30.0,
            transitions: FxHashMap::default(),
            simulation: DaylightSimulation::default(),
        }
    }
}

impl RenderSettings {
    pub fn scenevm_mode_2d(&self) -> SceneVmRenderMode {
        match self.backend_2d {
            // Raster backend scaffold: keep compute path until raster VM mode lands.
            RendererBackend::Compute | RendererBackend::Raster => SceneVmRenderMode::Compute2D,
        }
    }

    pub fn scenevm_mode_3d(&self) -> SceneVmRenderMode {
        match self.backend_3d {
            RendererBackend::Compute => SceneVmRenderMode::Compute3D,
            RendererBackend::Raster => SceneVmRenderMode::Raster3D,
        }
    }

    /// Parse render settings from a TOML string's [render] and [simulation] sections
    pub fn read(&mut self, toml_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let groups = ValueTomlLoader::from_str(toml_content)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        self.read_renderer_and_post_sections(toml_content)?;

        if let Some(render) = groups.get("render") {
            self.apply_render_values(render)?;
        }

        if let Some(sim) = groups.get("simulation") {
            self.apply_simulation_values(sim)?;
        }

        Ok(())
    }

    fn read_renderer_and_post_sections(
        &mut self,
        toml_content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let doc: toml::Value = toml::from_str(toml_content)?;

        if let Some(renderer) = doc.get("renderer").and_then(toml::Value::as_table) {
            if let Some(v) = renderer.get("backend_2d").and_then(toml::Value::as_str) {
                self.backend_2d = parse_backend(v);
            }
            if let Some(v) = renderer.get("backend_3d").and_then(toml::Value::as_str) {
                self.backend_3d = parse_backend(v);
            }
            if let Some(v) = renderer.get("quality").and_then(toml::Value::as_str) {
                self.quality = parse_quality(v);
                self.apply_quality_preset(self.quality);
            }
        }

        if let Some(raster3d) = doc.get("raster_3d").and_then(toml::Value::as_table) {
            if let Some(v) = raster3d
                .get("shadow_enabled")
                .and_then(toml::Value::as_bool)
            {
                self.raster_shadow_enabled = v;
            }
            if let Some(v) = raster3d
                .get("shadow_strength")
                .and_then(toml::Value::as_float)
            {
                self.raster_shadow_strength = v as f32;
            }
            if let Some(v) = raster3d
                .get("shadow_resolution")
                .and_then(toml::Value::as_integer)
            {
                self.raster_shadow_resolution = v as f32;
            } else if let Some(v) = raster3d
                .get("shadow_resolution")
                .and_then(toml::Value::as_float)
            {
                self.raster_shadow_resolution = v as f32;
            }
            if let Some(v) = raster3d.get("shadow_bias").and_then(toml::Value::as_float) {
                self.raster_shadow_bias = v as f32;
            }
            if let Some(v) = raster3d
                .get("avatar_highlight_enabled")
                .and_then(toml::Value::as_bool)
            {
                self.avatar_highlight_enabled = v;
            }
            if let Some(v) = raster3d
                .get("avatar_highlight_lift")
                .and_then(toml::Value::as_float)
            {
                self.avatar_highlight_lift = v as f32;
            }
            if let Some(v) = raster3d
                .get("avatar_highlight_fill")
                .and_then(toml::Value::as_float)
            {
                self.avatar_highlight_fill = v as f32;
            }
            if let Some(v) = raster3d
                .get("avatar_highlight_rim")
                .and_then(toml::Value::as_float)
            {
                self.avatar_highlight_rim = v as f32;
            }
        }

        if let Some(game) = doc.get("game").and_then(toml::Value::as_table) {
            if let Some(v) = game.get("avatar_shading").and_then(toml::Value::as_bool) {
                self.avatar_shading_enabled = v;
            }
            if let Some(v) = game
                .get("avatar_skin_auto_shading")
                .and_then(toml::Value::as_bool)
            {
                self.avatar_skin_shading_enabled = v;
            }
        }

        self.post.enabled = doc
            .get("post")
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("enabled"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(self.post.enabled);
        self.post_enabled = self.post.enabled;

        if let Some(post) = doc.get("post").and_then(toml::Value::as_table) {
            if let Some(v) = post.get("enabled").and_then(toml::Value::as_bool) {
                self.post_enabled = v;
                self.post.enabled = v;
            }
            if let Some(v) = post.get("tone_mapper").and_then(toml::Value::as_str) {
                self.post_tone_mapper = parse_tone_mapper(v);
            }
            if let Some(v) = post.get("exposure").and_then(toml::Value::as_float) {
                self.post_exposure = v as f32;
            }
            if let Some(v) = post.get("gamma").and_then(toml::Value::as_float) {
                self.post_gamma = v as f32;
            }
            if let Some(v) = post.get("saturation").and_then(toml::Value::as_float) {
                self.post_saturation = v as f32;
            }
            if let Some(v) = post.get("luminance").and_then(toml::Value::as_float) {
                self.post_luminance = v as f32;
            }
        }

        self.post.effects.clear();
        if let Some(effects) = doc
            .get("post")
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("effects"))
            .and_then(toml::Value::as_array)
        {
            for effect in effects {
                let Some(tbl) = effect.as_table() else {
                    continue;
                };
                let Some(name) = tbl.get("name").and_then(toml::Value::as_str) else {
                    continue;
                };
                let enabled = tbl
                    .get("enabled")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(true);
                let target = tbl
                    .get("target")
                    .and_then(toml::Value::as_str)
                    .map(parse_post_target)
                    .unwrap_or(PostTarget::Both);
                let intensity = tbl
                    .get("intensity")
                    .and_then(toml::Value::as_float)
                    .map(|v| v as f32)
                    .unwrap_or(1.0);
                let distance_start = tbl
                    .get("distance_start")
                    .and_then(toml::Value::as_float)
                    .map(|v| v as f32);
                let distance_end = tbl
                    .get("distance_end")
                    .and_then(toml::Value::as_float)
                    .map(|v| v as f32);
                self.post.effects.push(PostEffectSettings {
                    name: name.to_string(),
                    enabled,
                    target,
                    intensity,
                    distance_start,
                    distance_end,
                });
            }
        }

        Ok(())
    }

    fn apply_quality_preset(&mut self, preset: RenderQualityPreset) {
        match preset {
            RenderQualityPreset::Low => {
                self.ao_samples = 0.0;
                self.bump_strength = 0.0;
                self.max_shadow_distance = 0.0;
                self.reflection_samples = 0.0;
                self.max_sky_distance = 15.0;
            }
            RenderQualityPreset::Medium => {
                self.ao_samples = 2.0;
                self.bump_strength = 0.25;
                self.max_shadow_distance = 5.0;
                self.reflection_samples = 0.0;
                self.max_sky_distance = 30.0;
            }
            RenderQualityPreset::High => {
                self.ao_samples = 4.0;
                self.bump_strength = 0.6;
                self.max_shadow_distance = 10.0;
                self.reflection_samples = 1.0;
                self.max_sky_distance = 50.0;
            }
            RenderQualityPreset::Ultra => {
                self.ao_samples = 8.0;
                self.bump_strength = 1.0;
                self.max_shadow_distance = 15.0;
                self.reflection_samples = 2.0;
                self.max_sky_distance = 75.0;
            }
            RenderQualityPreset::Custom => {}
        }
    }

    /// Schedule a timed render setting change.
    /// `time` is the duration in seconds over which the setting interpolates from its current value.
    pub fn set(
        &mut self,
        name: &str,
        value: Value,
        time: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(key) = Self::key_from_name(name) else {
            return Err(format!("Unknown render setting '{}'", name).into());
        };

        if key == SettingKey::FrameTimeMs {
            let ms = Self::value_to_f32(&value)
                .ok_or_else(|| format!("Expected numeric value for '{}'", name))?;
            self.frame_time_ms = ms.max(0.0);
            return Ok(());
        }

        let target = Self::parse_value_for_key(key, value)?;
        let duration = time.max(0.0);

        if duration == 0.0 {
            self.apply_setting_value(key, target);
            self.transitions.remove(&key);
            return Ok(());
        }

        let start = self.current_value(key);
        let transition = match (start, target) {
            (SettingValue::Float(s), SettingValue::Float(t)) => Transition::Float {
                start: s,
                target: t,
                duration,
                elapsed: 0.0,
            },
            (SettingValue::Vec3(s), SettingValue::Vec3(t)) => Transition::Vec3 {
                start: s,
                target: t,
                duration,
                elapsed: 0.0,
            },
            (SettingValue::Bool(s), SettingValue::Bool(t)) => Transition::Bool {
                start: s,
                target: t,
                duration,
                elapsed: 0.0,
            },
            _ => {
                return Err("Mismatched setting value types".into());
            }
        };

        self.transitions.insert(key, transition);

        Ok(())
    }

    /// Apply time-of-day settings based on the current hour (0.0 - 24.0)
    /// This interpolates sky color, sun color, and calculates sun direction procedurally
    /// Only applies if simulation is enabled
    pub fn apply_hour(&mut self, hour: f32) {
        if !self.simulation.enabled {
            return;
        }

        let sim = &self.simulation;
        let hour = hour.rem_euclid(24.0);

        // Sun rises at sunrise_time and sets at sunset_time.
        let transition = sim.color_transition_duration_hours.max(0.0);

        // Short transition windows where morning/evening are intermediate colors only.
        let morning_start = sim.sunrise_time - transition;
        let morning_end = sim.sunrise_time + transition;
        let midday_to_evening_start = (sim.sunset_time - transition).max(morning_end);
        let evening_end = sim.sunset_time + transition;

        let (sky_color, sun_color) = if hour >= morning_start && hour < sim.sunrise_time {
            // Pre-sunrise: night -> morning
            let t = (hour - morning_start) / transition.max(f32::EPSILON);
            let sky = lerp_color(sim.night_sky_color, sim.morning_sky_color, t);
            let sun = lerp_color(sim.night_sun_color, sim.morning_sun_color, t);
            (sky, sun)
        } else if hour >= sim.sunrise_time && hour < morning_end {
            // Morning -> midday
            let t = (hour - sim.sunrise_time) / transition.max(f32::EPSILON);
            let sky = lerp_color(sim.morning_sky_color, sim.midday_sky_color, t);
            let sun = lerp_color(sim.morning_sun_color, sim.midday_sun_color, t);
            (sky, sun)
        } else if hour >= morning_end && hour < midday_to_evening_start {
            // Midday hold
            (sim.midday_sky_color, sim.midday_sun_color)
        } else if hour >= midday_to_evening_start && hour < sim.sunset_time {
            // Midday -> evening
            let t = (hour - midday_to_evening_start)
                / (sim.sunset_time - midday_to_evening_start).max(f32::EPSILON);
            let sky = lerp_color(sim.midday_sky_color, sim.evening_sky_color, t);
            let sun = lerp_color(sim.midday_sun_color, sim.evening_sun_color, t);
            (sky, sun)
        } else if hour >= sim.sunset_time && hour < evening_end {
            // Post-sunset: evening -> night
            let t = (hour - sim.sunset_time) / transition.max(f32::EPSILON);
            let sky = lerp_color(sim.evening_sky_color, sim.night_sky_color, t);
            let sun = lerp_color(sim.evening_sun_color, sim.night_sun_color, t);
            (sky, sun)
        } else {
            // Deep night hold
            (sim.night_sky_color, sim.night_sun_color)
        };

        // Apply interpolated colors
        self.sky_color = sky_color;
        self.sun_color = sun_color;

        // Sun angle is continuous over daytime; twilight transitions to/from night.
        let day_length = (sim.sunset_time - sim.sunrise_time).max(f32::EPSILON);
        let daylight_progress = ((hour - sim.sunrise_time) / day_length).clamp(0.0, 1.0);
        let daylight_angle = (daylight_progress * std::f32::consts::PI).sin() * 90.0;
        let sun_angle = if hour >= sim.sunrise_time && hour < sim.sunset_time {
            daylight_angle
        } else if hour >= morning_start && hour < sim.sunrise_time {
            let t = (hour - morning_start) / transition.max(f32::EPSILON);
            lerp(-30.0, 0.0, t)
        } else if hour >= sim.sunset_time && hour < evening_end {
            let t = (hour - sim.sunset_time) / transition.max(f32::EPSILON);
            lerp(0.0, -30.0, t)
        } else {
            -30.0
        };

        // Calculate sun direction procedurally from time of day.
        // Sun travels from east (-1, 0, 0) through zenith (0, -1, 0) to west (1, 0, 0).
        let angle_rad = sun_angle.to_radians();
        let progress = daylight_progress;

        // X goes from -1 (east) to 1 (west)
        let x = lerp(-1.0, 1.0, progress);
        // Y is based on angle above horizon
        let y = -angle_rad.sin();
        // Z stays slightly forward
        let z = -0.3;

        self.sun_direction = [x, y, z];
    }

    /// Apply these render settings to a SceneVM instance
    pub fn apply_2d(&mut self, vm: &mut SceneVM) {
        self.update_transitions();

        // gp1: Sky color (RGB) + unused w
        vm.execute(Atom::SetGP1(Vec4::new(
            self.sky_color[0],
            self.sky_color[1],
            self.sky_color[2],
            0.0,
        )));
    }

    /// Apply these render settings to a SceneVM instance
    pub fn apply_3d(&mut self, vm: &mut SceneVM) {
        self.update_transitions();

        // Convert sRGB colors to linear space (gamma 2.2) on CPU instead of per-pixel in shader
        let to_linear = |c: f32| c.powf(2.2);

        // gp0: Sky color (RGB, linear) + unused w
        vm.execute(Atom::SetGP0(Vec4::new(
            to_linear(self.sky_color[0]),
            to_linear(self.sky_color[1]),
            to_linear(self.sky_color[2]),
            0.0,
        )));

        // gp1: Sun color (RGB, linear) + sun intensity (w)
        vm.execute(Atom::SetGP1(Vec4::new(
            to_linear(self.sun_color[0]),
            to_linear(self.sun_color[1]),
            to_linear(self.sun_color[2]),
            self.sun_intensity,
        )));

        // gp2: Sun direction (XYZ, normalized) + sun enabled (w)
        let sun_dir = vek::Vec3::from(self.sun_direction).normalized();
        vm.execute(Atom::SetGP2(Vec4::new(
            sun_dir.x,
            sun_dir.y,
            sun_dir.z,
            if self.sun_enabled { 1.0 } else { 0.0 },
        )));

        // gp3: Ambient color (RGB, linear) + ambient strength (w)
        vm.execute(Atom::SetGP3(Vec4::new(
            to_linear(self.ambient_color[0]),
            to_linear(self.ambient_color[1]),
            to_linear(self.ambient_color[2]),
            self.ambient_strength,
        )));

        // gp4: Fog color (RGB, linear) + fog density (w)
        vm.execute(Atom::SetGP4(Vec4::new(
            to_linear(self.fog_color[0]),
            to_linear(self.fog_color[1]),
            to_linear(self.fog_color[2]),
            self.fog_density,
        )));

        // gp5: Rendering quality settings
        // x: AO samples, y: AO radius, z: Bump strength, w: Max transparency bounces
        vm.execute(Atom::SetGP5(Vec4::new(
            self.ao_samples,
            self.ao_radius,
            self.bump_strength,
            self.max_transparency_bounces,
        )));
        vm.execute(Atom::SetRaster3DMsaaSamples(self.msaa_samples));

        // gp6: Distance/settings
        // x: Max shadow distance, y: Max sky distance, z: FirstP blur near, w: FirstP blur far
        vm.execute(Atom::SetGP6(Vec4::new(
            self.max_shadow_distance,
            self.max_sky_distance,
            self.firstp_blur_near.max(0.0),
            self.firstp_blur_far
                .max(self.firstp_blur_near.max(0.0) + 0.001),
        )));

        // gp7: raster-3d specific controls
        // x: shadow enabled (0/1), y: shadow strength, z: shadow resolution, w: shadow bias
        vm.execute(Atom::SetGP7(Vec4::new(
            if self.raster_shadow_enabled { 1.0 } else { 0.0 },
            self.raster_shadow_strength.clamp(0.0, 1.0),
            self.raster_shadow_resolution.max(64.0),
            self.raster_shadow_bias.max(0.0),
        )));

        // gp8.x: fade mode (0 = ordered_dither, 1 = uniform)
        // gp8.y: lighting model (0 = lambert, 1 = cook_torrance, 2 = pbr)
        // gp8.z: post saturation, gp8.w: post luminance
        vm.execute(Atom::SetGP8(Vec4::new(
            self.fade_mode.as_code() as f32,
            self.lighting_model.as_code() as f32,
            self.post_saturation.max(0.0),
            self.post_luminance.max(0.0),
        )));

        // gp9: post-processing controls
        // x: post enabled (0/1), y: tone mapper (0=none,1=reinhard,2=aces), z: exposure, w: gamma
        vm.execute(Atom::SetGP9(Vec4::new(
            if self.post_enabled { 1.0 } else { 0.0 },
            self.post_tone_mapper.as_code() as f32,
            self.post_exposure.max(0.0),
            self.post_gamma.max(0.001),
        )));
        vm.vm.set_raster3d_avatar_highlight_params(Vec4::new(
            self.avatar_highlight_lift.max(0.0),
            self.avatar_highlight_fill.max(0.0),
            self.avatar_highlight_rim.max(0.0),
            if self.avatar_highlight_enabled {
                1.0
            } else {
                0.0
            },
        ));
    }

    fn update_transitions(&mut self) {
        if self.transitions.is_empty() {
            return;
        }

        let dt = (self.frame_time_ms / 1000.0).max(0.0001);
        let mut finished = Vec::new();
        let mut updates = Vec::new();

        for (key, transition) in self.transitions.iter_mut() {
            match transition {
                Transition::Float {
                    start,
                    target,
                    duration,
                    elapsed,
                } => {
                    *elapsed += dt;
                    let progress = if *duration == 0.0 {
                        1.0
                    } else {
                        (*elapsed / *duration).clamp(0.0, 1.0)
                    };
                    let value = lerp(*start, *target, progress);
                    updates.push((*key, SettingValue::Float(value)));

                    if progress >= 1.0 {
                        finished.push(*key);
                    }
                }
                Transition::Vec3 {
                    start,
                    target,
                    duration,
                    elapsed,
                } => {
                    *elapsed += dt;
                    let progress = if *duration == 0.0 {
                        1.0
                    } else {
                        (*elapsed / *duration).clamp(0.0, 1.0)
                    };
                    let value = lerp_color(*start, *target, progress);
                    updates.push((*key, SettingValue::Vec3(value)));

                    if progress >= 1.0 {
                        finished.push(*key);
                    }
                }
                Transition::Bool {
                    start,
                    target,
                    duration,
                    elapsed,
                } => {
                    *elapsed += dt;
                    let done = *duration == 0.0 || *elapsed >= *duration;
                    let value = if done { *target } else { *start };
                    updates.push((*key, SettingValue::Bool(value)));
                    if done {
                        finished.push(*key);
                    }
                }
            }
        }

        for (key, value) in updates {
            self.apply_setting_value(key, value);
        }
        for key in finished {
            self.transitions.remove(&key);
        }
    }

    fn current_value(&self, key: SettingKey) -> SettingValue {
        match key {
            SettingKey::SkyColor => SettingValue::Vec3(self.sky_color),
            SettingKey::SunColor => SettingValue::Vec3(self.sun_color),
            SettingKey::SunIntensity => SettingValue::Float(self.sun_intensity),
            SettingKey::SunDirection => SettingValue::Vec3(self.sun_direction),
            SettingKey::SunEnabled => SettingValue::Bool(self.sun_enabled),
            SettingKey::AmbientColor => SettingValue::Vec3(self.ambient_color),
            SettingKey::AmbientStrength => SettingValue::Float(self.ambient_strength),
            SettingKey::FogColor => SettingValue::Vec3(self.fog_color),
            SettingKey::FogDensity => SettingValue::Float(self.fog_density),
            SettingKey::AoSamples => SettingValue::Float(self.ao_samples),
            SettingKey::AoRadius => SettingValue::Float(self.ao_radius),
            SettingKey::BumpStrength => SettingValue::Float(self.bump_strength),
            SettingKey::MaxTransparencyBounces => {
                SettingValue::Float(self.max_transparency_bounces)
            }
            SettingKey::MaxShadowDistance => SettingValue::Float(self.max_shadow_distance),
            SettingKey::MaxSkyDistance => SettingValue::Float(self.max_sky_distance),
            SettingKey::MaxShadowSteps => SettingValue::Float(self.max_shadow_steps),
            SettingKey::ReflectionSamples => SettingValue::Float(self.reflection_samples),
            SettingKey::FrameTimeMs => SettingValue::Float(self.frame_time_ms),
        }
    }

    fn apply_setting_value(&mut self, key: SettingKey, value: SettingValue) {
        match (key, value) {
            (SettingKey::SkyColor, SettingValue::Vec3(v)) => self.sky_color = v,
            (SettingKey::SunColor, SettingValue::Vec3(v)) => self.sun_color = v,
            (SettingKey::SunIntensity, SettingValue::Float(v)) => self.sun_intensity = v,
            (SettingKey::SunDirection, SettingValue::Vec3(v)) => self.sun_direction = v,
            (SettingKey::SunEnabled, SettingValue::Bool(v)) => self.sun_enabled = v,
            (SettingKey::AmbientColor, SettingValue::Vec3(v)) => self.ambient_color = v,
            (SettingKey::AmbientStrength, SettingValue::Float(v)) => self.ambient_strength = v,
            (SettingKey::FogColor, SettingValue::Vec3(v)) => self.fog_color = v,
            (SettingKey::FogDensity, SettingValue::Float(v)) => self.fog_density = v,
            (SettingKey::AoSamples, SettingValue::Float(v)) => self.ao_samples = v,
            (SettingKey::AoRadius, SettingValue::Float(v)) => self.ao_radius = v,
            (SettingKey::BumpStrength, SettingValue::Float(v)) => self.bump_strength = v,
            (SettingKey::MaxTransparencyBounces, SettingValue::Float(v)) => {
                self.max_transparency_bounces = v
            }
            (SettingKey::MaxShadowDistance, SettingValue::Float(v)) => self.max_shadow_distance = v,
            (SettingKey::MaxSkyDistance, SettingValue::Float(v)) => self.max_sky_distance = v,
            (SettingKey::MaxShadowSteps, SettingValue::Float(v)) => self.max_shadow_steps = v,
            (SettingKey::ReflectionSamples, SettingValue::Float(v)) => self.reflection_samples = v,
            (SettingKey::FrameTimeMs, SettingValue::Float(v)) => self.frame_time_ms = v,
            _ => {}
        }
    }

    fn parse_value_for_key(
        key: SettingKey,
        value: Value,
    ) -> Result<SettingValue, Box<dyn std::error::Error>> {
        match key {
            SettingKey::SkyColor
            | SettingKey::SunColor
            | SettingKey::SunDirection
            | SettingKey::AmbientColor
            | SettingKey::FogColor => match value {
                Value::Vec3(v) => Ok(SettingValue::Vec3(v)),
                Value::Vec4(v) => Ok(SettingValue::Vec3([v[0], v[1], v[2]])),
                Value::Str(s) => Ok(SettingValue::Vec3(parse_hex_color(&s)?)),
                _ => Err(format!("Expected Vec3 or hex color for {:?}", key).into()),
            },
            SettingKey::SunEnabled => match value {
                Value::Bool(b) => Ok(SettingValue::Bool(b)),
                _ => Err("Expected bool for sun_enabled".into()),
            },
            SettingKey::SunIntensity
            | SettingKey::AmbientStrength
            | SettingKey::FogDensity
            | SettingKey::AoSamples
            | SettingKey::AoRadius
            | SettingKey::BumpStrength
            | SettingKey::MaxTransparencyBounces
            | SettingKey::MaxShadowDistance
            | SettingKey::MaxSkyDistance
            | SettingKey::MaxShadowSteps
            | SettingKey::ReflectionSamples
            | SettingKey::FrameTimeMs => {
                let Some(v) = Self::value_to_f32(&value) else {
                    return Err(format!("Expected numeric value for {:?}", key).into());
                };
                Ok(SettingValue::Float(v))
            }
        }
    }

    fn value_to_f32(value: &Value) -> Option<f32> {
        match value {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f32),
            Value::UInt(v) => Some(*v as f32),
            Value::Int64(v) => Some(*v as f32),
            _ => None,
        }
    }

    fn key_from_name(name: &str) -> Option<SettingKey> {
        match name {
            "sky_color" => Some(SettingKey::SkyColor),
            "sun_color" => Some(SettingKey::SunColor),
            "sun_intensity" => Some(SettingKey::SunIntensity),
            "sun_direction" => Some(SettingKey::SunDirection),
            "sun_enabled" => Some(SettingKey::SunEnabled),
            "ambient_color" => Some(SettingKey::AmbientColor),
            "ambient_strength" => Some(SettingKey::AmbientStrength),
            "fog_color" => Some(SettingKey::FogColor),
            "fog_density" => Some(SettingKey::FogDensity),
            "ao_samples" => Some(SettingKey::AoSamples),
            "ao_radius" => Some(SettingKey::AoRadius),
            "bump_strength" => Some(SettingKey::BumpStrength),
            "max_transparency_bounces" => Some(SettingKey::MaxTransparencyBounces),
            "max_shadow_distance" => Some(SettingKey::MaxShadowDistance),
            "max_sky_distance" => Some(SettingKey::MaxSkyDistance),
            "max_shadow_steps" => Some(SettingKey::MaxShadowSteps),
            "reflection_samples" => Some(SettingKey::ReflectionSamples),
            "ms_per_frame" => Some(SettingKey::FrameTimeMs),
            _ => None,
        }
    }
}

impl RenderSettings {
    fn apply_render_values(
        &mut self,
        render: &ValueContainer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(v) = render.get_str("sky_color") {
            self.sky_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("sky_color") {
            self.sky_color = v;
        }

        if let Some(v) = render.get_str("sun_color") {
            self.sun_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("sun_color") {
            self.sun_color = v;
        }

        self.sun_intensity = render.get_float_default("sun_intensity", self.sun_intensity);
        self.sun_direction = render.get_vec3_default("sun_direction", self.sun_direction);
        self.sun_enabled = render.get_bool_default("sun_enabled", self.sun_enabled);

        if let Some(v) = render.get_str("ambient_color") {
            self.ambient_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("ambient_color") {
            self.ambient_color = v;
        }

        self.ambient_strength = render.get_float_default("ambient_strength", self.ambient_strength);

        if let Some(v) = render.get_str("fog_color") {
            self.fog_color = parse_hex_color(v)?;
        } else if let Some(v) = render.get_vec3("fog_color") {
            self.fog_color = v;
        }

        // keep legacy percent scaling
        if let Some(d) = render.get_float("fog_density") {
            self.fog_density = d / 100.0;
        }
        self.ao_samples = render.get_float_default("ao_samples", self.ao_samples);
        self.ao_radius = render.get_float_default("ao_radius", self.ao_radius);
        self.bump_strength = render.get_float_default("bump_strength", self.bump_strength);
        self.msaa_samples = render
            .get_int("msaa_samples")
            .map(|v| v.max(0) as u32)
            .or_else(|| {
                render
                    .get_float("msaa_samples")
                    .map(|v| v.max(0.0).round() as u32)
            })
            .unwrap_or(self.msaa_samples);
        self.msaa_samples = if self.msaa_samples == 0 { 0 } else { 4 };
        self.max_transparency_bounces =
            render.get_float_default("max_transparency_bounces", self.max_transparency_bounces);
        self.max_shadow_distance =
            render.get_float_default("max_shadow_distance", self.max_shadow_distance);
        self.max_sky_distance = render.get_float_default("max_sky_distance", self.max_sky_distance);
        self.max_shadow_steps = render.get_float_default("max_shadow_steps", self.max_shadow_steps);
        self.reflection_samples =
            render.get_float_default("reflection_samples", self.reflection_samples);
        self.firstp_blur_near = render.get_float_default("firstp_blur_near", self.firstp_blur_near);
        self.firstp_blur_far = render.get_float_default("firstp_blur_far", self.firstp_blur_far);
        // Raster shadow controls (backward-compatible under [render])
        self.raster_shadow_enabled = render.get_bool_default(
            "shadow_enabled",
            render.get_bool_default("raster_shadow_enabled", self.raster_shadow_enabled),
        );
        self.raster_shadow_strength = render.get_float_default(
            "shadow_strength",
            render.get_float_default("raster_shadow_strength", self.raster_shadow_strength),
        );
        self.raster_shadow_resolution = render.get_float_default(
            "shadow_resolution",
            render.get_float_default("raster_shadow_resolution", self.raster_shadow_resolution),
        );
        self.raster_shadow_bias = render.get_float_default(
            "shadow_bias",
            render.get_float_default("raster_shadow_bias", self.raster_shadow_bias),
        );
        self.fade_mode = render
            .get_str("fade_mode")
            .map(parse_fade_mode)
            .unwrap_or(self.fade_mode);
        self.lighting_model = render
            .get_str("lighting_model")
            .map(parse_lighting_model)
            .unwrap_or(self.lighting_model);
        self.avatar_highlight_enabled =
            render.get_bool_default("avatar_highlight_enabled", self.avatar_highlight_enabled);
        self.avatar_highlight_lift =
            render.get_float_default("avatar_highlight_lift", self.avatar_highlight_lift);
        self.avatar_highlight_fill =
            render.get_float_default("avatar_highlight_fill", self.avatar_highlight_fill);
        self.avatar_highlight_rim =
            render.get_float_default("avatar_highlight_rim", self.avatar_highlight_rim);
        self.frame_time_ms = render.get_float_default("ms_per_frame", self.frame_time_ms);
        if let Some(fps) = render.get_float("fps") {
            if fps > 0.0 {
                self.frame_time_ms = 1000.0 / fps;
            }
        }

        Ok(())
    }

    fn apply_simulation_values(
        &mut self,
        sim: &ValueContainer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.simulation.enabled = sim.get_bool_default("enabled", self.simulation.enabled);

        if let Some(v) = sim.get_str("night_sky_color") {
            self.simulation.night_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("night_sky_color") {
            self.simulation.night_sky_color = v;
        }

        if let Some(v) = sim.get_str("morning_sky_color") {
            self.simulation.morning_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("morning_sky_color") {
            self.simulation.morning_sky_color = v;
        }

        if let Some(v) = sim.get_str("midday_sky_color") {
            self.simulation.midday_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("midday_sky_color") {
            self.simulation.midday_sky_color = v;
        }

        if let Some(v) = sim.get_str("evening_sky_color") {
            self.simulation.evening_sky_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("evening_sky_color") {
            self.simulation.evening_sky_color = v;
        }

        if let Some(v) = sim.get_str("night_sun_color") {
            self.simulation.night_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("night_sun_color") {
            self.simulation.night_sun_color = v;
        }

        if let Some(v) = sim.get_str("morning_sun_color") {
            self.simulation.morning_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("morning_sun_color") {
            self.simulation.morning_sun_color = v;
        }

        if let Some(v) = sim.get_str("midday_sun_color") {
            self.simulation.midday_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("midday_sun_color") {
            self.simulation.midday_sun_color = v;
        }

        if let Some(v) = sim.get_str("evening_sun_color") {
            self.simulation.evening_sun_color = parse_hex_color(v)?;
        } else if let Some(v) = sim.get_vec3("evening_sun_color") {
            self.simulation.evening_sun_color = v;
        }

        self.simulation.sunrise_time =
            sim.get_float_default("sunrise_time", self.simulation.sunrise_time);
        self.simulation.sunset_time =
            sim.get_float_default("sunset_time", self.simulation.sunset_time);
        self.simulation.color_transition_duration_hours = sim.get_float_default(
            "color_transition_duration_hours",
            self.simulation.color_transition_duration_hours,
        );

        Ok(())
    }
}

/// Linear interpolation between two f32 values
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linear interpolation between two RGB colors
fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

/// Parse a hex color string like "#RRGGBB" or "RRGGBB" into RGB floats (0.0-1.0)
fn parse_hex_color(hex: &str) -> Result<[f32; 3], Box<dyn std::error::Error>> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(format!(
            "Invalid hex color: expected 6 characters, got {}",
            hex.len()
        )
        .into());
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

fn parse_backend(v: &str) -> RendererBackend {
    match v.to_ascii_lowercase().as_str() {
        "raster" => RendererBackend::Raster,
        _ => RendererBackend::Compute,
    }
}

fn parse_quality(v: &str) -> RenderQualityPreset {
    match v.to_ascii_lowercase().as_str() {
        "low" => RenderQualityPreset::Low,
        "medium" => RenderQualityPreset::Medium,
        "high" => RenderQualityPreset::High,
        "ultra" => RenderQualityPreset::Ultra,
        _ => RenderQualityPreset::Custom,
    }
}

fn parse_post_target(v: &str) -> PostTarget {
    match v.to_ascii_lowercase().as_str() {
        "2d" => PostTarget::D2,
        "3d" => PostTarget::D3,
        _ => PostTarget::Both,
    }
}

fn parse_fade_mode(v: &str) -> FadeMode {
    match v.to_ascii_lowercase().as_str() {
        "uniform" | "uniformn" => FadeMode::Uniform,
        _ => FadeMode::OrderedDither,
    }
}

fn parse_lighting_model(v: &str) -> LightingModel {
    match v.to_ascii_lowercase().as_str() {
        "lambert" => LightingModel::Lambert,
        "cook_torrance" => LightingModel::CookTorrance,
        "pbr" => LightingModel::Pbr,
        _ => LightingModel::CookTorrance,
    }
}

fn parse_tone_mapper(v: &str) -> PostToneMapper {
    match v.to_ascii_lowercase().as_str() {
        "none" => PostToneMapper::None,
        "aces" => PostToneMapper::Aces,
        _ => PostToneMapper::Reinhard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_example_toml() {
        let example = include_str!("../render_settings_example.toml");
        let mut settings = RenderSettings::default();
        settings.read(example).expect("render settings parse");

        assert_eq!(settings.sky_color, [0.5294118, 0.80784315, 0.92156863]); // #87CEEB
        assert_eq!(settings.sun_color, [1.0, 0.98039216, 0.8039216]); // #FFFACD
        assert_eq!(settings.sun_intensity, 1.0);
        assert_eq!(settings.sun_direction, [-0.5, -1.0, -0.3]);
        assert!(settings.sun_enabled);
        assert!(settings.simulation.enabled);
        assert_eq!(settings.simulation.sunrise_time, 6.0);
        assert_eq!(settings.simulation.sunset_time, 18.0);
        assert_eq!(settings.simulation.color_transition_duration_hours, 0.5);
    }

    #[test]
    fn interpolates_with_set() {
        let mut settings = RenderSettings::default();
        settings.frame_time_ms = 1000.0; // 1 second per update for predictable progress

        settings
            .set("sun_intensity", Value::Float(3.0), 2.0)
            .expect("set sun_intensity");

        settings.update_transitions();
        assert!((settings.sun_intensity - 2.0).abs() < f32::EPSILON);

        settings.update_transitions();
        assert!((settings.sun_intensity - 3.0).abs() < f32::EPSILON);
        assert!(settings.transitions.is_empty());
    }
}
