use prism_renderer::{Mesh, Renderer, TriangleFaceInfo, Vertex};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{Window, WindowBuilder};

pub struct PrismApp {
    renderer: Renderer,
    window_event_loop: EventLoop<()>,
    window: Window,
}

impl PrismApp {
    pub fn new() -> Self {
        let window_event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Prism App")
            .with_inner_size(LogicalSize {
                width: 1920,
                height: 1080,
            })
            .build(&window_event_loop)
            .expect("Error initializing window");
        let mut renderer = Renderer::new(&window).expect("Error initializing Renderer");
        renderer.meshes.push(
            Mesh{
                vertices: vec![
                    Vertex{
                        position: glam::Vec4::new(-0.5f32, 0f32, 0f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(0.5f32, 0f32, 0f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(0f32, 0.5f32, 0f32, 1f32),
                        ..Default::default()
                    },
                ],
                faces: vec![
                    TriangleFaceInfo{
                        vertices: [0, 1, 2],
                    }
                ],
            }
        );
        renderer.meshes.push(
            Mesh{
                vertices: vec![
                    Vertex{
                        position: glam::Vec4::new(0.5f32, 0f32, 0f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(-0.5f32, -0.25f32, 0f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(0f32, -0.5f32, 0f32, 1f32),
                        ..Default::default()
                    },
                ],
                faces: vec![
                    TriangleFaceInfo{
                        vertices: [0, 1, 2],
                    }
                ],
            }
        );
        Self {
            window_event_loop,
            window,
            renderer,
        }
    }

    pub fn run(&mut self) {
        self.window_event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent {
                    event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    },
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::MainEventsCleared => {
                    match self.renderer.draw(&self.window) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("renderer draw error: {}", e);
                        }
                    };
                }
                _ => (),
            }
        });
    }
}
