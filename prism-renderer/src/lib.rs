pub mod presentation;

use presentation::PresentManager;
use std::sync::Arc;
pub use vk_context::vk;
pub use vk_context::VkLoaders;

pub struct Renderer {
  vk_context: Arc<vk_context::VkContext>,
}

impl Renderer {
  pub fn new(vk_loaders: Arc<VkLoaders>, surface: vk::SurfaceKHR) -> Result<Self, String> {
    let vk_context = Arc::new(vk_context::VkContext::new(vk_loaders, surface, None)?);

    Ok(Self { vk_context })
  }

  pub fn check_surface_support(&self, surface: vk::SurfaceKHR) -> Result<bool, String> {
    unsafe {
      self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_support(
          self.vk_context.gpu,
          self.vk_context.present_q_idx,
          surface,
        )
        .map_err(|e| format!("{e}"))
    }
  }

  pub fn make_presentation_manager(
    &self,
    surface: vk::SurfaceKHR,
    resolution: vk::Extent2D,
  ) -> Result<PresentManager, String> {
    PresentManager::new(Arc::clone(&self.vk_context), surface, resolution)
      .map_err(|e| format!("{e}"))
  }
}
