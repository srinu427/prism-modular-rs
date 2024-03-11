use ash::vk;
use gpu_allocator::vulkan::Allocation;

pub struct GPUImage {
    pub image: vk::Image,
    pub allocation: Allocation,
    pub current_res: vk::Extent2D,
}

pub struct GPUBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    pub current_size: u64,
}

pub struct PerFrameGraphicsPassResources {
    pub attachments: Vec<GPUImage>,
    pub attachment_image_views: Vec<vk::ImageView>,
    pub frame_buffer: vk::Framebuffer,
}

pub struct GraphicsPass {
    pub raw: vk::RenderPass,
    pub pipelines: Vec<(vk::PipelineLayout, vk::Pipeline)>,
    pub per_frame_resources: Vec<PerFrameGraphicsPassResources>,
}

impl GraphicsPass {}