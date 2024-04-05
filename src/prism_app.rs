use std::sync::Arc;
use prism_renderer::{Camera3D, Mesh, Renderer, TriangleFaceInfo, Vertex};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard;
use winit::window::{Window, WindowBuilder};

pub struct PrismApp {
    camera: Camera3D,
    renderer: Renderer,
    window: Arc<Window>,
    window_event_loop: EventLoop<()>,
}

impl PrismApp {
    pub fn new() -> Self {
        let window_event_loop = EventLoop::new()
            .expect("Error initializing window event loop");
        window_event_loop.set_control_flow(ControlFlow::Poll);
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Prism App")
                .with_inner_size(LogicalSize {
                    width: 1920,
                    height: 1080,
                })
                .build(&window_event_loop)
                .expect("Error initializing window")
        );
        let mut renderer = Renderer::new(Arc::clone(&window)).expect("Error initializing Renderer");
        renderer.meshes.push(
            Mesh{
                vertices: vec![
                    Vertex{
                        position: glam::Vec4::new(-0.5f32, 0f32, -1f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(0.5f32, 0f32, -1f32, 1f32),
                        ..Default::default()
                    },
                    Vertex{
                        position: glam::Vec4::new(0f32, 0.5f32, -1f32, 1f32),
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
        let camera = Camera3D {
            eye: glam::Vec4::new(1f32, 1f32, 1f32, 1f32),
            dir: glam::Vec4::new(-1f32, -1f32, -1f32, 0f32),
            up: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
            info: glam::Vec4::new(
                0.1f32, 10f32, 120f32 * (std::f32::consts::PI / 180f32), 16f32 / 9f32
            ),
        };
        Self {
            window_event_loop,
            window,
            renderer,
            camera,
        }
    }

    pub fn run(mut self){
        self.window_event_loop.run(
            |event, elw|{
                match event {
                    Event::WindowEvent {
                        event:
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                logical_key: keyboard::Key::Named(keyboard::NamedKey::Escape),
                                ..
                            },
                            ..
                        },
                        ..
                    } => {
                        elw.exit()
                    }
                    Event::WindowEvent {
                        event: WindowEvent::RedrawRequested,
                        ..
                    } => {
                        self.renderer.set_camera(self.camera);
                        self.renderer.draw()
                            .map_err(|e| println!("renderer draw error: {}", e))?
                    }
                    Event::AboutToWait => {
                        self.renderer.set_camera(self.camera);
                        self.renderer.draw()
                            .map_err(|e| println!("renderer draw error: {}", e))?
                    }
                    _ => (),
                }
            }
        ).expect("ERROR running window")
    }
}
