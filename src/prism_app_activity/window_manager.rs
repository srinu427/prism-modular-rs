use std::sync::Arc;
use prism_renderer::presentation::PresentManager;
use prism_renderer::Renderer;
use prism_renderer::VkLoaders;
use winit::window::Window;

pub struct WindowManager {
  window: Window,
  renderer: Renderer,
  present_manager: PresentManager,
}

impl WindowManager{
  pub fn new(vk_loader: Arc<VkLoaders>, window: Window) -> Result<Self, String>{
    let surface = vk_loader.make_surface(&window)?;

    Ok(Self{
      window,
    })
  }
}