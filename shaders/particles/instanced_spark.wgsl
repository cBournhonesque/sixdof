#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) pos_scale: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) time_offset: f32,
    @location(6) rotation: vec4<f32>,
    @location(7) parent_instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) time_offset: f32,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let scaled_position = vertex.position * vertex.pos_scale.w;
    let transformation_matrix = quaternion_to_transformation_matrix(vertex.rotation, vertex.pos_scale.xyz);
    let transformed_position = transformation_matrix * vec4<f32>(scaled_position, 1.0);

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_model_matrix(vertex.parent_instance_index),
        transformed_position
    );
    out.uv = vertex.uv;
    out.color = vertex.color;
    out.time_offset = vertex.time_offset;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5) - in.uv;
    let alpha = soft_circle(center, 0.5);
    return vec4<f32>(in.color.rgb * in.color.a, alpha * in.color.a);
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

