//use crate::prelude::*;
//use theframework::prelude::*;

/// Gets the current time in milliseconds
pub fn get_time() -> u128 {
    let time;
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        time = t.as_millis();
    }
    #[cfg(target_arch = "wasm32")]
    {
        time = web_sys::window().unwrap().performance().unwrap().now() as u128;
    }
    time
}
