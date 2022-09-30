use core_shared::prelude::*;
use rand::{thread_rng, Rng};

/*
#[derive(PartialEq, Clone, Debug)]
pub struct TileLighting {
    pub fixed                   : f32,
    pub dynamic                 : f32,
}*/

pub fn compute_lighting(_region: &GameRegionData, lights: &Vec<LightData>) -> FxHashMap<(isize, isize), f64> {
    let mut map : FxHashMap<(isize, isize), f64> = HashMap::default();

    let mut rng = thread_rng();

    for l in lights {
        map.insert(l.position.clone(), 1.0);

        if l.intensity > 0 {
            let mut tl = (l.position.0 - 1, l.position.1 - 1);
            let mut length = 3;

            let mut d = 1;

            let mut random : f64 = rng.gen();
            random -= 0.5;
            random *= 0.2;

            while d <= 3 { //l.intensity {

                let i = 1.0 / (d*2) as f64 + random / d as f64;
                for x in tl.0..tl.0 + length {
                    if let Some(value) = map.get_mut(&(x, tl.1)) {
                        *value += i;
                    } else {
                        map.insert((x, tl.1), i);
                    }

                    if let Some(value) = map.get_mut(&(x, tl.1 + length - 1)) {
                        *value += i;
                    } else {
                        map.insert((x, tl.1 + length - 1), i);
                    }
                }

                for y in tl.1+1..tl.1 + length - 1 {
                    if let Some(value) = map.get_mut(&(tl.0, y)) {
                        *value += i;
                    } else {
                        map.insert((tl.0, y), i);
                    }

                    if let Some(value) = map.get_mut(&(tl.0 + length - 1, y)) {
                        *value += i;
                    } else {
                        map.insert((tl.0 + length - 1, y), i);
                    }
                }

                d += 1;
                length += 2;
                tl.0 -= 1;
                tl.1 -= 1;
            }
        }
    }

    map
}