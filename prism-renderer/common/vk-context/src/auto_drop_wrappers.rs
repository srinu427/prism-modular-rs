use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use std::sync::{Arc, Mutex};

pub struct AdSemaphore {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::Semaphore,
}

impl Drop for AdSemaphore {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_semaphore(self.inner, None);
    }
  }
}

pub struct AdFence {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::Fence,
}

impl Drop for AdFence {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_fence(self.inner, None);
    }
  }
}

pub struct AdCommandPool {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::CommandPool,
}

impl AdCommandPool {
  pub fn allocate_command_buffers(&self, level: vk::CommandBufferLevel, count: u32) -> Result<Vec<AdCommandBuffer>, String> {
    let cmd_buffers = unsafe {
      self
        .device
        .allocate_command_buffers(
          &vk::CommandBufferAllocateInfo::default()
            .command_pool(self.inner)
            .level(level)
            .command_buffer_count(count),
        )
    }
      .map_err(|e| format!("at creating command buffer: {e}"))?
      .iter()
      .map(|&x| AdCommandBuffer{ device: Arc::clone(&self.device), pool: self.inner, inner: x })
      .collect::<Vec<_>>();
    Ok(cmd_buffers)
  }
}

impl Drop for AdCommandPool {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_command_pool(self.inner, None);
    }
  }
}

pub struct AdCommandBuffer {
  pub(crate) device: Arc<ash::Device>,
  pool: vk::CommandPool,
  pub inner: vk::CommandBuffer,
}

impl AdCommandBuffer {
  pub fn begin(&self, info: vk::CommandBufferBeginInfo) -> Result<(), String> {
    unsafe {
      self.device.begin_command_buffer(self.inner, &info)
        .map_err(|e| format!("at cmd buffer begin: {e}"))?;
      Ok(())
    }
  }

  pub fn end(&self) -> Result<(), String> {
    unsafe {
      self.device.end_command_buffer(self.inner)
        .map_err(|e| format!("at cmd buffer end: {e}"))?;
      Ok(())
    }
  }

  pub fn begin_render_pass(
    &self,
    render_pass_begin_info: vk::RenderPassBeginInfo,
    subpass_contents: vk::SubpassContents,
  ) {
    unsafe {
      self.device.cmd_begin_render_pass(self.inner, &render_pass_begin_info, subpass_contents);
    }
  }

  pub fn end_render_pass(&self) {
    unsafe {
      self.device.cmd_end_render_pass(self.inner);
    }
  }

  pub fn bind_pipeline(
    &self,
    pipeline_bind_point: vk::PipelineBindPoint,
    pipeline: vk::Pipeline
  ) {
    unsafe {
      self.device.cmd_bind_pipeline(self.inner, pipeline_bind_point, pipeline);
    }
  }

  pub fn bind_vertex_buffer(
    &self,
    binding_count: u32,
    buffers: &[vk::Buffer],
    offsets: &[vk::DeviceSize],
  ) {
    unsafe {
      self.device.cmd_bind_vertex_buffers(self.inner, binding_count, buffers, offsets);
    }
  }

  pub fn bind_index_buffer(
    &self,
    buffer: vk::Buffer,
    offset: vk::DeviceSize,
    index_type: vk::IndexType
  ) {
    unsafe {
      self.device.cmd_bind_index_buffer(self.inner, buffer, offset, index_type);
    }
  }

  pub fn pipeline_barrier(
    &self,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    dependency_flags: vk::DependencyFlags,
    memory_barriers: &[vk::MemoryBarrier],
    buffer_memory_barriers: &[vk::BufferMemoryBarrier],
    image_memory_barriers: &[vk::ImageMemoryBarrier],
  ) {
    unsafe {
      self.device.cmd_pipeline_barrier(
        self.inner,
        src_stage,
        dst_stage,
        dependency_flags,
        memory_barriers,
        buffer_memory_barriers,
        image_memory_barriers
      );
    }
  }

  pub fn blit_image(
    &self,
    src_image: vk::Image,
    src_image_layout: vk::ImageLayout,
    dst_image: vk::Image,
    dst_image_layout: vk::ImageLayout,
    regions: &[vk::ImageBlit],
    filter: vk::Filter,
  ) {
    unsafe {
      self.device.cmd_blit_image(
        self.inner,
        src_image,
        src_image_layout,
        dst_image,
        dst_image_layout,
        regions,
        filter
      );
    }
  }

  pub fn copy_buffer_to_image(
    &self,
    src_buffer: vk::Buffer,
    dst_image: vk::Image,
    dst_image_layout: vk::ImageLayout,
    regions: &[vk::BufferImageCopy],
  ) {
    unsafe {
      self.device.cmd_copy_buffer_to_image(
        self.inner,
        src_buffer,
        dst_image,
        dst_image_layout,
        regions
      );
    }
  }
}

impl Drop for AdCommandBuffer {
  fn drop(&mut self) {
    unsafe {
      self.device.free_command_buffers(self.pool, &[self.inner]);
    }
  }
}

pub struct ADRenderPass {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::RenderPass,
}

impl Drop for ADRenderPass {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_render_pass(self.inner, None);
    }
  }
}

pub struct AdAllocatedImage {
  pub inner: vk::Image,
  pub format: vk::Format,
  pub _type: vk::ImageType,
  pub resolution: vk::Extent3D,
  pub name: String,
  pub(crate) device: Arc<ash::Device>,
  allocator: Arc<Mutex<Allocator>>,
  allocation: Option<Allocation>,
}

impl AdAllocatedImage {
  pub fn new(
    device: Arc<ash::Device>,
    allocator: Arc<Mutex<Allocator>>,
    name: &str,
    info: vk::ImageCreateInfo,
    mem_location: gpu_allocator::MemoryLocation,
  ) -> Result<Self, String> {
    unsafe {
      let image = device.create_image(&info, None).map_err(|e| format!("at vk image create: {e}"))?;
      let allocation = allocator
        .lock()
        .map_err(|e| format!("at getting allocator lock: {e}"))?
        .allocate(&AllocationCreateDesc {
          name,
          requirements: device.get_image_memory_requirements(image),
          location: mem_location,
          linear: false,
          allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
        .map_err(|e| format!("at allocating image mem: {e}"))?;
      device
        .bind_image_memory(image, allocation.memory(), allocation.offset())
        .map_err(|e| format!("at image mem bind: {e}"))?;
      Ok(Self {
        inner: image,
        format: info.format,
        _type: info.image_type,
        resolution: info.extent,
        name: name.to_string(),
        device,
        allocator,
        allocation: Some(allocation),
      })
    }
  }
}

impl Drop for AdAllocatedImage {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_image(self.inner, None);
    }
    let _ = self
    .allocator
    .lock()
    .map(|mut altr| self.allocation.take().map(|altn| altr.free(altn)))
    .inspect_err(|e| eprintln!("at getting allocator lock while image destroy: {e}"));
  }
}

pub struct AdAllocatedBuffer {
  pub inner: vk::Buffer,
  pub size: vk::DeviceSize,
  pub name: String,
  pub(crate) device: Arc<ash::Device>,
  allocator: Arc<Mutex<Allocator>>,
  pub allocation: Option<Allocation>,
}

impl AdAllocatedBuffer {
  pub fn new(
    device: Arc<ash::Device>,
    allocator: Arc<Mutex<Allocator>>,
    name: &str,
    info: vk::BufferCreateInfo,
    mem_location: gpu_allocator::MemoryLocation,
  ) -> Result<Self, String> {
    unsafe {
      let buffer = device.create_buffer(&info, None).map_err(|e| format!("at vk buffer create: {e}"))?;
      let allocation = allocator
        .lock()
        .map_err(|e| format!("at getting allocator lock: {e}"))?
        .allocate(&AllocationCreateDesc {
          name,
          requirements: device.get_buffer_memory_requirements(buffer),
          location: mem_location,
          linear: false,
          allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
        .map_err(|e| format!("at allocating buffer mem: {e}"))?;
      device
        .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
        .map_err(|e| format!("at buffer mem bind: {e}"))?;
      Ok(Self {
        inner: buffer,
        size: info.size,
        name: name.to_string(),
        device,
        allocator,
        allocation: Some(allocation),
      })
    }
  }
}

impl Drop for AdAllocatedBuffer {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_buffer(self.inner, None);
    }
    let _ = self
    .allocator
    .lock()
    .map(|mut altr| self.allocation.take().map(|altn| altr.free(altn)))
    .inspect_err(|e| eprintln!("at getting allocator lock while buffer destroy: {e}"));
  }
}
