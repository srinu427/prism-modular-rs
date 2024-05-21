use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use vk_wrappers::structs::SDCommandPool;
use vk_wrappers::{gpu_allocator, vk, VKManager};

struct Texture {
  image: vk::Image,
  view: vk::ImageView,
  sampler: vk::Sampler,
  allocation: gpu_allocator::vulkan::Allocation,
}

struct TextureAllocator {
  vk_manager: Arc<VKManager>,
  allocator: Arc<Mutex<gpu_allocator::vulkan::Allocator>>,
  command_queue: vk::Queue,
  command_pool: SDCommandPool,
  textures: HashMap<String, Arc<Texture>>,
}

impl TextureAllocator {
  pub fn new(vk_manager: Arc<VKManager>) -> Result<Self, String> {
    let allocator = vk_manager
      .make_gpu_allocator()
      .map_err(|e| format!("allocator creation failed: {e}"))?;
    let command_pool = SDCommandPool::new(
      Arc::clone(&vk_manager.device),
      vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(vk_manager.t_q_idx),
    )
    .map_err(|e| format!("command pool creation failed: {e}"))?;
    Ok(Self {
      vk_manager: Arc::clone(&vk_manager),
      allocator,
      command_queue: vk_manager.t_queue,
      command_pool,
      textures: HashMap::new(),
    })
  }

  pub fn load_texture_from_image_file(&self, name: &str, image_path: &Path) {
    let texture_upload_cmd_buffer = unsafe {
      self
        .vk_manager
        .device
        .allocate_command_buffers(
          &vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool.value)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1),
        )
        .map_err(|_| "Error cmd buffer to upload texture")?[0]
    };
    unsafe {
      self
        .vk_manager
        .device
        .begin_command_buffer(
          texture_upload_cmd_buffer,
          &vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )
        .map_err(|_| "Error starting texture upload command buffer")?;
    }

    let image_info =
      image::open(image_path).map_err(|e| format!("error loading image file: {e}"))?;
    let image_rgba8 = image_info.to_rgba8();
    let tex_image = unsafe {
      self
        .vk_manager
        .device
        .create_image(
          &vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(
              vk::Extent3D::default()
                .width(image_info.width())
                .height(image_info.height())
                .depth(1),
            )
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED),
          None,
        )
        .map_err(|e| format!("vk image create failed: {e}"))?
    };
    let tex_mem_req = unsafe {
      self
        .vk_manager
        .device
        .get_image_memory_requirements(tex_image)
    };
    let tex_mem_allocation = self
      .allocator
      .lock()
      .map_err(|e| format!("gpu mem alloc lock error: {e}"))?
      .allocate(&gpu_allocator::vulkan::AllocationCreateDesc{
        name,
        requirements: tex_mem_req,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
      })
      .map_err(|e| format!("gpu mem alloc error: {e}"))?;
    unsafe {
      self
        .vk_manager
        .device
        .bind_image_memory(tex_image, tex_mem_allocation.memory(), tex_mem_allocation.offset())
        .map_err(|e| format!("tex image mem bind error: {e}"))?;
    }

    let tex_stage_buffer = unsafe{
      self
        .vk_manager
        .device
        .create_buffer(
          &vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(image_rgba8.len() as vk::DeviceSize),
          None
        )
        .map_err(|e| format!("Error creating staging buffer: {e}"))?
    };
    let tex_stage_buffer_mem_req = unsafe {
      self
        .vk_manager
        .device
        .get_buffer_memory_requirements(tex_stage_buffer)
    };
    let mut tex_stage_buffer_mem_allocation = self
      .allocator
      .lock()
      .map_err(|e| format!("gpu mem alloc lock error: {e}"))?
      .allocate(&gpu_allocator::vulkan::AllocationCreateDesc{
        name,
        requirements: tex_stage_buffer_mem_req,
        location: gpu_allocator::MemoryLocation::CpuToGpu,
        linear: false,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
      })
      .map_err(|e| format!("gpu mem alloc error: {e}"))?;
    unsafe {
      self
        .vk_manager
        .device
        .bind_buffer_memory(
          tex_stage_buffer,
          tex_stage_buffer_mem_allocation.memory(),
          tex_stage_buffer_mem_allocation.offset()
        )
        .map_err(|e| format!("tex image mem bind error: {e}"))?;
    }

    tex_stage_buffer_mem_allocation
      .as_mut()
      .ok_or("Error accessing buffer memory")?
      .mapped_slice_mut()
      .ok_or("Error mapping stage buffer memory to CPU")?
      .copy_from_slice(image_rgba8.as_raw().as_slice());

    unsafe {
      vk_manager.device.cmd_pipeline_barrier(
        texture_upload_cmd_buffer,
        vk::PipelineStageFlags::NONE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[vk::ImageMemoryBarrier {
          src_access_mask: vk::AccessFlags::NONE,
          dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
          old_layout: vk::ImageLayout::UNDEFINED,
          new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
          src_queue_family_index: vk_manager.t_q_idx,
          dst_queue_family_index: vk_manager.t_q_idx,
          image: tex_image.image,
          subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
          },
          ..Default::default()
        }],
      );
      vk_manager.device.cmd_copy_buffer_to_image(
        texture_upload_cmd_buffer,
        tex_stage_buffer.buffer,
        tex_image.image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[vk::BufferImageCopy {
          buffer_offset: 0,
          buffer_row_length: 0,
          buffer_image_height: 0,
          image_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
          },
          image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
          image_extent: vk::Extent3D {
            width: tex_image.current_res.width,
            height: tex_image.current_res.height,
            depth: 1,
          },
        }],
      );
      vk_manager.device.cmd_pipeline_barrier(
        texture_upload_cmd_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[vk::ImageMemoryBarrier {
          src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
          dst_access_mask: vk::AccessFlags::SHADER_READ,
          old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
          new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
          src_queue_family_index: vk_manager.t_q_idx,
          dst_queue_family_index: vk_manager.t_q_idx,
          image: tex_image.image,
          subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
          },
          ..Default::default()
        }],
      );
    }
  }
}
