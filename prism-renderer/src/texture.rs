use vk_context::gpu_allocator::MemoryLocation;
use vk_context::gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use vk_context::{vk, VkContext};

pub struct Texture {
  image: vk::Image,
  allocation: Allocation,
}

impl Texture {
  pub fn new(vk_context: &VkContext, allocator: &mut Allocator, name: &str) -> Result<Self, String> {
    let image_info =
      image::open("./tile_tex.png").map_err(|e| format!("error loading image file: {e}"))?;
    let image_rgba8 = image_info.to_rgba8();

    let image = unsafe {
      vk_context.device.create_image(
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
        None
      ).map_err(|e| format!("error creating tex image: {e}"))?
    };
    let tex_mem_req = unsafe {
      vk_context
        .device
        .get_image_memory_requirements(image)
    };

    let allocation = allocator.allocate(&AllocationCreateDesc{
      name,
      requirements: tex_mem_req,
      location: MemoryLocation::GpuOnly,
      linear: false,
      allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    }).map_err(|e| format!("error allocating mem: {e}"))?;

    unsafe {
      vk_context
        .device
        .bind_image_memory(image, allocation.memory(), allocation.offset())
        .map_err(|e| format!("tex image mem bind error: {e}"))?;
    }

    let tex_stage_buffer = unsafe{
      vk_context
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
      vk_context
        .device
        .get_buffer_memory_requirements(tex_stage_buffer)
    };
    let mut tex_stage_buffer_mem_allocation = allocator
      .allocate(&AllocationCreateDesc{
        name,
        requirements: tex_stage_buffer_mem_req,
        location: MemoryLocation::CpuToGpu,
        linear: false,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
      })
      .map_err(|e| format!("gpu mem alloc error: {e}"))?;
    unsafe {
      vk_context
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
      vk_context.device.create_command_pool(
        &vk::CommandPoolCreateInfo::default().queue_family_index(vk_context.transfer_q_idx),,
        None
      ).map_err(|e| format!("error creating cmd pool: {e}"))?;
      vk_context.device.cmd_copy_buffer_to_image(

      )
    }

    Ok(Self {
      image,
      allocation,
    })
  }
}
