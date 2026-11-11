use crate::prelude::*;

pub mod thepattern;
pub mod thesdfcanvas;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TheSDF {
    Circle(TheDim),
    Hexagon(TheDim),
    Rhombus(TheDim),
    RoundedRect(TheDim, (f32, f32, f32, f32)),
}

use TheSDF::*;

impl TheSDF {
    pub fn distance(&self, p: Vec2<f32>) -> f32 {
        match self {
            Circle(dim) => (p - dim.center()).magnitude() - dim.radius(),
            Hexagon(dim) => {
                let mut pp = (p - dim.center()).map(|v| v.abs());
                let r = dim.radius() - dim.radius() / 8.0;

                let k = Vec3::new(-0.866_025_4, 0.5, 0.577_350_26);
                pp -= 2.0 * k.xy() * k.xy().dot(pp).min(0.0);
                pp -= Vec2::new(pp.x.clamped(-k.z * r, k.z * r), r);
                pp.magnitude() * pp.y.signum()
            }
            Rhombus(dim) => {
                // fn ndot(a: Vec2<f32>, b: Vec2<f32>) -> f32 {
                //     a.x * b.x - a.y * b.y
                // }
                let pp = (p - dim.center()).map(|v| v.abs());
                let b = Vec2::new(dim.radius(), dim.radius());

                let delta = b - 2.0 * pp;
                let h = ((delta.x * b.x - delta.y * b.y) / b.magnitude_squared().max(f32::EPSILON))
                    .clamped(-1.0, 1.0);
                let mut d = (pp - 0.5 * b * Vec2::new(1.0 - h, 1.0 + h)).magnitude();
                d *= (pp.x * b.y + pp.y * b.x - b.x * b.y).signum();
                d
            }
            RoundedRect(dim, rounding) => {
                let pp = p - dim.center();
                let mut r = if pp.x > 0.0 {
                    Vec2::new(rounding.0, rounding.1)
                } else {
                    Vec2::new(rounding.2, rounding.3)
                };

                if pp.y <= 0.0 {
                    r.x = r.y;
                }

                let q = Vec2::new(
                    pp.x.abs() - dim.width as f32 / 2.0 + r.x,
                    pp.y.abs() - dim.height as f32 / 2.0 + r.x,
                );

                f32::min(q.x.max(q.y), 0.0) + Vec2::new(q.x.max(0.0), q.y.max(0.0)).magnitude()
                    - r.x
            }
        }
    }

    /// Returns a description of the SDF as string.
    pub fn describe(&self) -> String {
        match self {
            Circle(dim) => format!("Circle: {:?} {}", dim.center(), dim.radius()),
            Hexagon(dim) => format!("Hexagon: {:?} {}", dim.center(), dim.radius()),
            Rhombus(dim) => format!("Hexagon: {:?} {}", dim.center(), dim.radius()),
            RoundedRect(dim, rounding) => {
                format!(
                    "RoundedRect: {:?} {} {:?}",
                    dim.center(),
                    dim.radius(),
                    rounding
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_shapes_contain_their_interiors_and_exclude_exterior_points() {
        let d = TheDim::new(10, 20, 40, 40);
        for shape in [
            TheSDF::Circle(d),
            TheSDF::Hexagon(d),
            TheSDF::Rhombus(d),
            TheSDF::RoundedRect(d, (0., 0., 0., 0.)),
            TheSDF::RoundedRect(d, (5., 5., 5., 5.)),
        ] {
            assert!(shape.distance(Vec2::new(30., 40.)) < 0., "{shape:?}");
            assert!(shape.distance(Vec2::new(80., 40.)) > 0., "{shape:?}");
        }
        let diamond = TheSDF::Rhombus(d);
        assert!(diamond.distance(Vec2::new(40., 40.)) < 0.);
        assert!(diamond.distance(Vec2::new(50., 40.)).abs() < 0.001);
    }
}
