use vek::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum CameraKind {
    OrthoIso,
    OrbitPersp,
    FirstPersonPersp,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub kind: CameraKind,
    pub pos: Vec3<f32>,
    pub forward: Vec3<f32>, // normalized
    pub right: Vec3<f32>,   // normalized
    pub up: Vec3<f32>,      // normalized
    pub vfov_deg: f32,      // for perspective
    pub ortho_half_h: f32,  // for ortho
    pub near: f32,
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            kind: CameraKind::OrbitPersp,
            pos: Vec3::new(0.0, 0.0, 5.0),
            forward: Vec3::new(0.0, 0.0, -1.0),
            right: Vec3::new(1.0, 0.0, 0.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            vfov_deg: 50.0,
            ortho_half_h: 5.0,
            near: 0.01,
            far: 1000.0,
        }
    }
}

impl Camera3D {
    pub fn iso() -> Self {
        let dir = Vec3::new(1.0, -1.0, -1.0).normalized();
        let right = Vec3::new(1.0, 1.0, 0.0).normalized();
        let up = right.cross(dir).normalized();
        Self {
            kind: CameraKind::OrthoIso,
            pos: Vec3::new(0.0, 0.0, 0.0),
            forward: dir,
            right,
            up,
            vfov_deg: 50.0,
            ortho_half_h: 5.0,
            near: 0.01,
            far: 1000.0,
        }
    }

    // ---- Builder-style fluent API ----

    #[inline]
    pub fn builder() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_kind(mut self, kind: CameraKind) -> Self {
        self.kind = kind;
        self
    }

    #[inline]
    pub fn with_pos(mut self, pos: Vec3<f32>) -> Self {
        self.pos = pos;
        self
    }

    /// Sets forward/right/up ensuring an orthonormal basis (right = forward × up_hint).
    #[inline]
    pub fn with_basis(mut self, forward: Vec3<f32>, up_hint: Vec3<f32>) -> Self {
        let f = forward.normalized();
        // If forward and up_hint are nearly collinear, fall back to world up.
        let mut r = f.cross(if up_hint.magnitude_squared() > 1e-8 {
            up_hint
        } else {
            Vec3::unit_y()
        });
        if r.magnitude_squared() < 1e-12 {
            r = f.cross(Vec3::unit_x());
        }
        r = r.normalized();
        let u = r.cross(f).normalized();
        self.forward = f;
        self.right = r;
        self.up = u;
        self
    }

    /// Set the full camera basis exactly as provided (no recomputation/normalization).
    /// Use when you already computed an orthonormal basis elsewhere (e.g., Rusterix).
    #[inline]
    pub fn with_basis_exact(mut self, forward: Vec3<f32>, right: Vec3<f32>, up: Vec3<f32>) -> Self {
        self.forward = forward;
        self.right = right;
        self.up = up;
        self
    }

    /// Non-consuming variant of `with_basis_exact`.
    #[inline]
    pub fn set_basis_exact(&mut self, forward: Vec3<f32>, right: Vec3<f32>, up: Vec3<f32>) {
        self.forward = forward;
        self.right = right;
        self.up = up;
    }

    /// Convenience: look-at style setup using position, target and up hint.
    #[inline]
    pub fn look_at(mut self, pos: Vec3<f32>, target: Vec3<f32>, up_hint: Vec3<f32>) -> Self {
        self.pos = pos;
        let f = (target - pos).normalized();
        // Reuse with_basis to compute right/up from the hint
        self.with_basis(f, up_hint)
    }

    #[inline]
    pub fn with_forward(mut self, forward: Vec3<f32>) -> Self {
        // Keep existing up; recompute right/up to stay orthonormal
        self = self.with_basis(forward, self.up);
        self
    }

    #[inline]
    pub fn with_up(mut self, up: Vec3<f32>) -> Self {
        // Keep existing forward; recompute right/up to stay orthonormal
        self = self.with_basis(self.forward, up);
        self
    }

    /// Force-set right (rarely needed). Will re-orthonormalize with current forward.
    #[inline]
    pub fn with_right(mut self, right: Vec3<f32>) -> Self {
        let f = self.forward.normalized();
        let mut r = right.normalized();
        // Ensure right is not collinear with forward
        if (r.dot(f)).abs() > 0.9999 {
            r = f.cross(Vec3::unit_y()).normalized();
        }
        let u = r.cross(f).normalized();
        self.right = r;
        self.up = u;
        self
    }

    #[inline]
    pub fn with_vfov_deg(mut self, vfov_deg: f32) -> Self {
        self.vfov_deg = vfov_deg;
        self
    }

    #[inline]
    pub fn with_ortho_half_h(mut self, ortho_half_h: f32) -> Self {
        self.ortho_half_h = ortho_half_h;
        self
    }

    #[inline]
    pub fn with_near(mut self, near: f32) -> Self {
        self.near = near.max(1e-6);
        self
    }

    #[inline]
    pub fn with_far(mut self, far: f32) -> Self {
        self.far = far;
        self
    }

    /// Convenience for perspective cameras.
    #[inline]
    pub fn with_perspective(mut self, vfov_deg: f32, near: f32, far: f32) -> Self {
        self.kind = CameraKind::OrbitPersp;
        self.vfov_deg = vfov_deg;
        self.near = near.max(1e-6);
        self.far = far;
        self
    }

    /// Convenience for orthographic/isometric cameras.
    #[inline]
    pub fn with_ortho(mut self, half_h: f32, near: f32, far: f32) -> Self {
        self.kind = CameraKind::OrthoIso;
        self.ortho_half_h = half_h.max(1e-6);
        self.near = near.max(1e-6);
        self.far = far;
        self
    }
}
