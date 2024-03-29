use std::mem::size_of;
pub use ash::vk;
pub use glam;

#[derive(Default, Clone, Copy)]
pub struct Vertex {
    pub position: glam::Vec4,
    pub normal: glam::Vec4,
    pub tangent: glam::Vec4,
    pub uv_coordinates: glam::Vec4,
}

impl Vertex{
    pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1]{
        [vk::VertexInputBindingDescription{
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX
        }]
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription;4]{
        [
            vk::VertexInputAttributeDescription{
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription{
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 1 * 4 * 4,
            },
            vk::VertexInputAttributeDescription{
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 2 * 4 * 4,
            },
            vk::VertexInputAttributeDescription{
                binding: 0,
                location: 3,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 3 * 4 * 4,
            },
        ]
    }
}

#[derive(Clone, Copy)]
pub struct TriangleFaceInfo {
    pub vertices: [u32; 3],
}

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<TriangleFaceInfo>,
}

impl Mesh {
    pub fn new_cube(x: f32, y: f32, z: f32) -> Self{
        let x_2 = x/2f32;
        let y_2 = y/2f32;
        let z_2 = z/2f32;
        Mesh{
            vertices: vec![
                // top face
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, -z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, -z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, z_2, 0f32, 0f32),
                },
                // bottom face
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, -z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, -z_2, 0f32, 0f32),
                },
                // front face
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, 1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, -y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, 1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, 1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, 1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, -y_2, 0f32, 0f32),
                },
                // back face
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, -1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, -1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(x_2, -y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, -1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, -y_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(0f32, 0f32, -1f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-x_2, y_2, 0f32, 0f32),
                },
                // right face
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(y_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-y_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-y_2, -z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(y_2, -z_2, 0f32, 0f32),
                },
                // left face
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, z_2, 1f32),
                    normal: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-y_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, z_2, 1f32),
                    normal: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(y_2, z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(y_2, -z_2, 0f32, 0f32),
                },
                Vertex{
                    position: glam::Vec4::new(-x_2, -y_2, -z_2, 1f32),
                    normal: glam::Vec4::new(-1f32, 0f32, 0f32, 0f32),
                    tangent: glam::Vec4::new(0f32, -1f32, 0f32, 0f32),
                    uv_coordinates: glam::Vec4::new(-y_2, -z_2, 0f32, 0f32),
                },
            ],
            faces: vec![
                TriangleFaceInfo{
                    vertices: [0, 1, 2],
                },
                TriangleFaceInfo{
                    vertices: [2, 3, 0],
                },
                TriangleFaceInfo{
                    vertices: [4, 5, 6],
                },
                TriangleFaceInfo{
                    vertices: [6, 7, 4],
                },
                TriangleFaceInfo{
                    vertices: [8, 9, 10],
                },
                TriangleFaceInfo{
                    vertices: [10, 11, 8],
                },
                TriangleFaceInfo{
                    vertices: [12, 13, 14],
                },
                TriangleFaceInfo{
                    vertices: [14, 15, 12],
                },
            ],
        }
    }

    pub fn get_draw_index_list(&self) -> Vec<u32> {
        self.faces
            .iter()
            .map(|x| x.vertices)
            .flatten()
            .collect::<Vec<u32>>()
    }
}
