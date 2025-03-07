use bevy::{
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline}, 
    prelude::*, render::{mesh::MeshVertexBufferLayoutRef, render_resource::{AsBindGroup, AsBindGroupShaderType, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, CompareFunction, DepthBiasState, DepthStencilState, RenderPipelineDescriptor, Sampler, ShaderRef, ShaderStages, ShaderType, SpecializedMeshPipelineError, StencilState, TextureFormat, TextureSampleType, TextureViewDimension}, renderer::RenderDevice}};

const SHADER_ASSET_PATH: &str = "shaders/soft_particle_material.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct SoftParticleMaterialExtension {

    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    pub softness_factor: f32,
    #[uniform(100)]
    pub wave_amplitude: f32,
    #[uniform(100)]
    pub wave_frequency: f32,
    #[uniform(100)]
    pub time: f32,
}

impl Default for SoftParticleMaterialExtension {
    fn default() -> Self {
        Self {
            softness_factor: 6.0,
            wave_amplitude: 0.0,
            wave_frequency: 0.0,
            time: 0.0,
        }
    }
}

impl MaterialExtension for SoftParticleMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn specialize(
        pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Make sure the pipeline knows we use the depth texture
        descriptor.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Greater,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });
        Ok(())
    }
}
