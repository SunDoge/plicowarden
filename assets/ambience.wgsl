fn wash(base: vec3<f32>, color: vec3<f32>, uv: vec2<f32>, center: vec2<f32>, strength: f32) -> vec3<f32> {
    let distance = length((uv - center) * vec2<f32>(1.15, 1.0));
    let amount = strength * (1.0 - smoothstep(0.0, 0.85, distance));
    return mix(base, color, amount);
}
fn plico_effect(uv: vec2<f32>) -> vec4<f32> {
    var color = vec3<f32>(0.95, 0.95, 0.97);
    color = wash(color, vec3<f32>(0.55, 0.76, 0.93), uv, vec2<f32>(0.03, 0.05), 0.60);
    color = wash(color, vec3<f32>(0.98, 0.78, 0.65), uv, vec2<f32>(1.0, 0.0), 0.36);
    color = wash(color, vec3<f32>(0.40, 0.71, 0.78), uv, vec2<f32>(0.15, 1.0), 0.55);
    color = wash(color, vec3<f32>(0.62, 0.84, 0.78), uv, vec2<f32>(0.90, 1.0), 0.30);
    color = wash(color, vec3<f32>(0.75, 0.84, 0.96), uv, vec2<f32>(0.56, 0.48), 0.30);
    // A quiet, sculpted ribbon gives the pane real spatial detail to refract.
    let wave = uv.x + sin(uv.y * 5.0 + 0.8) * 0.20;
    let ribbon = exp(-pow((wave - 0.05) * 7.0, 2.0));
    color = mix(color, vec3<f32>(0.38, 0.66, 0.79), ribbon * 0.23);
    return vec4<f32>(color, 1.0);
}
