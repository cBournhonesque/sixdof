#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

#import bevy_render::view::View
@group(0) @binding(0) var<uniform> view: View;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let camera_pos = view.world_position;
    let pos = in.position;
    let world_normal = in.world_normal;
    let camera_dir = normalize(vec3<f32>(
        in.world_position.x - view.world_position.x,
        in.world_position.y - view.world_position.y,
        in.world_position.z - view.world_position.z
    ));

    let uv = in.world_position.xy / 0.5;

    let ripple_x = sin(uv.y * 1.5 + globals.time * 1.0) * 0.1;
    let ripple_y = cos(uv.x * 1.5 + globals.time * 1.0) * 0.1;
    let ripple_x_neg = sin(-uv.y * 2.5 + globals.time * 2.0) * 0.1;
    let ripple_y_neg = cos(-uv.x * 4.5 + globals.time * 2.0) * 0.1;
    let uv_rippled_fast = uv + vec2<f32>(ripple_x_neg, ripple_y) + vec2<f32>(ripple_x, ripple_y_neg);

    let uv_final = uv_rippled_fast;

    let base_color = vec4<f32>(0.2, 0.0, 0.8, 1.0);
    let base_color_dark = base_color * 0.5;

    let black = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    let black_transparent = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    let white = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    let ripples_mask = mix(
        black_transparent,
        white,
        ripple(uv_final, globals.time) * 0.3
    );

    let core_mask = mix(
        white,
        black_transparent,
        pow(1.650 - abs(dot(world_normal, camera_dir)), 1.25)
    );

    let fresnel = mix(
        black,
        base_color,
        pow(1.0 - abs(dot(world_normal, camera_dir)), 2.5)
    );

    let final_color = mix(
        fresnel,
        mix(
            black_transparent,
            base_color,
            ripples_mask + core_mask
        ) * 0.1,
        ripples_mask
    );

    return vec4<f32>(final_color.rgb * 8.0, final_color.a);
}

fn ripple(uv: vec2<f32>, time: f32) -> f32 {

    let size = 17.0;
    let dist_from_center = length(uv) * 1.25;
    return sin(dist_from_center * size - time) * 1.0 + 1.0;
}
