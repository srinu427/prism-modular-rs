use std::sync::Arc;
use ash::vk;

#[derive(Copy, Clone, Default)]
pub struct PWImage {
  pub inner: vk::Image,
  pub format: vk::Format,
  pub _type: vk::ImageType,
  pub resolution: vk::Extent3D,
}

pub struct CommandBufferRecorder {
  device: Arc<ash::Device>,
  inner: vk::CommandBuffer,
}

impl CommandBufferRecorder {
  pub fn new(device: Arc<ash::Device>, cmd_buffer: vk::CommandBuffer) -> Self {
    Self { device, inner: cmd_buffer }
  }

  pub fn begin(self, info: vk::CommandBufferBeginInfo) -> Result<Self, String> {
    unsafe {
      self.device.begin_command_buffer(self.inner, &info)
        .map_err(|e| format!("at cmd buffer begin: {e}"))?;
      Ok(self)
    }
  }

  pub fn end(self) -> Result<Self, String> {
    unsafe {
      self.device.end_command_buffer(self.inner)
        .map_err(|e| format!("at cmd buffer end: {e}"))?;
      Ok(self)
    }
  }

  pub fn begin_render_pass(
    self,
    render_pass_begin_info: vk::RenderPassBeginInfo,
    subpass_contents: vk::SubpassContents,
  ) -> Self {
    unsafe {
      self.device.cmd_begin_render_pass(self.inner, &render_pass_begin_info, subpass_contents);
      self
    }
  }

  pub fn end_render_pass(self) -> Self {
    unsafe {
      self.device.cmd_end_render_pass(self.inner);
      self
    }
  }

  pub fn bind_pipeline(
    self,
    pipeline_bind_point: vk::PipelineBindPoint,
    pipeline: vk::Pipeline
  ) -> Self {
    unsafe {
      self.device.cmd_bind_pipeline(self.inner, pipeline_bind_point, pipeline);
      self
    }
  }

  pub fn bind_vertex_buffer(
    self,
    binding_count: u32,
    buffers: Vec<vk::Buffer>,
    offsets: Vec<vk::DeviceSize>,
  ) -> Self {
    unsafe {
      self.device.cmd_bind_vertex_buffers(self.inner, binding_count, &buffers, &offsets);
      self
    }
  }

  pub fn bind_index_buffer(
    self,
    buffer: vk::Buffer,
    offset: vk::DeviceSize,
    index_type: vk::IndexType
  ) -> Self {
    unsafe {
      self.device.cmd_bind_index_buffer(self.inner, buffer, offset, index_type);
      self
    }
  }

  pub fn pipeline_barrier(
    self,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    dependency_flags: vk::DependencyFlags,
    memory_barriers: Vec<vk::MemoryBarrier>,
    buffer_memory_barriers: Vec<vk::BufferMemoryBarrier>,
    image_memory_barriers: Vec<vk::ImageMemoryBarrier>,
  ) -> Self {
    unsafe {
      self.device.cmd_pipeline_barrier(
        self.inner,
        src_stage,
        dst_stage,
        dependency_flags,
        &memory_barriers,
        &buffer_memory_barriers,
        &image_memory_barriers
      );
      self
    }
  }

  pub fn blit_image(
    self,
    src_image: vk::Image,
    src_image_layout: vk::ImageLayout,
    dst_image: vk::Image,
    dst_image_layout: vk::ImageLayout,
    regions: Vec<vk::ImageBlit>,
    filter: vk::Filter,
  ) -> Self {
    unsafe {
      self.device.cmd_blit_image(
        self.inner,
        src_image,
        src_image_layout,
        dst_image,
        dst_image_layout,
        &regions,
        filter
      );
      self
    }
  }
}
