use std::sync::Arc;
use prism_renderer::Renderer;
use winit::window::Window;

pub struct WindowManager {
  renderer: Renderer,
  window: Arc<Window>,
}

impl WindowManager {
  pub fn new(window: Window) -> Result<Self, String> {
    let window_size = window.inner_size();
    let renderer = Renderer::new(&window, window_size.width, window_size.height)?;

    Ok(Self { window: Arc::new(window), renderer })
  }

  pub fn refresh_surface(&mut self) -> Result<(), String> {
    let window_size = self.window.inner_size();
    self.renderer.refresh_surface(&self.window, window_size.width, window_size.height)
  }

  pub fn update_resolution(&mut self) -> Result<(), String> {
    let window_size = self.window.inner_size();
    self.renderer.resize_swapchain(window_size.width, window_size.height)
  }

  pub fn redraw(&mut self) {
    let _ = self.renderer.draw()
      .map(|x| {
        if x {
          let _ = self.update_resolution().inspect_err(|e| println!("{e}"));
          self.redraw();
        }
      })
      .inspect_err(|e| println!("at redraw: {e}"));
  }
}
