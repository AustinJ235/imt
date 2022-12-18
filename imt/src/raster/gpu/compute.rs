use std::sync::Arc;

use vulkano::buffer::{BufferUsage, DeviceLocalBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::{ImageCreateFlags, ImageDimensions, ImageUsage, StorageImage};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::sync::GpuFuture;

use crate::parse::OutlineGeometry;
use crate::raster::gpu::image_view::ImtImageView;
use crate::raster::gpu::shaders::nonzero_cs;
use crate::raster::gpu::GpuRasterizer;
use crate::raster::ScaledGlyph;

#[derive(Debug, Clone)]
pub struct GpuRasteredGlyph {
    pub width: u32,
    pub height: u32,
    pub bearing_x: i16,
    pub bearing_y: i16,
    pub advance_w: i16,
    pub bitmap: Arc<ImtImageView>,
    pub unique_id: u64,
}

pub fn raster(glyph: &ScaledGlyph, rasterizer: &GpuRasterizer) -> GpuRasteredGlyph {
    let outline = glyph.outline.as_ref().unwrap();
    let mut segment_data: Vec<[f32; 4]> = Vec::new();

    for geometry in outline.geometry.iter() {
        if let OutlineGeometry::Segment {
            p1,
            p2,
        } = geometry
        {
            segment_data.push([p1.x, p1.y, p2.x, p2.y]);
        } else {
            for i in 0..16 {
                let p1 = geometry.evaluate(i as f32 / 16.0);
                let p2 = geometry.evaluate((i + 1) as f32 / 16.0);
                segment_data.push([p1.x, p1.y, p2.x, p2.y]);
            }
        }
    }

    let ray_data: Vec<[f32; 2]> = [
        45.0_f32.to_radians(),
        135.0_f32.to_radians(),
        225.0_f32.to_radians(),
        315.0_f32.to_radians(),
    ]
    .into_iter()
    .map(|a| [a.cos(), a.sin()])
    .collect();

    let nonzero_info = nonzero_cs::ty::Info {
        extent: [glyph.width as f32 * 12.0, glyph.height as f32 * 4.0],
        numSegments: segment_data.len() as _,
        numRays: ray_data.len() as _,
    };

    let mut tx_cmd_b = AutoCommandBufferBuilder::primary(
        &rasterizer.cmd_alloc,
        rasterizer.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    let nonzero_raydata = DeviceLocalBuffer::from_iter(
        &rasterizer.mem_alloc,
        ray_data,
        BufferUsage {
            storage_buffer: true,
            ..BufferUsage::empty()
        },
        &mut tx_cmd_b,
    )
    .unwrap();

    let nonzero_segdata = DeviceLocalBuffer::from_iter(
        &rasterizer.mem_alloc,
        segment_data,
        BufferUsage {
            storage_buffer: true,
            ..BufferUsage::empty()
        },
        &mut tx_cmd_b,
    )
    .unwrap();

    let tx_cmd = tx_cmd_b
        .build()
        .unwrap()
        .execute(rasterizer.queue.clone())
        .unwrap()
        .then_signal_semaphore_and_flush()
        .unwrap();

    let nonzero_image = ImtImageView::from_storage(
        StorageImage::with_usage(
            &rasterizer.mem_alloc,
            ImageDimensions::Dim2d {
                width: glyph.width * 12,
                height: glyph.height * 4,
                array_layers: 1,
            },
            Format::R8_UNORM,
            ImageUsage {
                storage: true,
                ..ImageUsage::empty()
            },
            ImageCreateFlags::empty(),
            [rasterizer.queue.queue_family_index()],
        )
        .unwrap(),
    )
    .unwrap();

    let downscale_image = ImtImageView::from_storage(
        StorageImage::with_usage(
            &rasterizer.mem_alloc,
            ImageDimensions::Dim2d {
                width: glyph.width * 3,
                height: glyph.height * 1,
                array_layers: 1,
            },
            Format::R8_UNORM,
            ImageUsage {
                storage: true,
                ..ImageUsage::empty()
            },
            ImageCreateFlags::empty(),
            [rasterizer.queue.queue_family_index()],
        )
        .unwrap(),
    )
    .unwrap();

    let hinting_image = ImtImageView::from_storage(
        StorageImage::with_usage(
            &rasterizer.mem_alloc,
            ImageDimensions::Dim2d {
                width: glyph.width,
                height: glyph.height,
                array_layers: 1,
            },
            Format::R8G8B8A8_UNORM,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::empty()
            },
            ImageCreateFlags::empty(),
            [rasterizer.queue.queue_family_index()],
        )
        .unwrap(),
    )
    .unwrap();

    let nonzero_desc_set = PersistentDescriptorSet::new(
        &rasterizer.set_alloc,
        rasterizer
            .nonzero_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone(),
        [
            WriteDescriptorSet::buffer(0, nonzero_raydata.clone()),
            WriteDescriptorSet::buffer(1, nonzero_segdata.clone()),
            WriteDescriptorSet::image_view(2, nonzero_image.clone()),
        ],
    )
    .unwrap();

    let downscale_desc_set = PersistentDescriptorSet::new(
        &rasterizer.set_alloc,
        rasterizer
            .downscale_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone(),
        [
            WriteDescriptorSet::image_view(0, nonzero_image.clone()),
            WriteDescriptorSet::image_view(1, downscale_image.clone()),
        ],
    )
    .unwrap();

    let hinting_desc_set = PersistentDescriptorSet::new(
        &rasterizer.set_alloc,
        rasterizer
            .hinting_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone(),
        [
            WriteDescriptorSet::image_view(0, downscale_image.clone()),
            WriteDescriptorSet::image_view(1, hinting_image.clone()),
        ],
    )
    .unwrap();

    let mut cmd_buf = AutoCommandBufferBuilder::primary(
        &rasterizer.cmd_alloc,
        rasterizer.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    cmd_buf
        .bind_pipeline_compute(rasterizer.nonzero_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            rasterizer.nonzero_pipeline.layout().clone(),
            0,
            nonzero_desc_set,
        )
        .push_constants(
            rasterizer.nonzero_pipeline.layout().clone(),
            0,
            nonzero_info,
        )
        .dispatch([glyph.width * 12, glyph.height * 4, 1])
        .unwrap()
        .bind_pipeline_compute(rasterizer.downscale_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            rasterizer.downscale_pipeline.layout().clone(),
            0,
            downscale_desc_set,
        )
        .dispatch([glyph.width * 3, glyph.height, 1])
        .unwrap()
        .bind_pipeline_compute(rasterizer.hinting_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            rasterizer.hinting_pipeline.layout().clone(),
            0,
            hinting_desc_set,
        )
        .dispatch([glyph.width, glyph.height, 1])
        .unwrap();

    let exec_cmd = cmd_buf.build().unwrap();

    tx_cmd
        .then_execute_same_queue(exec_cmd)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    GpuRasteredGlyph {
        width: glyph.width,
        height: glyph.height,
        bearing_x: glyph.bearing_x,
        bearing_y: glyph.bearing_y,
        advance_w: glyph.advance_w,
        bitmap: hinting_image,
        unique_id: glyph.unique_id,
    }
}
