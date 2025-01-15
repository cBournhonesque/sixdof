#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

@group(2) @binding(100)
var<uniform> fresnel_strength: f32;

@group(2) @binding(101)
var<uniform> fresnel_color: vec3<f32>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply fresnel
    let F0 = vec3<f32>(0.1, 0.1, 0.1);
    let ct = dot(normalize(pbr_input.V), normalize(pbr_input.N));
    let fresnel = F0 + (vec3<f32>(1.0, 1.0, 1.0) - F0) * pow(1.0 - max(ct, 0.0), 3.0);
    let color_with_fresnel = out.color.rgb + (fresnel_strength * out.color.rgb * 2.0 - out.color.rgb) * fresnel;
    out.color = vec4<f32>(color_with_fresnel, out.color.a);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    return out;
}
