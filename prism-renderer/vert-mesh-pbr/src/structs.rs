use std::path::Path;
use std::sync::{Arc, Mutex};
use transfer_manager::TransferManager;
use vk_context::ash;
use vk_context::ash::vk;
use vk_context::gpu_allocator::vulkan::{Allocation, Allocator};
use vk_context::helpers::PWImage;

pub struct VertMesh {
  vert_buffer: vk::Buffer,
  idx_buffer: vk::Buffer,
  vb_alloc: Allocation,
  ib_alloc: Allocation,
}

pub struct PbrMaterial {
  image: PWImage,
  allocation: Option<Allocation>,
  allocator: Arc<Mutex<Allocator>>,
  device: Arc<ash::Device>,
}

impl PbrMaterial {
  pub fn new(
    transfer_manager: &TransferManager,
    allocator: Arc<Mutex<Allocator>>,
    name: &str,
    path: &Path,
  ) -> Result<Self, String> {
    let (image, allocation) = transfer_manager
      .load_image_from_file(Arc::clone(&allocator), path, name)
      .map_err(|e| format!("at image upload: {e}"))?;
    Ok(Self {
      image,
      allocation: Some(allocation),
      allocator,
      device: transfer_manager.get_arc_device(),
    })
  }
}

impl Drop for PbrMaterial {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_image(self.image.inner, None);
      if let Some(alloc_mem) = self.allocation.take() {
        let _ = self
          .allocator
          .lock()
          .inspect_err(|e| eprintln!("error acquiring allocator lock {e}"))
          .map(|mut x| x.free(alloc_mem));
      }
    }
  }
}
