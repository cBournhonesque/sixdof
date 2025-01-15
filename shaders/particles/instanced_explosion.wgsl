#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::globals

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
    @location(4) i_color: vec4<f32>,
    @location(5) i_time_offset: f32,
    @location(6) rotation: vec4<f32>,
    @location(7) parent_instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) time_offset: f32,
};

@group(2) @binding(0) var myTexture: texture_2d<f32>;
@group(2) @binding(1) var mySampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let scaled_position = vertex.position * vertex.i_pos_scale.w;
    let transformation_matrix = quaternion_to_transformation_matrix(vertex.rotation, vertex.i_pos_scale.xyz);
    let transformed_position = transformation_matrix * vec4<f32>(scaled_position, 1.0);

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_model_matrix(vertex.parent_instance_index),
        transformed_position
    );
    out.uv = vertex.uv;
    out.color = vertex.i_color;
    out.time_offset = vertex.i_time_offset;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {

    let uv = in.uv;
    let time = globals.time + in.time_offset;

    let uv_smoke = vec2<f32>(uv.x + time * 1.0, uv.y + time * -0.1);
    let uv_smoke_final = vec2<f32>(
        uv_smoke.x + cos(uv_smoke.x * 2.0 + time * 0.1) * 0.1,
        uv_smoke.y + sin(uv_smoke.y * 2.0 + time * 0.1) * 0.1
    );

    let uv_thirded = in.uv * 3.0 - 1.5;
    let uv_core = vec2<f32>(
        uv_thirded.x + cos(uv_thirded.x * 2.0 + time * 5.2) * 0.1,
        uv_thirded.y + sin(uv_thirded.y * 2.0 + time * 5.2) * 0.1
    );
    let core = soft_circle(uv_core / 1.0, 0.5);

    let texture_color = textureSample(myTexture, mySampler, uv_smoke_final);
    let texture_as_alpha = (texture_color.r + texture_color.g + texture_color.b) / 3.0;

    let final_alpha = mix(
        vec4<f32>(0.0, 0.0, 0.0, 0.0),
        vec4<f32>(1.0, 1.0, 1.0, 1.0),
        core * texture_as_alpha
    );

    return vec4<f32>(final_alpha.rgb * in.color.rgb * in.color.a,  final_alpha.a * in.color.a);
}

fn rand(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn rotate(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    let x = uv.x * c - uv.y * s;
    let y = uv.x * s + uv.y * c;
    return vec2<f32>(x, y);
}

fn soft_circle(position: vec2<f32>, radius: f32) -> f32 {
    return 1.0 - smoothstep(0.0, radius, length(position));
}

// Function to convert a quaternion to a 4x4 transformation matrix, including translation
fn quaternion_to_transformation_matrix(q: vec4<f32>, translation: vec3<f32>) -> mat4x4<f32> {
    let qxx = q.x * q.x;
    let qyy = q.y * q.y;
    let qzz = q.z * q.z;
    let qxz = q.x * q.z;
    let qxy = q.x * q.y;
    let qyz = q.y * q.z;
    let qwx = q.w * q.x;
    let qwy = q.w * q.y;
    let qwz = q.w * q.z;

    // Construct the rotation part of the matrix
    let rotation = mat4x4<f32>(
        vec4<f32>(1.0 - 2.0 * (qyy + qzz), 2.0 * (qxy + qwz), 2.0 * (qxz - qwy), 0.0),
        vec4<f32>(2.0 * (qxy - qwz), 1.0 - 2.0 * (qxx + qzz), 2.0 * (qyz + qwx), 0.0),
        vec4<f32>(2.0 * (qxz + qwy), 2.0 * (qyz - qwx), 1.0 - 2.0 * (qxx + qyy), 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    // Add translation to the matrix
    var transformation = rotation;
    transformation[3][0] = translation.x;
    transformation[3][1] = translation.y;
    transformation[3][2] = translation.z;

    return transformation;
}