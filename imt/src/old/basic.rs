use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::buffer::cpu_access::CpuAccessibleBuffer;
use vulkano::buffer::{BufferUsage, TypedBufferAccess};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo,
    PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::Queue;
use vulkano::format::{ClearValue, Format};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::ImageUsage;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::depth_stencil::{
    CompareOp, DepthStencilState, StencilOp, StencilOpState, StencilOps, StencilState,
};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipeline;
use vulkano::pipeline::StateMode;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, Subpass};
use vulkano::sync::GpuFuture;
use vulkano::{impl_vertex, single_pass_renderpass};

use crate::parse::Font;
use crate::raster::gpu::image_view::ImtImageView;

#[inline]
fn neg_one_to_one(from: f32) -> f32 {
    (from * 2.0) - 1.0
}

pub struct BasicGlyphRender {
    pub width: u32,
    pub height: u32,
    pub bearing_y: i16,
    pub data: Vec<u8>,
}

pub fn basic_render(
    font: &Font,
    glyph_index: u16,
    size: f32,
    queue: Arc<Queue>,
) -> BasicGlyphRender {
    // Load Shaders

    let stencil_vs = stencil_vs::load(queue.device().clone()).unwrap();

    // Renderpasses & Pipelines

    let stencil_renderpass = single_pass_renderpass!(
        queue.device().clone(),
        attachments: {
            stencil: {
                load: Clear,
                store: Store,
                format: Format::S8_UINT,
                samples: 1,
            }
        },
        pass: {
            color: [],
            depth_stencil: { stencil }
        }
    )
    .unwrap();

    let stencil_pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<StencilVertex>())
        .vertex_shader(stencil_vs.entry_point("main").unwrap(), ())
        .input_assembly_state(
            InputAssemblyState::new().topology(PrimitiveTopology::TriangleList),
        )
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .render_pass(Subpass::from(stencil_renderpass.clone(), 0).unwrap())
        .depth_stencil_state(DepthStencilState {
            depth: None,
            depth_bounds: None,
            stencil: Some(StencilState {
                enable_dynamic: false,
                front: StencilOpState {
                    ops: StateMode::Fixed(StencilOps {
                        fail_op: StencilOp::Invert,
                        pass_op: StencilOp::Invert,
                        depth_fail_op: StencilOp::Keep,
                        compare_op: CompareOp::Always,
                    }),
                    ..Default::default()
                },
                back: StencilOpState {
                    ops: StateMode::Fixed(StencilOps {
                        fail_op: StencilOp::Invert,
                        pass_op: StencilOp::Invert,
                        depth_fail_op: StencilOp::Keep,
                        compare_op: CompareOp::Always,
                    }),
                    ..Default::default()
                },
            }),
        })
        .build(queue.device().clone())
        .unwrap();

    // Size / Bearings / Scalers

    let outline = font
        .glyf_table()
        .outlines
        .get(&glyph_index)
        .unwrap()
        .clone();

    let scaler = (1.0 / font.head_table().units_per_em as f32) * size;
    let width_f = (outline.x_max as f32 - outline.x_min as f32) * scaler;
    let width_r = width_f.ceil();
    let image_w = width_r as u32;
    let scaler_hori = ((width_r / width_f) * scaler) / width_r;

    let (image_h, bearing_y, scaler_above, scaler_below) = if outline.y_max <= 0 || outline.y_min >= 0 {
        // Everything above or below baseline
        let height_f = (outline.y_max as f32 - outline.y_min as f32) * scaler;
        let y_max_r = (outline.y_max as f32 * scaler).ceil();
        let y_min_r = (outline.y_min as f32 * scaler).ceil();
        let height_r = y_max_r - y_min_r;
        let image_h = height_r as u32;
        let scaler_vert = ((height_r / height_f) * scaler) / image_h as f32;
        (image_h, y_min_r as i16, scaler_vert, scaler_vert)
    } else {
        // Some above and below baseline
        let above_f = outline.y_max as f32 * scaler;
        let below_f = outline.y_min as f32 * scaler;
        let above_r = above_f.ceil();
        let below_r = below_f.ceil();
        let image_h = (above_r - below_r) as u32;
        let scaler_above = ((above_r / above_f) * scaler) / image_h as f32;
        let scaler_below = ((below_r / below_f) * scaler) / image_h as f32;
        (image_h, below_r as i16, scaler_above, scaler_below)
    };

    // Resources

    let mem_alloc = StandardMemoryAllocator::new_default(queue.device().clone());

    let mut vertexes = Vec::new();
    let x_min_f = outline.x_min as f32;
    let y_min_f = outline.y_min as f32;

    let trans_x = |x: f32| {
        neg_one_to_one((x - x_min_f) * scaler_hori)
    };

    let trans_y = |y: f32| {
        if y > 0.0 {
            -neg_one_to_one((y - y_min_f) * scaler_above)
        } else {
            -neg_one_to_one((y - y_min_f) * scaler_below)
        }
    };

    for segment in outline.segments() {
        vertexes.push(StencilVertex {
            position: [-1.5; 2],
        });

        vertexes.push(StencilVertex {
            position: [trans_x(segment.p1.x), trans_y(segment.p1.y)],
        });

        vertexes.push(StencilVertex {
            position: [trans_x(segment.p2.x), trans_y(segment.p2.y)],
        });
    }

    for curve in outline.curves() {
        for i in 0..16 {
            let [p1x, p1y] = curve.evaluate(i as f32 / 16.0);
            let [p2x, p2y] = curve.evaluate((i + 1) as f32 / 16.0);

            vertexes.push(StencilVertex {
                position: [-1.5; 2],
            });

            vertexes.push(StencilVertex {
                position: [trans_x(p1x), trans_y(p1y)],
            });

            vertexes.push(StencilVertex {
                position: [trans_x(p2x), trans_y(p2y)],
            });
        }
    }

    let stencil_vertex_buf = CpuAccessibleBuffer::from_iter(
        &mem_alloc,
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        false,
        vertexes,
    )
    .unwrap();

    let stencil_buffer = ImtImageView::from_attachment(
        AttachmentImage::with_usage(
            &mem_alloc,
            [image_w, image_h],
            Format::S8_UINT,
            ImageUsage {
                depth_stencil_attachment: true,
                sampled: true,
                transfer_src: true,
                ..ImageUsage::empty()
            },
        )
        .unwrap(),
    )
    .unwrap();

    // Framebuffers

    let stencil_framebuffer = Framebuffer::new(
        stencil_renderpass.clone(),
        FramebufferCreateInfo {
            attachments: vec![stencil_buffer.clone()],
            ..Default::default()
        },
    )
    .unwrap();

    let cmd_alloc = StandardCommandBufferAllocator::new(queue.device().clone(), Default::default());

    let mut cmd_buf = AutoCommandBufferBuilder::primary(
        &cmd_alloc,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    cmd_buf
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![
                    Some(ClearValue::Stencil(0)),
                ],
                ..RenderPassBeginInfo::framebuffer(stencil_framebuffer.clone())
            },
            SubpassContents::Inline,
        )
        .unwrap()
        .set_viewport(
            0,
            std::iter::once(Viewport {
                origin: [0.0; 2],
                dimensions: [image_w as f32, image_h as f32],
                depth_range: 0.0..1.0,
            }),
        )
        .bind_pipeline_graphics(stencil_pipeline.clone())
        .bind_vertex_buffers(0, stencil_vertex_buf.clone())
        .draw(stencil_vertex_buf.len() as u32, 1, 0, 0)
        .unwrap()
        .end_render_pass()
        .unwrap();

    cmd_buf
        .build()
        .unwrap()
        .execute(queue.clone())
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let stencil_out: Arc<CpuAccessibleBuffer<[u8]>> = unsafe {
        CpuAccessibleBuffer::uninitialized_array(
            &mem_alloc,
            (image_w * image_h) as _,
            BufferUsage {
                transfer_dst: true,
                ..BufferUsage::empty()
            },
            false,
        )
        .unwrap()
    };

    let mut cmd_buf = AutoCommandBufferBuilder::primary(
        &cmd_alloc,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    cmd_buf
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            stencil_buffer.clone(),
            stencil_out.clone(),
        ))
        .unwrap();

    cmd_buf
        .build()
        .unwrap()
        .execute(queue.clone())
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let mut data: Vec<u8> = Vec::with_capacity((image_w * image_h) as usize);

    for val in (stencil_out.read().unwrap()).iter() {
        data.push(*val);
    }

    BasicGlyphRender {
        width: image_w,
        height: image_h,
        bearing_y,
        data,
    }
}

mod stencil_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "
    }
}

#[derive(Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
struct StencilVertex {
    position: [f32; 2],
}

impl_vertex!(StencilVertex, position);
