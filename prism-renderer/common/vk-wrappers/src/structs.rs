mod error_types;

use std::collections::HashMap;
use ash::vk;
use vk_mem::{Alloc, Allocation, AllocationCreateInfo, Allocator};
use std::sync::{Arc, Mutex};

pub use error_types::*;

pub struct SDCommandPool {
  device: Arc<ash::Device>,
  pub value: vk::CommandPool,
}

impl SDCommandPool {
  pub fn new(
    device: Arc<ash::Device>,
    cmd_pool_create_info: vk::CommandPoolCreateInfo,
  ) -> Result<Self, RendererError> {
    let cmd_pool = unsafe {
      device
        .create_command_pool(&cmd_pool_create_info, None)
        .map_err(|_| RendererError::CommandPool(CommandPoolError::CreateError))?
    };
    Ok(Self {
      device,
      value: cmd_pool,
    })
  }

  pub fn make_sd_command_buffers(
    &self,
    level: vk::CommandBufferLevel,
    count: u32,
  ) -> Result<Vec<SDCommandBuffer>, RendererError> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
      .command_pool(self.value)
      .level(level)
      .command_buffer_count(count);
    unsafe {
      Ok(
        self
          .device
          .allocate_command_buffers(&command_buffer_allocate_info)
          .map_err(|_| RendererError::CommandPool(CommandPoolError::BuffersAllocationError))?
          .drain(..)
          .map(|x| SDCommandBuffer::new(Arc::clone(&self.device), self.value, x))
          .collect(),
      )
    }
  }
}

impl Drop for SDCommandPool {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_command_pool(self.value, None);
    }
  }
}

pub struct SDSemaphore {
  device: Arc<ash::Device>,
  pub value: vk::Semaphore,
}

impl SDSemaphore {
  pub fn new(
    device: Arc<ash::Device>,
    semaphore_create_info: vk::SemaphoreCreateInfo,
  ) -> Result<Self, RendererError> {
    let semaphore = unsafe {
      device
        .create_semaphore(&semaphore_create_info, None)
        .map_err(|_| RendererError::Semaphore(SemaphoreError::CreateError))?
    };
    Ok(Self {
      device,
      value: semaphore,
    })
  }
}

impl Drop for SDSemaphore {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_semaphore(self.value, None);
    }
  }
}

pub struct SDFence {
  device: Arc<ash::Device>,
  pub value: vk::Fence,
}

impl SDFence {
  pub fn new(
    device: Arc<ash::Device>,
    fence_create_info: vk::FenceCreateInfo,
  ) -> Result<Self, RendererError> {
    let fence = unsafe {
      device
        .create_fence(&fence_create_info, None)
        .map_err(|_| RendererError::Fence(FenceError::CreateError))?
    };
    Ok(Self {
      device,
      value: fence,
    })
  }

  pub fn wait(&self, timeout: u64) -> Result<(), RendererError> {
    unsafe {
      self
        .device
        .wait_for_fences(&[self.value], true, timeout)
        .map_err(|_| RendererError::Fence(FenceError::WaitError))?
    }
    Ok(())
  }

  pub fn reset(&self) -> Result<(), RendererError> {
    unsafe {
      self
        .device
        .reset_fences(&[self.value])
        .map_err(|_| RendererError::Fence(FenceError::ResetError))?
    }
    Ok(())
  }
}

impl Drop for SDFence {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_fence(self.value, None);
    }
  }
}

struct AllocatedImage{
  image: vk::Image,
  allocation: Allocation,
  current_res: vk::Extent3D,
  format: vk::Format,
}

struct AllocatedBuffer{
  image: vk::Image,
  allocation: Allocation,
  current_res: vk::Extent3D,
  format: vk::Format,
}

pub struct SDAllocator{
  inner: Allocator,
  images: HashMap<String, AllocatedImage>,
  buffers: HashMap<String, AllocatedBuffer>,
}

pub struct SDImage {
  allocator: Option<Arc<Mutex<Allocator>>>,
  pub image: vk::Image,
  pub allocation: Option<Allocation>,
  pub current_res: vk::Extent3D,
  pub format: vk::Format,
}

impl SDImage {
  pub fn new(
    image_create_info: vk::ImageCreateInfo,
    allocator: Arc<Mutex<Allocator>>,
    memory_property_flags: vk::MemoryPropertyFlags,
  ) -> Result<Self, RendererError> {
    let malloc_info = AllocationCreateInfo::default()
      .required_flags(memory_property_flags);
    let (image, allocation) = unsafe {
      allocator
        .lock()
        .map_err(|_| RendererError::Allocation(AllocationError::LockError))?
        .create_image(&image_create_info, &malloc_info)
        .map_err(|_| RendererError::Allocation(AllocationError::AllocationFailed))?
    };
    Ok(Self {
      image,
      allocator: Some(allocator),
      allocation: Some(allocation),
      format: image_create_info.format,
      current_res: image_create_info.extent,
    })
  }
}

impl Drop for SDImage {
  fn drop(&mut self) {
    let allocator = std::mem::replace(&mut self.allocator, None);
    let allocation = std::mem::replace(&mut self.allocation, None);
    if let Some(allocator) = allocator {
      if let Some(mut allocation) = allocation {
        if let Ok(allocator) = allocator.lock() {
          unsafe{
            let _ = allocator.destroy_image(self.image, &mut allocation);
          }
        }
      };
    };
  }
}

pub struct SDImageView {
  device: Arc<ash::Device>,
  pub value: vk::ImageView,
}

impl SDImageView {
  pub fn new(
    device: Arc<ash::Device>,
    image_view_create_info: vk::ImageViewCreateInfo,
  ) -> Result<Self, RendererError> {
    let image_view = unsafe {
      device
        .create_image_view(&image_view_create_info, None)
        .map_err(|_| RendererError::ImageView(ImageViewError::CreateError))?
    };

    Ok(Self {
      device,
      value: image_view,
    })
  }
}

impl Drop for SDImageView {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_image_view(self.value, None);
    }
  }
}

pub struct SDFrameBuffer {
  device: Arc<ash::Device>,
  pub value: vk::Framebuffer,
}

impl SDFrameBuffer {
  pub fn new(
    device: Arc<ash::Device>,
    frame_buffer_create_info: vk::FramebufferCreateInfo,
  ) -> Result<Self, RendererError> {
    let frame_buffer = unsafe {
      device
        .create_framebuffer(&frame_buffer_create_info, None)
        .map_err(|_| RendererError::FrameBuffer(FrameBufferError::CreateError))?
    };

    Ok(Self {
      device,
      value: frame_buffer,
    })
  }
}

impl Drop for SDFrameBuffer {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_framebuffer(self.value, None);
    }
  }
}

pub struct SDBuffer {
  allocator: Option<Arc<Mutex<Allocator>>>,
  pub buffer: vk::Buffer,
  pub allocation: Option<Allocation>,
  pub current_size: u64,
}

impl SDBuffer {
  pub fn new(
    buffer_create_info: vk::BufferCreateInfo,
    allocator: Arc<Mutex<Allocator>>,
    memory_property_flags: vk::MemoryPropertyFlags,
  ) -> Result<Self, RendererError> {
    let malloc_info = AllocationCreateInfo::default()
      .required_flags(memory_property_flags);
    let (buffer, allocation) = unsafe{
      allocator
        .lock()
        .map_err(|_| RendererError::Allocation(AllocationError::LockError))?
        .create_buffer(&buffer_create_info, &malloc_info)
        .map_err(|_| RendererError::Allocation(AllocationError::AllocationFailed))?
    };

    Ok(Self {
      allocator: Some(allocator),
      allocation: Some(allocation),
      buffer,
      current_size: buffer_create_info.size,
    })
  }
}

impl Drop for SDBuffer {
  fn drop(&mut self) {
    let allocator = std::mem::replace(&mut self.allocator, None);
    let allocation = std::mem::replace(&mut self.allocation, None);
    if let Some(allocator) = allocator {
      if let Some(mut allocation) = allocation {
        if let Ok(allocator) = allocator.lock() {
          unsafe{
            let _ = allocator.destroy_buffer(self.buffer, &mut allocation);
          }
        }
      };
    };
  }
}

pub struct SDDescriptorSet {
  device: Arc<ash::Device>,
  descriptor_pool: vk::DescriptorPool,
  pub value: vk::DescriptorSet,
}

impl SDDescriptorSet {
  pub fn new(
    device: Arc<ash::Device>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
  ) -> Self {
    Self {
      device,
      descriptor_pool,
      value: descriptor_set,
    }
  }

  pub fn write_image(
    &self,
    view: vk::ImageView,
    sampler: vk::Sampler,
    layout: vk::ImageLayout
  ){
    unsafe{
      self.device.update_descriptor_sets(
        &[
          vk::WriteDescriptorSet::default()
            .dst_set(self.value)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_count(1)
            .image_info(
              &[
                vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(view)
                .image_layout(layout)
              ]
            )
        ],
        &[]
      );
    }
  }
}

impl Drop for SDDescriptorSet {
  fn drop(&mut self) {
    unsafe {
      let _ = self
        .device
        .free_descriptor_sets(self.descriptor_pool, &[self.value]);
    }
  }
}

pub struct PipelinePack {
  pub pipeline: vk::Pipeline,
  pub pipeline_layout: vk::PipelineLayout,
  pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
}

pub struct PerFrameGraphicsPassResources {
  pub attachments: Vec<SDImage>,
  pub attachment_image_views: Vec<SDImageView>,
  pub frame_buffer: SDFrameBuffer,
  pub uniform_buffers: Vec<SDBuffer>,
  pub descriptor_sets: Vec<SDDescriptorSet>,
}

pub struct  SDRenderPass {
  device: Arc<ash::Device>,
  pub render_pass: vk::RenderPass,
  pub pipeline_packs: Vec<PipelinePack>,
  pub per_frame_resources: Vec<PerFrameGraphicsPassResources>,
}

impl SDRenderPass {
  pub fn new(
    device: Arc<ash::Device>,
    render_pass: vk::RenderPass,
    pipeline_packs: Vec<PipelinePack>,
    per_frame_resources: Vec<PerFrameGraphicsPassResources>,
  ) -> Self {
    Self {
      device,
      render_pass,
      pipeline_packs,
      per_frame_resources,
    }
  }
}

impl Drop for SDRenderPass {
  fn drop(&mut self) {
    unsafe {
      for pipeline_pack in self.pipeline_packs.drain(..) {
        for dsl in pipeline_pack.descriptor_set_layouts{
          self
            .device
            .destroy_descriptor_set_layout(dsl, None);
        }
        self
          .device
          .destroy_pipeline_layout(pipeline_pack.pipeline_layout, None);
        self.device.destroy_pipeline(pipeline_pack.pipeline, None);
      }
      self.device.destroy_render_pass(self.render_pass, None)
    };
  }
}

pub struct SDCommandBuffer {
  device: Arc<ash::Device>,
  command_pool: vk::CommandPool,
  pub value: vk::CommandBuffer,
}

impl SDCommandBuffer {
  pub fn new(
    device: Arc<ash::Device>,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
  ) -> Self {
    Self {
      device,
      command_pool,
      value: command_buffer,
    }
  }
}

impl Drop for SDCommandBuffer {
  fn drop(&mut self) {
    unsafe {
      self
        .device
        .free_command_buffers(self.command_pool, &[self.value]);
    }
  }
}

pub struct SDDescriptorPool {
  device: Arc<ash::Device>,
  pub value: vk::DescriptorPool,
}

impl SDDescriptorPool {
  pub fn new(
    device: Arc<ash::Device>,
    descriptor_pool_create_info: vk::DescriptorPoolCreateInfo,
  ) -> Result<Self, RendererError> {
    let descriptor_pool = unsafe {
      device
        .create_descriptor_pool(&descriptor_pool_create_info, None)
        .map_err(|_| RendererError::DescriptorPool(DescriptorPoolError::CreateError))?
    };
    Ok(Self {
      device,
      value: descriptor_pool,
    })
  }

  pub fn make_sd_descriptor_sets(
    &self,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
  ) -> Result<Vec<SDDescriptorSet>, RendererError> {
    let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
      .descriptor_pool(self.value)
      .set_layouts(descriptor_set_layouts.as_slice());
    unsafe {
      Ok(
        self
          .device
          .allocate_descriptor_sets(&descriptor_set_allocate_info)
          .map_err(|_| RendererError::DescriptorPool(DescriptorPoolError::SetsAllocationError))?
          .drain(..)
          .map(|x| SDDescriptorSet::new(Arc::clone(&self.device), self.value, x))
          .collect(),
      )
    }
  }
}

impl Drop for SDDescriptorPool {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_descriptor_pool(self.value, None);
    }
  }
}
