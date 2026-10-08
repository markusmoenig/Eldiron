pub type V3 = [f32; 3];
pub type Q4 = [f32; 4];
pub fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn mul(a: V3, s: f32) -> V3 {
    a.map(|x| x * s)
}
pub fn dot(a: V3, b: V3) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn unit(a: V3) -> V3 {
    mul(a, 1.0 / dot(a, a).sqrt().max(1e-8))
}
pub fn qx(angle: f32) -> Q4 {
    [(angle * 0.5).sin(), 0.0, 0.0, (angle * 0.5).cos()]
}
pub fn qmul(a: Q4, b: Q4) -> Q4 {
    let v = add(
        add(mul([b[0], b[1], b[2]], a[3]), mul([a[0], a[1], a[2]], b[3])),
        cross([a[0], a[1], a[2]], [b[0], b[1], b[2]]),
    );
    [
        v[0],
        v[1],
        v[2],
        a[3] * b[3] - dot([a[0], a[1], a[2]], [b[0], b[1], b[2]]),
    ]
}
pub fn rotate(q: Q4, p: V3) -> V3 {
    let u = [q[0], q[1], q[2]];
    add(
        p,
        add(
            mul(cross(u, p), 2.0 * q[3]),
            mul(cross(u, cross(u, p)), 2.0),
        ),
    )
}

pub fn qy(angle: f32) -> Q4 {
    [0.0, (angle * 0.5).sin(), 0.0, (angle * 0.5).cos()]
}
pub fn conjugate(q: Q4) -> Q4 {
    [-q[0], -q[1], -q[2], q[3]]
}
/// Shortest rotation taking a downward limb to the requested direction.
pub fn swing_down(direction: V3) -> Q4 {
    let d = unit(direction);
    let q = [-d[2], 0.0, d[0], 1.0 - d[1]];
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    if length < 1e-6 {
        qx(std::f32::consts::PI)
    } else {
        q.map(|v| v / length)
    }
}
