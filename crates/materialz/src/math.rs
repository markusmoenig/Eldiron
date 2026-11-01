//! Canonical float32 trigonometry for the portable tile IR.
//! Hash noises amplify single-ULP vendor sin differences into unrelated patterns.
//! CPU and Metal therefore use the same range reduction and polynomial order.
pub(crate) fn sin(x: f32) -> f32 {
    let q = (x * 0.159_154_94 + 0.5).floor();
    let mut r = (x - q * 6.281_25) - q * 0.001_935_307_2;
    if r > std::f32::consts::FRAC_PI_2 {
        r = std::f32::consts::PI - r;
    } else if r < -std::f32::consts::FRAC_PI_2 {
        r = -std::f32::consts::PI - r;
    }
    let z = r * r;
    let p = 1.605_904_4e-10 * z - 2.505_210_8e-8;
    let p = p * z + 2.755_731_9e-6;
    let p = p * z - 1.984_127e-4;
    let p = p * z + 8.333_334e-3;
    let p = p * z - 1.666_666_7e-1;
    r + (r * z) * p
}
pub(crate) fn cos(x: f32) -> f32 {
    sin(x + std::f32::consts::FRAC_PI_2)
}
#[cfg(test)]
mod tests {
    #[test]
    fn canonical_trig_tracks_reference_on_the_principal_range() {
        for i in -10000..=10000 {
            let x = i as f32 / 1000.;
            assert!((super::sin(x) - x.sin()).abs() < 0.000002);
            assert!((super::cos(x) - x.cos()).abs() < 0.000003);
        }
    }
}
