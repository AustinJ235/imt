pub mod compute;
pub mod image_view;
pub mod shaders;

use std::sync::Arc;

use vulkano::buffer::{BufferUsage, DeviceLocalBuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
    PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::Queue;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::ComputePipeline;
use vulkano::shader::ShaderModule;
use vulkano::sync::GpuFuture;

use crate::raster::gpu::compute::{raster, GpuRasteredGlyph};
use crate::raster::gpu::shaders::*;
use crate::raster::ScaledGlyph;

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
    nonzero_raydata: Arc<DeviceLocalBuffer<[[f32; 2]]>>,
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

        // TODO: Set local size here
        let nonzero_pipeline = ComputePipeline::new(
            queue.device().clone(),
            nonzero_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        // TODO: Set local size here
        let downscale_pipeline = ComputePipeline::new(
            queue.device().clone(),
            downscale_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        // TODO: Set local size here
        let hinting_pipeline = ComputePipeline::new(
            queue.device().clone(),
            hinting_cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        let ray_data: Vec<[f32; 2]> = [
            45.0_f32.to_radians(),
            135.0_f32.to_radians(),
            //225.0_f32.to_radians(),
            //315.0_f32.to_radians(),
        ]
        .into_iter()
        .map(|a| [a.cos(), a.sin()])
        .collect();

        let mut tx_cmd_b = AutoCommandBufferBuilder::primary(
            &cmd_alloc,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let nonzero_raydata = DeviceLocalBuffer::from_iter(
            &mem_alloc,
            ray_data,
            BufferUsage {
                storage_buffer: true,
                ..BufferUsage::empty()
            },
            &mut tx_cmd_b,
        )
        .unwrap();

        tx_cmd_b
            .build()
            .unwrap()
            .execute(queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
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
            nonzero_raydata,
        }
    }

    pub fn process(&self, glyphs: &[ScaledGlyph]) -> Vec<GpuRasteredGlyph> {
        let mut previous = None;
        let mut output = Vec::with_capacity(glyphs.len());

        for glyph in glyphs.iter() {
            let (rastered, future) = raster(
                &glyph,
                self,
                previous.take().map(
                    |v: CommandBufferExecFuture<Box<dyn GpuFuture + Send + Sync>>| {
                        v.boxed_send_sync()
                    },
                ),
            );

            previous = Some(future);
            output.push(rastered);
        }

        if let Some(future) = previous.take() {
            future
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();
        }

        output
    }
}
