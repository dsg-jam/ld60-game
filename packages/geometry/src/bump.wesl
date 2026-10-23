/// Bump mapping without textures or tangents, from a height in meters,
/// after Mikkelsen, "Bump Mapping Unparametrized Surfaces on the GPU".

/// Tilts the normal `n` by how `height` changes across the screen at
/// `world`. Call it where control flow is uniform, as it takes derivatives.
fn bumped(n: vec3<f32>, world: vec3<f32>, height: f32) -> vec3<f32> {
    let sx = dpdx(world);
    let sy = dpdy(world);
    let r1 = cross(sy, n);
    let r2 = cross(n, sx);
    let det = dot(sx, r1);
    let grad = sign(det) * (dpdx(height) * r1 + dpdy(height) * r2);
    return normalize(abs(det) * n - grad);
}
