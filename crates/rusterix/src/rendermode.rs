/// The RenderMode defines the features for the Rasterizer.
#[derive(Clone, PartialEq)]
pub struct RenderMode {
    /// Render 2D batches
    pub d2_active: bool,
    /// Render 3D batches
    pub d3_active: bool,
}

impl RenderMode {
    pub fn render_all() -> Self {
        Self {
            d2_active: true,
            d3_active: true,
        }
    }

    pub fn render_2d() -> Self {
        Self {
            d2_active: true,
            d3_active: false,
        }
    }

    pub fn render_3d() -> Self {
        Self {
            d2_active: false,
            d3_active: true,
        }
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
