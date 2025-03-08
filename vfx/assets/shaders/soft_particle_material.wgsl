#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    view_transformations::depth_ndc_to_view_z,
    prepass_utils::prepass_depth,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct SoftParticleMaterialExtension {
    softness_factor: f32,
    wave_amplitude: f32,
    wave_frequency: f32,
    time: f32,
}

@group(2) @binding(100)
var<uniform> extended_material: SoftParticleMaterialExtension;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var inMut = in;

    // Adds a wave effect to the particle
    if (extended_material.wave_amplitude > 0.0 && extended_material.wave_frequency > 0.0) {
        var wave_offset = sin(extended_material.wave_frequency * inMut.uv.x + extended_material.time) * extended_material.wave_amplitude;
        inMut.uv.y += wave_offset;
    }

    var pbr_input = pbr_input_from_standard_material(inMut, is_front);
    let colorCopy = pbr_input.material.base_color;

    #ifdef DEPTH_PREPASS
        let depth_scene = prepass_depth(inMut.position, 0u);
        let diff = abs(1.0 / inMut.position.z - 1.0 / depth_scene);
        let soft_factor = smoothstep(0.0, extended_material.softness_factor, diff);
        pbr_input.material.base_color.a *= soft_factor;
    #endif

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(inMut, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    out.color = vec4(colorCopy.rgb, out.color.a);
#endif

    return out;
}
