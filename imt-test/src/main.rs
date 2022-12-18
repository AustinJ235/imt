use std::time::Instant;

use basalt::interface::bin::{self, BinStyle, ImageEffect};
use basalt::{Basalt, BstOptions};

pub const TEXT_HEIGHT: f32 = 32;
pub const TEXT: &'static str = "Sphinx of black quartz, judge my vow.";
pub const VARIATION_INSTANCE: usize = 6;

/* Axes:
  0: 'wght', Min: 100, Default: 400, Max: 1000
  1: 'wdth', Min: 25, Default: 100, Max: 151
  2: 'opsz', Min: 8, Default: 14, Max: 144
  3: 'GRAD', Min: -200, Default: 0, Max: 150
  4: 'slnt', Min: -10, Default: 0, Max: 0
*/

/* Instances:
  0: 'Thin'
  1: 'ExtraLight'
  2: 'Light'
  3: 'Regular'
  4: 'Medium'
  5: 'SemiBold'
  6: 'Bold'
  7: 'ExtraBold'
  8: 'Black'
  9: 'ExtraBlack'
  10: 'Thin Italic'
  11: 'ExtraLight Italic'
  12: 'Light Italic'
  13: 'Italic'
  14: 'Medium Italic'
  15: 'SemiBold Italic'
  16: 'Bold Italic'
  17: 'ExtraBold Italic'
  18: 'Black Italic'
  19: 'ExtraBlack Italic'
*/

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
            let mut start = Instant::now();
            let font = imt::parse::Font::from_bytes(roboto).unwrap();

            println!(
                "Time to Parse: {} ms",
                start.elapsed().as_micros() as f32 / 1000.0
            );

            let est_bin_count = TEXT
                .chars()
                .filter(|c| !c.is_control() && !c.is_whitespace())
                .count();

            let mut bins = Vec::with_capacity(est_bin_count);
            let mut empty_bins = basalt.interface_ref().new_bins(est_bin_count);

            let scaler = (1.0 / font.head_table().units_per_em as f32) * TEXT_HEIGHT;
            let max_y = (font.head_table().y_max as f32 * scaler).ceil();
            let rasterizer = imt::raster::gpu::GpuRasterizer::new(basalt.compute_queue());

            let mut coords = font.fvar_table().unwrap().instances[VARIATION_INSTANCE]
                .coordinates
                .clone();
            imt::util::variation::normalize_axis_coords(&font, &mut coords).unwrap();

            let mut x = 0.0;
            std::thread::sleep(std::time::Duration::from_millis(500));
            start = Instant::now();

            for c in TEXT.chars() {
                let index = font.cmap_table().encoding_records[0]
                    .subtable
                    .glyph_id_map
                    .get(&(c as u16))
                    .unwrap();

                let scaled = imt::raster::ScaledGlyph::evaluate(
                    &font,
                    Some(&coords),
                    true,
                    *index,
                    TEXT_HEIGHT,
                )
                .unwrap();

                if scaled.outline.is_none() {
                    x += scaled.advance_w as f32;
                    continue;
                }

                let rastered = imt::raster::gpu::compute::raster(&scaled, &rasterizer);
                let position_y = max_y - rastered.height as f32 - rastered.bearing_y as f32;
                let disp = empty_bins.pop().unwrap();

                disp.style_update(BinStyle {
                    pos_from_t: Some(10.0 + position_y),
                    pos_from_l: Some(10.0 + rastered.bearing_x as f32 + x),
                    // border_color_t: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_b: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_l: Some(bin::Color::srgb_hex("ffffffa0")),
                    // border_color_r: Some(bin::Color::srgb_hex("ffffffa0")),
                    border_size_t: Some(1.0),
                    border_size_b: Some(1.0),
                    border_size_l: Some(1.0),
                    border_size_r: Some(1.0),
                    height: Some(rastered.height as f32),
                    width: Some(rastered.width as f32),
                    back_image_raw: Some(imt_image_to_bst(rastered.bitmap)),
                    ..BinStyle::default()
                })
                .debug();

                x += rastered.advance_w as f32;
                bins.push(disp);
            }

            println!("New: {} ms", start.elapsed().as_micros() as f32 / 1000.0);
            /*start = Instant::now();
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
            println!("Old: {} ms", start.elapsed().as_micros() as f32 / 1000.0);*/
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
