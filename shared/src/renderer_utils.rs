//use crate::prelude::*;
// use theframework::prelude::*;

/*
pub fn ray_models(ray: &Ray, models: &(Vec<ModelFXFloor>, Vec<ModelFXWall>)) -> Option<Hit> {
    let mut hit: Option<Hit> = None;
    let (floors, walls) = models;
    for fx in floors {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    for fx in walls {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    hit
}
*/
