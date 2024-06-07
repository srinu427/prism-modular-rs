use vk_context::ash::vk;
use vk_context::gpu_allocator::vulkan::Allocation;

pub struct VertMesh {
  vert_buffer: vk::Buffer,
  idx_buffer: vk::Buffer,
  vb_alloc: Allocation,
  ib_alloc: Allocation,
}