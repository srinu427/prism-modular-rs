use mesh_structs::Mesh;
use vk_wrappers::structs::{GPUBuffer, GPUImage};

pub struct RenderObject{
    mesh: Mesh,
    textures: Vec<GPUImage>,
    vertex_buffers: Vec<GPUBuffer>,
    index_buffers: Vec<GPUBuffer>,
}
