#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

#import bevy_render::view::View
@group(0) @binding(0) 
var<uniform> view: View;

@group(2) @binding(100)
var<uniform> color: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {

    let white = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    let black = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    let black_transparent = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    let uv = (in.uv * 2.0) - 1.0;
    let time = globals.time;
    let orbit_position = vec2<f32>(0.0, 0.0);

    let new_pos = vec2<f32>(
        sin(time * 0.5) * 0.5,
        cos(time * 0.5) * 0.25
    );

    let particle = mix(
        black_transparent,
        white,
        circle(uv, new_pos.xy, 0.01, 0.02)
    );

    let inner_core = circle(uv, vec2<f32>(0.0, 0.0), 0.01, 0.2);
    let outer_core = circle_soft(uv, vec2<f32>(0.0, 0.0), 0.02, 0.45);
    let dark_outer_core = circle(uv, vec2<f32>(0.0, 0.0), 0.2, 0.35);

    let ripple_x = sin(uv.x * 3.0 + time * 2.5) * 0.5;
    let ripple_y = cos(uv.y * 3.0 + time * 2.5) * 0.5;
    let ripple_x_neg = sin(-uv.x * 4.0 + time * 3.5) * 0.5;
    let ripple_y_neg = cos(-uv.y * 3.5 + time * 4.5) * 0.5;
    let uv_rippled = uv + vec2<f32>(ripple_x, ripple_y) + vec2<f32>(ripple_x_neg, ripple_y_neg);

    let ripples = ripple(uv_rippled, 2.5);
    let plasma = (mix(black_transparent, white, outer_core) * 0.85) * ripples;

    let final_alpha = (mix(
        black_transparent,
        white,
        (dark_outer_core + plasma + inner_core)
    ));

    let final_color = mix(
        black_transparent,
        black,
        dark_outer_core
    ) + mix(
        black_transparent,
        white,
        (inner_core + (outer_core * 0.02) + (plasma * 0.05))
    );

    return vec4<f32>((final_color * color).rgb * color.a, final_alpha.a * color.a);
}

fn circle(uv: vec2<f32>, center: vec2<f32>, radius: f32, softness: f32) -> f32 {
    let d = length(uv - center);
    return 1.0 - smoothstep(radius - softness, radius + softness, d);
}

fn circle_soft(uv: vec2<f32>, center: vec2<f32>, radius: f32, softness: f32) -> f32 {
    let d = length(uv - center);
    return 1.0 - smoothstep(radius - softness, softness, d);
}

fn ripple(uv: vec2<f32>, time: f32) -> f32 {
    let size = 2.5;
    let dist_from_center = length(uv) * 1.1;
    return cos(dist_from_center * size - time) * 0.2 + 0.2;
}
