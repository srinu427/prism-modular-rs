use std::path::Path;
use std::sync::{Arc, Mutex};
use vk_context::gpu_allocator::vulkan::{
  Allocation, AllocationCreateDesc, AllocationScheme, Allocator,
};
use vk_context::gpu_allocator::MemoryLocation;
use vk_context::helpers::PWImage;
use vk_context::{ash, ash::vk, VkContext};

pub struct TransferManager {
  cmd_pool: vk::CommandPool,
  vk_context: Arc<VkContext>,
}

impl TransferManager {
  pub fn new(vk_context: Arc<VkContext>) -> Result<Self, String> {
    let cmd_pool = unsafe {
      vk_context
        .device
        .create_command_pool(
          &vk::CommandPoolCreateInfo::default()
            .queue_family_index(vk_context.transfer_q_idx)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT),
          None,
        )
        .map_err(|e| format!("at cmd pool create: {e}"))?
    };

    Ok(Self { cmd_pool, vk_context })
  }

  pub fn get_arc_device(&self) -> Arc<ash::Device> {
    Arc::clone(&self.vk_context.device)
  }

  pub fn load_image_from_file(
    &self,
    allocator: Arc<Mutex<Allocator>>,
    path: &Path,
    name: &str,
  ) -> Result<(PWImage, Allocation), String> {
    let image_info = image::open(path).map_err(|e| format!("at loading image file: {e}"))?;
    let image_rgba8 = image_info.to_rgba8();

    let (image, allocation) = unsafe {
      let image = self
        .vk_context
        .device
        .create_image(
          &vk::ImageCreateInfo::default()
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
          None,
        )
        .map_err(|e| format!("at creating tex image: {e}"))?;

      let tex_mem_req = self.vk_context.device.get_image_memory_requirements(image);

      let allocation = allocator
        .lock()
        .map_err(|e| format!("at getting allocator lock: {e}"))?
        .allocate(&AllocationCreateDesc {
          name,
          requirements: tex_mem_req,
          location: MemoryLocation::GpuOnly,
          linear: false,
          allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
        .map_err(|e| format!("at allocating mem: {e}"))?;

      self
        .vk_context
        .device
        .bind_image_memory(image, allocation.memory(), allocation.offset())
        .map_err(|e| format!("at tex image mem bind: {e}"))?;

      (image, allocation)
    };

    let (stage_buffer, mut stage_buffer_allocation) = unsafe {
      let stage_buffer = self
        .vk_context
        .device
        .create_buffer(
          &vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(image_rgba8.len() as vk::DeviceSize),
          None,
        )
        .map_err(|e| format!("at creating staging buffer: {e}"))?;
      let stage_buffer_mem_req =
        self.vk_context.device.get_buffer_memory_requirements(stage_buffer);

      let stage_buffer_allocation = allocator
        .lock()
        .map_err(|e| format!("at getting allocator lock: {e}"))?
        .allocate(&AllocationCreateDesc {
          name: &format!("{name}_stage_buffer"),
          requirements: stage_buffer_mem_req,
          location: MemoryLocation::CpuToGpu,
          linear: false,
          allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
        .map_err(|e| format!("at gpu mem alloc: {e}"))?;
      self
        .vk_context
        .device
        .bind_buffer_memory(
          stage_buffer,
          stage_buffer_allocation.memory(),
          stage_buffer_allocation.offset(),
        )
        .map_err(|e| format!("at stage buffer mem bind: {e}"))?;

      (stage_buffer, stage_buffer_allocation)
    };

    stage_buffer_allocation
      .mapped_slice_mut()
      .ok_or("at mapping stage buffer memory to CPU".to_string())?
      .copy_from_slice(image_rgba8.as_raw().as_slice());

    unsafe {
      let cmd_buffer = self
        .vk_context
        .device
        .allocate_command_buffers(
          &vk::CommandBufferAllocateInfo::default()
            .command_pool(self.cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1),
        )
        .map_err(|e| format!("at creating command buffer: {e}"))?[0];

      self
        .vk_context
        .device
        .begin_command_buffer(cmd_buffer, &vk::CommandBufferBeginInfo::default())
        .map_err(|e| format!("at starting command buffer: {e}"))?;

      self.vk_context.device.cmd_pipeline_barrier(
        cmd_buffer,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[vk::ImageMemoryBarrier::default()
          .image(image)
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
      self.vk_context.device.cmd_copy_buffer_to_image(
        cmd_buffer,
        stage_buffer,
        image,
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
      self.vk_context.device.cmd_pipeline_barrier(
        cmd_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[vk::ImageMemoryBarrier::default()
          .image(image)
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

      self
        .vk_context
        .device
        .end_command_buffer(cmd_buffer)
        .map_err(|e| format!("at ending command buffer: {e}"))?;

      let upload_fence = self
        .vk_context
        .device
        .create_fence(&vk::FenceCreateInfo::default(), None)
        .map_err(|e| format!("at creating fence: {e}"))?;

      self
        .vk_context
        .device
        .queue_submit(
          self.vk_context.transfer_q,
          &[vk::SubmitInfo::default().command_buffers(&[cmd_buffer])],
          upload_fence,
        )
        .map_err(|e| format!("at copying data to image: {e}"))?;

      self
        .vk_context
        .device
        .wait_for_fences(&[upload_fence], true, u64::MAX)
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| format!("at waiting for fence: {e}"))?;
      self.vk_context.device.destroy_fence(upload_fence, None);
      self.vk_context.device.destroy_buffer(stage_buffer, None);
    }

    allocator
      .lock()
      .map_err(|e| format!("at getting allocator lock: {e}"))?
      .free(stage_buffer_allocation)
      .map_err(|e| format!("at freeing stage buffer memory: {e}"))?;

    Ok((
      PWImage {
        inner: image,
        format: vk::Format::R8G8B8A8_UNORM,
        _type: vk::ImageType::TYPE_2D,
        resolution: vk::Extent3D {
          width: image_info.width(),
          height: image_info.height(),
          depth: 1,
        },
      },
      allocation,
    ))
  }
}

impl Drop for TransferManager {
  fn drop(&mut self) {
    unsafe {
      self.vk_context.device.destroy_command_pool(self.cmd_pool, None);
    }
  }
}
