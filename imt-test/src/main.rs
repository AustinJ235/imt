use std::time::Instant;

use basalt::interface::bin::{self, BinStyle, ImageEffect};
use basalt::{Basalt, BstOptions};

pub const TEXT_HEIGHT: f32 = 13.0;
pub const TEXT: &'static str = "Sphinx of black quartz, judge my vow.";

fn main() {
    Basalt::initialize(
        BstOptions::default()
            .window_size(1024, 512)
            .title("Basalt")
            //.imt_fill_quality(basalt::ilmenite::raster::ImtFillQuality::Best)
            //.imt_sample_quality(basalt::ilmenite::raster::ImtSampleQuality::Best)
            .app_loop(),
        Box::new(move |basalt_res| {
            let basalt = basalt_res.unwrap();
            let roboto = include_bytes!("../../imt/src/RobotoFlex.ttf");
            let font = imt::parse::Font::from_bytes(roboto).unwrap();

            let fvar = font.fvar_table().unwrap();

            for axis in fvar.axes.iter() {
                if axis.hidden_axis() {
                    continue;
                }

                let name = font
                    .name_table()
                    .name_records
                    .iter()
                    .filter(|record| record.name_id == axis.axis_name_id)
                    .next()
                    .unwrap();

                println!(
                    "Axis '{}', Min: {}, Default: {}, Max: {}",
                    name.name, axis.min_value, axis.default_value, axis.max_value
                );
            }

            let mut bins = Vec::new();
            let mut x = 0.0;
            let scaler = (1.0 / font.head_table().units_per_em as f32) * TEXT_HEIGHT;
            let max_y = (font.head_table().y_max as f32 * scaler).ceil();

            let rasterizer = imt::raster::gpu::GpuRasterizer::new(basalt.compute_queue());
            let mut start = Instant::now();

            for c in TEXT.chars() {
                let index = font.cmap_table().encoding_records[0]
                    .subtable
                    .glyph_id_map
                    .get(&(c as u16))
                    .unwrap();

                if c == ' ' {
                    let hor_metric = &font.hmtx_table().hor_metric[*index as usize];
                    x += (hor_metric.advance_width as f32 * scaler).ceil();
                    continue;
                }

                let render =
                    imt::raster::gpu::compute::raster(&font, *index, TEXT_HEIGHT, &rasterizer);
                let hor_metric = &font.hmtx_table().hor_metric[*index as usize];
                let y_offset = max_y - render.height as f32 - render.bearing_y as f32;
                let outline = font.glyf_table().outlines.get(&index).unwrap().clone();
                let raw_width = (outline.x_max as f32 - outline.x_min as f32) * scaler;
                let add_advance = render.width as f32 - raw_width;
                let advance = ((hor_metric.advance_width as f32 * scaler) + add_advance).ceil();
                let x_offset = (outline.x_min as f32 * scaler).ceil();
                let disp = basalt.interface_ref().new_bin();

                disp.style_update(BinStyle {
                    pos_from_t: Some(10.0 + y_offset),
                    pos_from_l: Some(10.0 + x_offset + x),
                    // border_color_t: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_b: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_l: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_r: Some(bin::Color::srgb_hex("ffffffa0")),
                    border_size_t: Some(1.0),
                    border_size_b: Some(1.0),
                    border_size_l: Some(1.0),
                    border_size_r: Some(1.0),
                    height: Some(render.height as f32),
                    width: Some(render.width as f32),
                    back_image_raw: Some(imt_image_to_bst(render.bitmap)),
                    ..BinStyle::default()
                })
                .debug();

                x += advance;
                bins.push(disp);
            }

            println!("New: {} ms", start.elapsed().as_micros() as f32 / 1000.0);
            start = Instant::now();
            let disp = basalt.interface_ref().new_bin();

            disp.style_update(BinStyle {
                pos_from_t: Some(40.0 + TEXT_HEIGHT),
                pos_from_l: Some(10.0),
                pos_from_r: Some(10.0),
                pos_from_b: Some(0.0),
                text: TEXT.to_string(),
                text_color: Some(bin::Color::srgb_hex("ffffff")),
                text_height: Some(TEXT_HEIGHT),
                ..BinStyle::default()
            })
            .debug();

            disp.wait_for_update();
            println!("Old: {} ms", start.elapsed().as_micros() as f32 / 1000.0);
            basalt.wait_for_exit().unwrap();
        }),
    );
}

use std::sync::Arc;

use basalt::image_view::BstImageView;
use imt::raster::gpu::image_view::{ImtImageVarient, ImtImageView};

fn imt_image_to_bst(view: Arc<ImtImageView>) -> Arc<BstImageView> {
    let actual_view = view.image_view_ref();

    match &**actual_view.image() {
        ImtImageVarient::Storage(img) => BstImageView::from_storage(img.clone()).unwrap(),
        ImtImageVarient::Attachment(img) => BstImageView::from_attachment(img.clone()).unwrap(),
    }
}
