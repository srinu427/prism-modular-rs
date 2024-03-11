use std::mem::size_of;
use ash::vk;

#[derive(Default, Clone, Copy)]
pub struct Vertex {
    pub position: glam::Vec4,
    pub normal: glam::Vec4,
    pub tangent: glam::Vec4,
    pub bi_tangent: glam::Vec4,
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

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription;5]{
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
            vk::VertexInputAttributeDescription{
                binding: 0,
                location: 4,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 4 * 4 * 4,
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
    pub fn get_draw_index_list(&self) -> Vec<u32> {
        self.faces
            .iter()
            .map(|x| x.vertices)
            .flatten()
            .collect::<Vec<u32>>()
    }
}
