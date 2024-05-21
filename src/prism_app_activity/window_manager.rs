use std::sync::Arc;
use winit::window::Window;
use prism_renderer::presentation::PresentManager;
use prism_renderer::VkLoaders;
use prism_renderer::{vk, Renderer};

pub struct WindowManager {
  window: Window,
  renderer: Renderer,
  present_manager: PresentManager,
}

impl WindowManager {
  pub fn new(vk_loaders: Arc<VkLoaders>, window: Window) -> Result<Self, String> {
    let surface = vk_loaders.make_surface(&window)?;
    let renderer = Renderer::new(Arc::clone(&vk_loaders), surface)?;
    let window_size = window.inner_size();
    let present_manager = renderer.make_presentation_manager(
      surface,
      vk::Extent2D::default().width(window_size.width).height(window_size.height),
    )?;

    Ok(Self { window, renderer, present_manager })
  }
}
