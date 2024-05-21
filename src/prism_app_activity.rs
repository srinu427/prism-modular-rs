mod window_manager;

use prism_renderer::VkLoaders;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{WindowAttributes, WindowId};

pub struct PrismAppActivity {
  vk_loaders: Arc<VkLoaders>,
  window_manager: Option<window_manager::WindowManager>,
}

impl PrismAppActivity {
  pub fn new() -> Result<Self, String> {
    let vk_loaders = Arc::new(VkLoaders::new()?);
    Ok(Self { vk_loaders, window_manager: None })
  }
}

impl ApplicationHandler for PrismAppActivity {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    match event_loop.create_window(WindowAttributes::default()) {
      Ok(w) => {
        if self.window_manager.is_none() {
          match window_manager::WindowManager::new(Arc::clone(&self.vk_loaders), w) {
            Ok(wm) => {
              self.window_manager = Some(wm);
            }
            Err(e) => {
              println!("can't start window mgr: {e}");
              event_loop.exit()
            }
          }
        } else { 
          
        }
      }
      Err(e) => {
        println!("error creating window: {e}");
        event_loop.exit()
      }
    }
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::ActivationTokenDone { .. } => {}
      WindowEvent::Resized(_) => {}
      WindowEvent::Moved(_) => {}
      WindowEvent::CloseRequested => {}
      WindowEvent::Destroyed => {}
      WindowEvent::DroppedFile(_) => {}
      WindowEvent::HoveredFile(_) => {}
      WindowEvent::HoveredFileCancelled => {}
      WindowEvent::Focused(_) => {}
      WindowEvent::KeyboardInput { .. } => {}
      WindowEvent::ModifiersChanged(_) => {}
      WindowEvent::Ime(_) => {}
      WindowEvent::CursorMoved { .. } => {}
      WindowEvent::CursorEntered { .. } => {}
      WindowEvent::CursorLeft { .. } => {}
      WindowEvent::MouseWheel { .. } => {}
      WindowEvent::MouseInput { .. } => {}
      WindowEvent::PinchGesture { .. } => {}
      WindowEvent::PanGesture { .. } => {}
      WindowEvent::DoubleTapGesture { .. } => {}
      WindowEvent::RotationGesture { .. } => {}
      WindowEvent::TouchpadPressure { .. } => {}
      WindowEvent::AxisMotion { .. } => {}
      WindowEvent::Touch(_) => {}
      WindowEvent::ScaleFactorChanged { .. } => {}
      WindowEvent::ThemeChanged(_) => {}
      WindowEvent::Occluded(_) => {}
      WindowEvent::RedrawRequested => {}
    }
  }
}
