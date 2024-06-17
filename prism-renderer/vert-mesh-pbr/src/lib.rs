use std::sync::Arc;
use vk_context::ash;
use vk_context::ash::vk;

pub mod structs;

pub struct VertMeshPbrPipeline {
  set_layouts: Vec<vk::DescriptorSetLayout>,
  pipeline_layout: vk::PipelineLayout,
  pipeline: vk::Pipeline,
}

pub fn make_vert_mesh_pbr_pipeline(
  device: Arc<ash::Device>,
  render_pass: vk::RenderPass,
  subpass_idx: u32,
) -> Result<VertMeshPbrPipeline, String> {
  let descriptor_set_layout_0 = unsafe {
    device
      .create_descriptor_set_layout(
        &vk::DescriptorSetLayoutCreateInfo::default()
          .bindings(&[
            vk::DescriptorSetLayoutBinding::default()
              .stage_flags(vk::ShaderStageFlags::VERTEX)
              .binding(0)
              .descriptor_count(1)
              .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER),
          ]),
        None
      )
      .map_err(|e| format!("at descriptor set layout 0 create: {e}"))?
  };
  let descriptor_set_layout_1 = unsafe {
    device
      .create_descriptor_set_layout(
        &vk::DescriptorSetLayoutCreateInfo::default()
          .bindings(&[
            vk::DescriptorSetLayoutBinding::default()
              .stage_flags(vk::ShaderStageFlags::FRAGMENT)
              .binding(0)
              .descriptor_count(1)
              .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
          ]),
        None
      )
      .map_err(|e| format!("at descriptor set layout 1 create: {e}"))?
  };
  let pipeline_layout = unsafe {
    device
      .create_pipeline_layout(
        &vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&[descriptor_set_layout_0, descriptor_set_layout_1]),
        None
      )
      .map_err(|e| format!("at pipeline layout create: {e}"))?
  };
  let pipeline = unsafe {
    device
      .create_graphics_pipelines(
        vk::PipelineCache::null(),
        &[
          vk::GraphicsPipelineCreateInfo::default()
            .render_pass(render_pass)
            .subpass(subpass_idx)
            .layout(pipeline_layout)
            .stages(&[
              vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(c"main"),
              vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(c"main"),
            ])
            .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::default())
            .input_assembly_state(
              &vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            )
            .rasterization_state(
              &vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            )
            .color_blend_state(
              &vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op_enable(false)
            )
        ],
        None
      )
      .map_err(|e| format!("at creating pipeline: {}", e.1))?[0]
  };
  Ok(VertMeshPbrPipeline {
    set_layouts: vec![descriptor_set_layout_0, descriptor_set_layout_1],
    pipeline_layout,
    pipeline,
  })
}
