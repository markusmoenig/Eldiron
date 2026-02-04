// Simple procedural noise shader demonstrating viewport_rect usage

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;

    // Get viewport rect from uniforms
    let viewport = U.viewport_rect; // [x, y, width, height]

    // If viewport is set (width > 0), use it; otherwise use full framebuffer
    let use_viewport = viewport.z > 0.0;
    let screen_x = select(f32(px), viewport.x + f32(px), use_viewport);
    let screen_y = select(f32(py), viewport.y + f32(py), use_viewport);

    // Bounds check
    if (use_viewport && (f32(px) >= viewport.z || f32(py) >= viewport.w)) {
        return;
    }

    // Simple value noise using hash function
    let scale = 0.05;
    let x = screen_x * scale;
    let y = screen_y * scale;

    // Integer coordinates
    let ix = i32(floor(x));
    let iy = i32(floor(y));

    // Fractional part
    let fx = fract(x);
    let fy = fract(y);

    // Smooth interpolation (smoothstep)
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    // Hash function for pseudo-random values
    let h00 = hash2d(ix, iy);
    let h10 = hash2d(ix + 1, iy);
    let h01 = hash2d(ix, iy + 1);
    let h11 = hash2d(ix + 1, iy + 1);

    // Bilinear interpolation
    let n0 = mix(h00, h10, ux);
    let n1 = mix(h01, h11, ux);
    let noise = mix(n0, n1, uy);

    // Add some color variation based on position and time
    let time_factor = f32(U.anim_counter) * 0.01;
    let r = noise;
    let g = fract(noise + 0.333 + time_factor);
    let b = fract(noise + 0.667 - time_factor);

    // Use gp0 to control noise parameters if needed
    let brightness = U.gp0.x; // Default 0.0, can be set via SetGP0
    let color = vec4<f32>(
        r + brightness,
        g + brightness,
        b + brightness,
        1.0
    );

    // Write to output
    let out_x = i32(screen_x);
    let out_y = i32(screen_y);
    textureStore(color_out, vec2<i32>(out_x, out_y), color);
}

// Simple hash function for 2D coordinates
fn hash2d(x: i32, y: i32) -> f32 {
    var n = x * 374761393 + y * 668265263;
    n = (n ^ (n >> 13)) * 1274126177;
    n = n ^ (n >> 16);
    return f32(n & 0xFFFFFF) / 16777216.0;
}
