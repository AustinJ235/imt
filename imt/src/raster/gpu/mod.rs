pub mod compute;
pub mod image_view;
pub mod shaders;

use std::sync::Arc;

use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::Queue;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::ComputePipeline;
use vulkano::shader::ShaderModule;

use crate::raster::gpu::shaders::*;

#[allow(dead_code)]
pub struct GpuRasterizer {
    queue: Arc<Queue>,
    mem_alloc: StandardMemoryAllocator,
    cmd_alloc: StandardCommandBufferAllocator,
    set_alloc: StandardDescriptorSetAllocator,
    nonzero_cs: Arc<ShaderModule>,
    downscale_cs: Arc<ShaderModule>,
    hinting_cs: Arc<ShaderModule>,
    nonzero_pipeline: Arc<ComputePipeline>,
    downscale_pipeline: Arc<ComputePipeline>,
    hinting_pipeline: Arc<ComputePipeline>,
}

impl GpuRasterizer {
    pub fn new(queue: Arc<Queue>) -> Self {
        let mem_alloc = StandardMemoryAllocator::new_default(queue.device().clone());
        let cmd_alloc =
            StandardCommandBufferAllocator::new(queue.device().clone(), Default::default());
        let set_alloc = StandardDescriptorSetAllocator::new(queue.device().clone());
        let nonzero_cs = nonzero_cs::load(queue.device().clone()).unwrap();
        let downscale_cs = downscale_cs::load(queue.device().clone()).unwrap();
        let hinting_cs = hinting_cs::load(queue.device().clone()).unwrap();

        let nonzero_pipeline = ComputePipeline::new(
            queue.device().clone(),
            nonzero_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        let downscale_pipeline = ComputePipeline::new(
            queue.device().clone(),
            downscale_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        let hinting_pipeline = ComputePipeline::new(
            queue.device().clone(),
            hinting_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        Self {
            queue,
            mem_alloc,
            cmd_alloc,
            set_alloc,
            nonzero_cs,
            downscale_cs,
            hinting_cs,
            nonzero_pipeline,
            downscale_pipeline,
            hinting_pipeline,
        }
    }
}
