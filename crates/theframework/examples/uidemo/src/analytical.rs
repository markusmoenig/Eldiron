use rust_pathtracer::prelude::*;

pub struct AnalyticalScene {
    lights: Vec<AnalyticalLight>,
    pinhole: Box<dyn Camera3D>,
    material: Material,
}

// The Scene

impl Scene for AnalyticalScene {
    fn new() -> Self {
        let em = 3.0;
        let lights = vec![AnalyticalLight::spherical(
            F3::new(2.0, 2.0, 2.0),
            1.0,
            F3::new(em, em, em),
        )];

        let mut camera = Pinhole::new();
        camera.set(F3::new(0.0, 0.3, 3.0), F3::new(0.0, 0.2, 0.0));

        Self {
            lights,
            pinhole: Box::new(camera),
            material: Material::new(),
        }
    }

    fn camera(&self) -> &Box<dyn Camera3D> {
        &self.pinhole
    }

    fn background(&self, ray: &Ray) -> F3 {
        // Taken from https://raytracing.github.io/books/RayTracingInOneWeekend.html, a source of great knowledge
        let t = 0.5 * (ray.direction.y + 1.0);
        self.to_linear((1.0 - t) * F3::new(1.0, 1.0, 1.0) + t * F3::new(0.5, 0.7, 1.0))
            * F3::new_x(0.5)
    }

    /// The closest hit, includes light sources.
    fn closest_hit(&self, ray: &Ray, state: &mut State, light_sample: &mut LightSampleRec) -> bool {
        let mut dist = F::MAX;
        let mut hit = false;

        let center = F3::new(0.0, 0.3, 0.0);

        if let Some(d) = self.sphere(ray, center, 1.3) {
            if d < dist {
                let hp = ray.at(&d);
                let normal = normalize(&(hp - center));

                state.hit_dist = d;
                state.normal = normal;

                state.material = self.material.clone();

                hit = true;
                dist = d;
            }
        }

        if let Some(d) = self.plane(ray) {
            if d < dist {
                state.hit_dist = d;
                state.normal = F3::new(0.0, 1.0, 0.0);

                fn checker(x: F, y: F) -> F {
                    let x1 = x.floor() % 2.0;
                    let y1 = y.floor() % 2.0;
                    if (x1 + y1) % 2.0 < 1.0 {
                        0.25
                    } else {
                        0.1
                    }
                }

                let c = checker(
                    ray.direction.x / ray.direction.y * 0.5 + 100.0,
                    ray.direction.z / ray.direction.y * 0.5 + 100.0,
                );

                state.material.rgb = F3::new(c, c, c);
                state.material.roughness = 1.0;

                hit = true;
            }
        }

        if self.sample_lights(ray, state, light_sample, &self.lights) {
            hit = true;
        }

        hit
    }

    /// Any hit
    fn any_hit(&self, ray: &Ray, _max_dist: F) -> bool {
        if let Some(_d) = self.sphere(ray, F3::new(0.0, 0.3, 0.0), 1.3) {
            return true;
        }

        if let Some(_d) = self.plane(ray) {
            return true;
        }

        false
    }

    /// Returns the light at the given index
    fn light_at(&self, index: usize) -> &AnalyticalLight {
        &self.lights[index]
    }

    fn number_of_lights(&self) -> usize {
        self.lights.len()
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Analytical Intersections

impl AnalyticalIntersections for AnalyticalScene {
    // Based on https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-sphere-intersection
    fn sphere(&self, ray: &Ray, center: F3, radius: F) -> Option<F> {
        let l = center - ray.origin;
        let tca = l.dot(&ray.direction);
        let d2 = l.dot(&l) - tca * tca;
        let radius2 = radius * radius;
        if d2 > radius2 {
            return None;
        }
        let thc = (radius2 - d2).sqrt();
        let mut t0 = tca - thc;
        let mut t1 = tca + thc;

        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }

        if t0 < 0.0 {
            t0 = t1;
            if t0 < 0.0 {
                return None;
            }
        }

        Some(t0)
    }

    // Ray plane intersection
    fn plane(&self, ray: &Ray) -> Option<F> {
        let normal = F3::new(0.0, 1.0, 0.0);
        let denom = dot(&normal, &ray.direction);

        if denom.abs() > 0.0001 {
            let t = dot(&(F3::new(0.0, -1.0, 0.0) - ray.origin), &normal) / denom;
            if t >= 0.0 {
                return Some(t);
            }
        }
        None
    }
}

#[allow(unused)]
pub trait AnalyticalIntersections: Sync + Send {
    fn sphere(&self, ray: &Ray, center: F3, radius: F) -> Option<F>;
    fn plane(&self, ray: &Ray) -> Option<F>;
}

pub trait UpdateTrait: Sync + Send {
    fn set_material(&mut self, material: Material);
}

impl UpdateTrait for AnalyticalScene {
    fn set_material(&mut self, material: Material) {
        self.material = material;
    }
}
