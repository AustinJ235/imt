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

use crate::parse::Font;
use crate::raster::gpu::image_view::ImtImageView;
use crate::raster::gpu::shaders::nonzero_cs;
use crate::raster::gpu::GpuRasterizer;

pub struct ComputeGlyphRender {
    pub width: u32,
    pub height: u32,
    pub bearing_y: i16,
    pub bitmap: Arc<ImtImageView>,
}

pub fn raster(
    font: &Font,
    glyph_index: u16,
    size: f32,
    rasterizer: &GpuRasterizer,
) -> ComputeGlyphRender {
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
    let trans_x = |x: f32| (x - outline.x_min as f32) * scaler_hori;

    let (image_h, bearing_y, scaler_above, scaler_below) =
        if outline.y_max <= 0 || outline.y_min >= 0 {
            // Everything above or below baseline
            let height_f = (outline.y_max as f32 - outline.y_min as f32) * scaler;
            let y_max_r = (outline.y_max as f32 * scaler).ceil();
            let y_min_r = (outline.y_min as f32 * scaler).floor(); // Ceil or Floor?
            let height_r = y_max_r - y_min_r;
            let image_h = height_r as u32;
            let scaler_vert = ((height_r / height_f) * scaler) / image_h as f32;
            (image_h, y_min_r as i16, scaler_vert, scaler_vert)
        } else {
            // Some above and below baseline
            let above_f = outline.y_max as f32 * scaler;
            let below_f = outline.y_min as f32 * scaler;
            let above_r = above_f.ceil();
            let below_r = below_f.round(); // Ideally floor, but on small text round works better
            let image_h = (above_r - below_r) as u32;
            let scaler_above = ((above_r / above_f) * scaler) / image_h as f32;
            let scaler_below = ((below_r / below_f) * scaler) / image_h as f32;
            (image_h, below_r as i16, scaler_above, scaler_below)
        };

    let y_min_f = outline.y_min as f32;

    let trans_y = |y: f32| {
        if y > 0.0 {
            1.0 - ((y - y_min_f) * scaler_above)
        } else {
            1.0 - ((y - y_min_f) * scaler_below)
        }
    };

    let mut segment_data: Vec<[f32; 4]> = Vec::new();

    for segment in outline.segments() {
        segment_data.push([
            trans_x(segment.p1.x),
            trans_y(segment.p1.y),
            trans_x(segment.p2.x),
            trans_y(segment.p2.y),
        ]);
    }

    for curve in outline.curves() {
        for i in 0..16 {
            let [p1x, p1y] = curve.evaluate(i as f32 / 16.0);
            let [p2x, p2y] = curve.evaluate((i + 1) as f32 / 16.0);
            segment_data.push([trans_x(p1x), trans_y(p1y), trans_x(p2x), trans_y(p2y)]);
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
        extent: [image_w as f32 * 12.0, image_h as f32 * 4.0],
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
                width: image_w * 12,
                height: image_h * 4,
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
                width: image_w * 3,
                height: image_h * 1,
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
                width: image_w,
                height: image_h,
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
        .dispatch([image_w * 12, image_h * 4, 1])
        .unwrap()
        .bind_pipeline_compute(rasterizer.downscale_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            rasterizer.downscale_pipeline.layout().clone(),
            0,
            downscale_desc_set,
        )
        .dispatch([image_w * 3, image_h, 1])
        .unwrap()
        .bind_pipeline_compute(rasterizer.hinting_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            rasterizer.hinting_pipeline.layout().clone(),
            0,
            hinting_desc_set,
        )
        .dispatch([image_w, image_h, 1])
        .unwrap();

    let exec_cmd = cmd_buf.build().unwrap();

    tx_cmd
        .then_execute_same_queue(exec_cmd)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    ComputeGlyphRender {
        width: image_w,
        height: image_h,
        bearing_y,
        bitmap: hinting_image,
    }
}
