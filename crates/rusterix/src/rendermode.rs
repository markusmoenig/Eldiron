/// The RenderMode defines the features for the Rasterizer.
#[derive(Clone, PartialEq)]
pub struct RenderMode {
    /// Render 2D batches
    pub d2_active: bool,
    /// Render 3D batches
    pub d3_active: bool,
    /// Flag to ignore the background shader in the scene
    pub ignore_background_shader: bool,
}

impl RenderMode {
    pub fn render_all() -> Self {
        Self {
            d2_active: true,
            d3_active: true,
            ignore_background_shader: false,
        }
    }

    pub fn render_2d() -> Self {
        Self {
            d2_active: true,
            d3_active: false,
            ignore_background_shader: false,
        }
    }

    pub fn render_3d() -> Self {
        Self {
            d2_active: false,
            d3_active: true,
            ignore_background_shader: false,
        }
    }

    /// Ignores the background shader.
    pub fn ignore_background_shader(mut self, value: bool) -> Self {
        self.ignore_background_shader = value;
        self
    }

    #[inline(always)]
    pub fn supports2d(&self) -> bool {
        self.d2_active
    }

    #[inline(always)]
    pub fn supports3d(&self) -> bool {
        self.d3_active
    }
}
