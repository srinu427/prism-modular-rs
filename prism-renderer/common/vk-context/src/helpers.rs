use std::sync::Arc;
use ash::vk;
use crate::auto_drop_wrappers::ADRenderPass;

#[derive(Copy, Clone, Default)]
pub struct PWImage {
  pub inner: vk::Image,
  pub format: vk::Format,
  pub _type: vk::ImageType,
  pub resolution: vk::Extent3D,
}

pub struct ADRenderPassBuilder<'a> {
  device: Arc<ash::Device>,
  flags: vk::RenderPassCreateFlags,
  attachments: Vec<vk::AttachmentDescription>,
  sub_pass_descriptions: Vec<vk::SubpassDescription<'a>>,
  sub_pass_dependencies: Vec<vk::SubpassDependency>,
}

impl<'a> ADRenderPassBuilder<'a> {
  pub fn new(device: Arc<ash::Device>, flags: vk::RenderPassCreateFlags) -> Self {
    Self {
      device,
      flags,
      attachments: vec![],
      sub_pass_descriptions: vec![],
      sub_pass_dependencies: vec![],
    }
  }

  pub fn add_attachment(mut self, attachment_description: vk::AttachmentDescription) -> Self {
    self.attachments.push(attachment_description);
    self
  }

  pub fn add_sub_pass(mut self, sub_pass_description: vk::SubpassDescription<'a>) -> Self {
    self.sub_pass_descriptions.push(sub_pass_description);
    self
  }

  pub fn add_sub_pass_dependency(mut self, sub_pass_dependency: vk::SubpassDependency) -> Self {
    self.sub_pass_dependencies.push(sub_pass_dependency);
    self
  }

  pub fn build(self) -> Result<ADRenderPass, String> {
    let render_pass_create_info = vk::RenderPassCreateInfo::default()
      .flags(self.flags)
      .attachments(&self.attachments)
      .subpasses(&self.sub_pass_descriptions)
      .dependencies(&self.sub_pass_dependencies);
    let vk_render_pass = unsafe {
      self
        .device
        .create_render_pass(&render_pass_create_info, None)
        .map_err(|e| format!("at creating render pass: {e}"))?
    };
    Ok(ADRenderPass { device: self.device, inner: vk_render_pass })
  }
}
