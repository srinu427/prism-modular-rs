use std::ffi::CString;
use std::mem;
use std::sync::{Arc, Mutex};
use camera_3d::Camera3D;
use vk_wrappers::gpu_allocator::MemoryLocation;
use vk_wrappers::gpu_allocator::vulkan::Allocator;
use vk_wrappers::vk;
use vk_wrappers::{GraphicsPassGenerator, VKManager};
use vk_wrappers::structs::*;
use mesh_structs::Vertex;

pub struct VertexPass {}

impl GraphicsPassGenerator for VertexPass {
    fn make_gpu_render_pass(
        vk_manager: &VKManager,
        image_format: vk::Format,
    ) -> Result<SDRenderPass, String> {
        let attachment_desc = vk::AttachmentDescription {
            format: image_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            final_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ..Default::default()
        };

        let attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass_desc = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &attachment_ref,
            ..Default::default()
        };

        let subpass_dependencies = [
            vk::SubpassDependency {
                src_subpass: vk::SUBPASS_EXTERNAL,
                dst_subpass: 0,
                src_stage_mask: vk::PipelineStageFlags::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: vk::AccessFlags::TRANSFER_READ,
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dependency_flags: Default::default(),
            },
            vk::SubpassDependency {
                src_subpass: 0,
                dst_subpass: vk::SUBPASS_EXTERNAL,
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_stage_mask: vk::PipelineStageFlags::TRANSFER,
                src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                dependency_flags: Default::default(),
            },
        ];

        let render_pass = unsafe {
            vk_manager
                .device
                .create_render_pass(
                    &vk::RenderPassCreateInfo {
                        attachment_count: 1,
                        p_attachments: &attachment_desc,
                        subpass_count: 1,
                        p_subpasses: &subpass_desc,
                        dependency_count: 2,
                        p_dependencies: subpass_dependencies.as_ptr(),
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|_| "Error creating triangle render pass")?
        };

        let main_c_str = CString::new("main").expect("c str creation error");
        let vert_shader = vk_manager.make_shader_from_spv(
            "prism-renderer/src/vertex_pass/shaders/gbuffer.vert.spv".into(),
        ).map_err(|_| "Error loading triangle vert shader")?;
        let frag_shader = vk_manager.make_shader_from_spv(
            "prism-renderer/src/vertex_pass/shaders/gbuffer.frag.spv".into(),
        ).map_err(|_| "Error loading triangle vert shader")?;
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfo {
                module: vert_shader,
                stage: vk::ShaderStageFlags::VERTEX,
                p_name: main_c_str.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: frag_shader,
                stage: vk::ShaderStageFlags::FRAGMENT,
                p_name: main_c_str.as_ptr(),
                ..Default::default()
            },
        ];

        let vertex_binding_descriptions = Vertex::get_binding_descriptions();
        let vertex_attribute_descriptions = Vertex::get_attribute_descriptions();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
            vertex_attribute_description_count: vertex_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: vertex_attribute_descriptions.as_ptr(),
            ..Default::default()
        };
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: 2,
            p_dynamic_states: [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR].as_ptr(),
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            line_width: 1f32,
            ..Default::default()
        };
        let multisample_info = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let color_blend_attachment_info = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::RGBA,
            blend_enable: vk::FALSE,
            ..Default::default()
        };
        let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: &color_blend_attachment_info,
            ..Default::default()
        };

        let descriptor_layout_bindings = vec![
            vk::DescriptorSetLayoutBinding{
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            }
        ];
        let descriptor_layout_info = vk::DescriptorSetLayoutCreateInfo{
            binding_count: descriptor_layout_bindings.len() as u32,
            p_bindings: descriptor_layout_bindings.as_ptr(),
            ..Default::default()
        };
        let descriptor_layout = unsafe{
            vk_manager
                .device
                .create_descriptor_set_layout(&descriptor_layout_info, None)
                .map_err(|_| "Error creating descriptor layout for vertex pipeline")?
        };

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: 1,
            p_set_layouts: &descriptor_layout,
            ..Default::default()
        };
        let pipeline_layout = unsafe {
            vk_manager
                .device
                .create_pipeline_layout(&pipeline_layout_info, None)
        }
            .map_err(|_| String::from("lol1"))?;

        let pipeline = unsafe {
            vk_manager
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::GraphicsPipelineCreateInfo {
                        stage_count: shader_stages.len() as u32,
                        p_stages: shader_stages.as_ptr(),
                        p_vertex_input_state: &vertex_input_info,
                        p_input_assembly_state: &input_assembly_info,
                        p_viewport_state: &viewport_state,
                        p_rasterization_state: &rasterizer_info,
                        p_multisample_state: &multisample_info,
                        p_color_blend_state: &color_blend_state_info,
                        layout: pipeline_layout,
                        p_dynamic_state: &dynamic_state_info,
                        render_pass,
                        ..Default::default()
                    }],
                    None,
                )
                .map_err(|_| String::from("lol"))?
        }[0];
        unsafe {
            vk_manager.device.destroy_shader_module(vert_shader, None);
            vk_manager.device.destroy_shader_module(frag_shader, None);
        }

        Ok(SDRenderPass::new(
            Arc::clone(&vk_manager.device),
            render_pass,
            vec![
                PipelinePack{
                    pipeline,
                    pipeline_layout,
                    descriptor_set_layout: descriptor_layout,
                }
            ],
            vec![],
        ))
    }

    fn create_per_frame_resources(
        vk_manager: &VKManager,
        graphics_pass: &mut SDRenderPass,
        allocator: Arc<Mutex<Allocator>>,
        resolution: vk::Extent2D,
        descriptor_pool: &SDDescriptorPool,
    ) -> Result<(), String> {
        let mut triangle_per_frame_resources = Vec::with_capacity(3);
        for i in 0..3 {
            let cam_buffer_name = format!("vertex_cam_buffer_{}", i);
            let cam_buffer = vk_manager.create_buffer(
                Arc::clone(&allocator),
                &cam_buffer_name,
                mem::size_of::<Camera3D>() as vk::DeviceSize,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                MemoryLocation::CpuToGpu
            ).map_err(|_| "Error creating uniform buffer for cam data")?;
            let img_name = format!("vertex_attachment_{}", i);
            let attachment = vk_manager.create_2d_image(
                Arc::clone(&allocator),
                &img_name,
                resolution,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC
            ).map_err(|_| "Error creating triangle attachment image")?;
            let attachment_view_info = vk::ImageViewCreateInfo {
                image: attachment.image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                components: vk::ComponentMapping::builder()
                    .r(vk::ComponentSwizzle::IDENTITY)
                    .g(vk::ComponentSwizzle::IDENTITY)
                    .b(vk::ComponentSwizzle::IDENTITY)
                    .a(vk::ComponentSwizzle::IDENTITY)
                    .build(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let attachment_view = SDImageView::new(
                Arc::clone(&vk_manager.device),
                attachment_view_info,
            )
                .map_err(|_| "Error creating image view for triangle render image")?;
            let frame_buffer_info = vk::FramebufferCreateInfo {
                render_pass: graphics_pass.render_pass,
                attachment_count: 1,
                p_attachments: &attachment_view.value,
                width: resolution.width,
                height: resolution.height,
                layers: 1,
                ..Default::default()
            };
            let frame_buffer = SDFrameBuffer::new(
                Arc::clone(&vk_manager.device),
                frame_buffer_info
            )
                .map_err(|_| "Error creating frame buffer for triangle render pipeline")?;

            let descriptor_sets = descriptor_pool.make_sd_descriptor_sets(
                vec![graphics_pass.pipeline_packs[0].descriptor_set_layout],
            )
                .map_err(|_| "Error creating descriptor sets")?;

            triangle_per_frame_resources.push(PerFrameGraphicsPassResources {
                attachments: vec![attachment],
                attachment_image_views: vec![attachment_view],
                frame_buffer,
                uniform_buffers: vec![cam_buffer],
                descriptor_sets,
            });
        }

        graphics_pass.per_frame_resources = triangle_per_frame_resources;
        Ok(())
    }

    fn add_init_per_frame_resources_commands(
        vk_manager: &VKManager,
        graphics_pass: &SDRenderPass,
        command_buffer: vk::CommandBuffer
    ) -> Result<(), String> {
        for i in 0..3 {
            let init_layout_barrier = vk::ImageMemoryBarrier {
                src_access_mask: vk::AccessFlags::NONE_KHR,
                dst_access_mask: vk::AccessFlags::MEMORY_READ,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                src_queue_family_index: vk_manager.g_q_idx,
                dst_queue_family_index: vk_manager.g_q_idx,
                image: graphics_pass.per_frame_resources[i].attachments[0].image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            unsafe {
                vk_manager.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::DependencyFlags::BY_REGION,
                    &[],
                    &[],
                    &[init_layout_barrier],
                );
            }
        }
        Ok(())
    }
}
