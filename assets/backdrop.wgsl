// Pastel wash behind the floating vault panels, following Triwarden's
// WindowBackdrop (pink top-left, peach top-right, mint bottom-right,
// lavender bottom-left, a soft sky tail through the middle).
fn wash(base: vec3<f32>, p: vec2<f32>, center: vec2<f32>, radius: f32,
        tint: vec3<f32>, strength: f32) -> vec3<f32> {
    let d = distance(p, center) / max(radius, 1.0);
    let t = clamp(1.0 - d, 0.0, 1.0);
    return mix(base, tint, strength * t * t);
}

fn plico_effect(uv: vec2<f32>) -> vec4<f32> {
    let res = max(plico.resolution_time_scale.xy, vec2<f32>(1.0, 1.0));
    let p = uv * res;
    let span = max(res.x, res.y);
    var color = vec3<f32>(0.953, 0.953, 0.980);
    color = wash(color, p, vec2<f32>(0.08, 0.10) * res, span * 0.62,
                 vec3<f32>(0.97, 0.72, 0.79), 0.42);
    color = wash(color, p, vec2<f32>(0.92, 0.04) * res, span * 0.58,
                 vec3<f32>(1.00, 0.84, 0.60), 0.34);
    color = wash(color, p, vec2<f32>(0.74, 0.98) * res, span * 0.60,
                 vec3<f32>(0.56, 0.84, 0.78), 0.36);
    color = wash(color, p, vec2<f32>(0.22, 0.92) * res, span * 0.58,
                 vec3<f32>(0.73, 0.65, 0.95), 0.36);
    color = wash(color, p, vec2<f32>(0.55, 0.40) * res, span * 0.55,
                 vec3<f32>(0.80, 0.89, 0.96), 0.40);
    return vec4<f32>(color, 1.0);
}
