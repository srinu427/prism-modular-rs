pub mod presentation;

use std::sync::Arc;
use vk_context::vk;
pub use vk_context::VkLoaders;

pub struct Renderer {
  vk_context: Arc<vk_context::VkContext>,
}

impl Renderer {
  pub fn new(vk_loaders: Arc<VkLoaders>, surface: vk::SurfaceKHR) -> Result<Self, String> {
    let vk_context = Arc::new(vk_context::VkContext::new(vk_loaders, surface, None)?);

    Ok(Self { vk_context })
  }
}
