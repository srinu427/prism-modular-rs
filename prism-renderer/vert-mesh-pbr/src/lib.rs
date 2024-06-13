use std::sync::Arc;
use vk_context::ash;
use vk_context::ash::vk;

mod structs;

pub fn make_vert_mesh_pbr_pipeline(
  device: Arc<ash::Device>,
  render_pass: vk::RenderPass,
) -> Result<(), String> {
  let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
    .render_pass(render_pass);
  Ok(())
}
