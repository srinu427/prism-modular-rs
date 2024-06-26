use std::path::Path;
use std::sync::{Arc, Mutex};
use vk_context::gpu_allocator::vulkan::Allocator;
use vk_context::gpu_allocator::MemoryLocation;
use vk_context::{ash::vk, VkContext};
use vk_context::auto_drop_wrappers::{AdAllocatedBuffer, AdAllocatedImage, AdCommandPool};

pub struct TransferManager {
  cmd_pool: AdCommandPool,
  vk_context: Arc<VkContext>,
}

impl TransferManager {
  pub fn new(vk_context: Arc<VkContext>) -> Result<Self, String> {
    let cmd_pool = vk_context
        .create_ad_command_pool(
          vk::CommandPoolCreateInfo::default()
            .queue_family_index(vk_context.transfer_q_idx)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT),
        )?;

    Ok(Self { cmd_pool, vk_context })
  }

  pub fn load_image_from_file(
    &self,
    allocator: Arc<Mutex<Allocator>>,
    path: &Path,
    name: &str,
  ) -> Result<AdAllocatedImage, String> {
    let image_info = image::open(path).map_err(|e| format!("at loading image file: {e}"))?;
    let image_rgba8 = image_info.to_rgba8();

    let image = AdAllocatedImage::new(
      Arc::clone(&self.vk_context.device),
      Arc::clone(&allocator),
      name,
      vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .mip_levels(1)
        .array_layers(1)
        .extent(
          vk::Extent3D::default()
            .width(image_info.width())
            .height(image_info.height())
            .depth(1),
        ),
        MemoryLocation::GpuOnly
    )
      .map_err(|e| format!("at creating tex ad image: {e}"))?;

    let mut stage_buffer = AdAllocatedBuffer::new(
      Arc::clone(&self.vk_context.device),
      Arc::clone(&allocator),
      name,
      vk::BufferCreateInfo::default()
        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .size(image_rgba8.len() as vk::DeviceSize),
      MemoryLocation::CpuToGpu
    )
      .map_err(|e| format!("at creating staging buffer: {e}"))?;

    stage_buffer
      .allocation
      .as_mut()
      .ok_or("stage buffer not allocated, hmmm".to_string())?
      .mapped_slice_mut()
      .ok_or("at mapping stage buffer memory to CPU".to_string())?
      .copy_from_slice(image_rgba8.as_raw().as_slice());

    let cmd_buffer = self
      .cmd_pool
      .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)?
      .swap_remove(0);

    cmd_buffer.begin(vk::CommandBufferBeginInfo::default())?;
    cmd_buffer.pipeline_barrier(
      vk::PipelineStageFlags::BOTTOM_OF_PIPE,
      vk::PipelineStageFlags::TRANSFER,
      vk::DependencyFlags::BY_REGION,
      &[],
      &[],
      &[vk::ImageMemoryBarrier::default()
        .image(image.inner)
        .subresource_range(
          vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1),
        )
        .src_access_mask(vk::AccessFlags::NONE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(self.vk_context.transfer_q_idx)
        .dst_queue_family_index(self.vk_context.transfer_q_idx)],
    );
    cmd_buffer.copy_buffer_to_image(
      stage_buffer.inner,
      image.inner,
      vk::ImageLayout::TRANSFER_DST_OPTIMAL,
      &[vk::BufferImageCopy::default()
        .image_subresource(
          vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1),
        )
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(
          vk::Extent3D::default().width(image_info.width()).height(image_info.height()).depth(1),
        )],
    );
    cmd_buffer.pipeline_barrier(
      vk::PipelineStageFlags::TRANSFER,
      vk::PipelineStageFlags::TRANSFER,
      vk::DependencyFlags::BY_REGION,
      &[],
      &[],
      &[vk::ImageMemoryBarrier::default()
        .image(image.inner)
        .subresource_range(
          vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1),
        )
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .src_queue_family_index(self.vk_context.transfer_q_idx)
        .dst_queue_family_index(self.vk_context.transfer_q_idx)],
    );
    cmd_buffer.end()?;

    unsafe {
      let upload_fence = self.vk_context.create_ad_fence()?;

      self
        .vk_context
        .device
        .queue_submit(
          self.vk_context.transfer_q,
          &[vk::SubmitInfo::default().command_buffers(&[cmd_buffer.inner])],
          upload_fence.inner,
        )
        .map_err(|e| format!("at copying data to image: {e}"))?;

      self
        .vk_context
        .device
        .wait_for_fences(&[upload_fence.inner], true, u64::MAX)
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| format!("at waiting for fence: {e}"))?;
    }
    Ok(image)
  }
}

impl Drop for TransferManager {
  fn drop(&mut self) {}
}
