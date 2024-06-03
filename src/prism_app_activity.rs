mod window_manager;

use window_manager::WindowManager;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{WindowAttributes, WindowId};

pub struct PrismAppActivity {
  window_manager: Option<WindowManager>,
}

impl PrismAppActivity {
  pub fn new() -> Result<Self, String> {
    Ok(Self { window_manager: None })
  }
}

impl ApplicationHandler for PrismAppActivity {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    match event_loop.create_window(WindowAttributes::default()) {
      Ok(w) => {
        if let Some(mut wm) = self.window_manager.take() {
          if let Err(e) = wm.refresh_surface() {
            eprintln!("can't refresh surface: {e}");
            event_loop.exit()
          }
        } else {
          match WindowManager::new(w) {
            Ok(wm) => {
              self.window_manager = Some(wm);
            }
            Err(e) => {
              eprintln!("can't start window mgr: {e}");
              event_loop.exit()
            }
          }
        }
      }
      Err(e) => {
        eprintln!("error creating window: {e}");
        event_loop.exit()
      }
    }
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::ActivationTokenDone { .. } => {}
      WindowEvent::Resized(_) => {}
      WindowEvent::Moved(_) => {}
      WindowEvent::CloseRequested => event_loop.exit(),
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

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(wm) = self.window_manager.as_mut() {
      wm.redraw()
    }
  }
}
