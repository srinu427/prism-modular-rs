use std::collections::HashMap;

pub enum InputKeyState {
  Unknown,
  Pressed,
  Held,
  Released,
}

pub struct InputManager {
  keys_state_last: HashMap<winit::event::VirtualKeyCode, InputKeyState>,
  keys_state_now: HashMap<winit::event::VirtualKeyCode, InputKeyState>,
}

impl InputManager {
  pub fn new() -> Self {
    Self {
      keys_state_last: HashMap::with_capacity(256),
      keys_state_now: HashMap::with_capacity(256),
    }
  }

  pub fn process_event(&mut self, event: winit::event::DeviceEvent) {}
}
