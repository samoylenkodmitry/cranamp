@fragment
fn effect_fs(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(input_texture));
    let effect = u[62];
    let size = u[0].xy;
    let source_height = u[0].z;
    let depth = u[0].w;
    let local = (input.uv * texture_size - effect.xy) * size / max(effect.zw, vec2<f32>(1.0));
    let distance = local.y - source_height;
    if distance < 0.0 {
        return textureSampleLevel(input_texture, input_sampler, input.uv, 0.0);
    }
    let t = clamp(distance / max(depth, 0.001), 0.0, 1.0);
    let source_width = u[2].x;
    let spread = u[2].y;
    let skew = u[2].z;
    // Invert the floor's projective transform. The entire lower half is
    // mirrored into the plane, with a shared contact edge and a wider near edge.
    let width_scale = 1.0 + spread * t;
    let mirrored_x = (local.x - source_width * (0.5 + skew * t)) / width_scale
        + source_width * 0.5;
    if mirrored_x < 0.0 || mirrored_x > source_width {
        return vec4<f32>(u[1].rgb, 1.0);
    }
    let source_fraction = t * (1.0 + spread) / width_scale;
    let mirrored_y = source_height * (1.0 - source_fraction);
    let sample_px = effect.xy + vec2<f32>(mirrored_x, mirrored_y) * effect.zw / size;
    let safe_px = clamp(sample_px, effect.xy + 0.5, effect.xy + effect.zw - 0.5);
    let reflected = textureSampleLevel(input_texture, input_sampler, safe_px / texture_size, 0.0);
    let opacity = 0.42 * (1.0 - t) * (1.0 - t);
    let floor = mix(u[1].rgb, reflected.rgb, opacity);
    // A narrow light at the contact edge suggests a horizontal glass surface.
    let sheen = 0.035 * exp(-distance / 5.0);
    return vec4<f32>(mix(floor, vec3<f32>(1.0), sheen), 1.0);
}
