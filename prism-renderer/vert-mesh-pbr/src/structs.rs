use std::path::Path;
use std::sync::{Arc, Mutex};
use transfer_manager::TransferManager;
use vk_context::auto_drop_wrappers::{AdAllocatedBuffer, AdAllocatedImage};
use vk_context::gpu_allocator::vulkan::Allocator;

pub struct VertMesh {
  vert_buffer: AdAllocatedBuffer,
  idx_buffer: AdAllocatedBuffer,
}

pub struct PbrMaterial {
  image: AdAllocatedImage,
}

impl PbrMaterial {
  pub fn new(
    transfer_manager: &TransferManager,
    allocator: Arc<Mutex<Allocator>>,
    name: &str,
    path: &Path,
  ) -> Result<Self, String> {
    let image = transfer_manager
      .load_image_from_file(Arc::clone(&allocator), path, name)
      .map_err(|e| format!("at image upload: {e}"))?;
    Ok(Self {
      image,
    })
  }
}

impl Drop for PbrMaterial {
  fn drop(&mut self) {}
}
