mod prism_app_activity;

use winit::event_loop::{ControlFlow, EventLoop};
use prism_app_activity::PrismAppActivity;

fn main() {
  let mut prism_app = PrismAppActivity::new();
  let window_event_loop = EventLoop::new().expect("Error initializing window event loop");
  window_event_loop.set_control_flow(ControlFlow::Poll);
  let window = window_event_loop.run_app(&mut prism_app);
}
