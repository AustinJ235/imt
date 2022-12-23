use std::time::Instant;

use basalt::interface::bin::{self, Bin, BinStyle};
use basalt::interface::slider::{self, Slider};
use basalt::{Basalt, BstOptions};
use imt::parse::Font;
use imt::raster::gpu::GpuRasterizer;
use imt::raster::ScaledGlyph;
use parking_lot::Mutex;

pub const TEXT_HEIGHT: f32 = 32.0;
pub const TEXT: &'static str = "Sphinx of black quartz, judge my vow.";
pub const VARIATION_INSTANCE: usize = 3;

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

            let coords = font.fvar_table().unwrap().instances[VARIATION_INSTANCE]
                .coordinates
                .clone();
            let mut norm_coords = coords.clone();

            imt::util::variation::normalize_axis_coords(&font, &mut norm_coords).unwrap();

            let rasterizer = GpuRasterizer::new(basalt.compute_queue());

            struct RenderState {
                disp_bins: Vec<Arc<Bin>>,
                axis_coords: Vec<f32>,
                size: f32,
            }

            start = Instant::now();

            let render_state: Arc<Mutex<RenderState>> = Arc::new(Mutex::new(RenderState {
                disp_bins: render_line(
                    &basalt,
                    &font,
                    &rasterizer,
                    TEXT,
                    TEXT_HEIGHT,
                    &norm_coords,
                    10.0,
                ),
                axis_coords: coords,
                size: TEXT_HEIGHT,
            }));

            println!(
                "Time to Raster: {} ms",
                start.elapsed().as_micros() as f32 / 1000.0
            );

            let bst = basalt.clone();

            let change_axis: Arc<dyn Fn(usize, f32) + Send + Sync> =
                Arc::new(move |axis, value| {
                    let start = Instant::now();
                    let mut state = render_state.lock();

                    if state.disp_bins.is_empty() {
                        return;
                    }

                    // state.disp_bins.clear();

                    if axis == 100 {
                        state.size = value;
                    } else {
                        state.axis_coords[axis] = value;
                    }

                    let mut coords = state.axis_coords.clone();
                    imt::util::variation::normalize_axis_coords(&font, &mut coords).unwrap();

                    state.disp_bins =
                        render_line(&bst, &font, &rasterizer, TEXT, state.size, &coords, 10.0);

                    println!(
                        "Time to Raster: {} ms",
                        start.elapsed().as_micros() as f32 / 1000.0
                    );
                });

            let _size = create_slider(
                basalt.clone(),
                String::from("Size"),
                150.0,
                100,
                4.0,
                96.0,
                1.0,
                32.0,
                change_axis.clone(),
            );

            let _weight = create_slider(
                basalt.clone(),
                String::from("Weight"),
                115.0,
                0,
                100.0,
                1000.0,
                50.0,
                400.0,
                change_axis.clone(),
            );

            let _width = create_slider(
                basalt.clone(),
                String::from("Width"),
                80.0,
                1,
                25.0,
                150.0,
                5.0,
                100.0,
                change_axis.clone(),
            );

            let _grade = create_slider(
                basalt.clone(),
                String::from("Grade"),
                45.0,
                3,
                -200.0,
                150.0,
                25.0,
                0.0,
                change_axis.clone(),
            );

            let _slant = create_slider(
                basalt.clone(),
                String::from("Slant"),
                10.0,
                4,
                -10.0,
                0.0,
                1.0,
                0.0,
                change_axis.clone(),
            );

            basalt.wait_for_exit().unwrap();
        }),
    );
}

fn create_slider(
    basalt: Arc<Basalt>,
    name: String,
    pos_from_b: f32,
    axis: usize,
    min: f32,
    max: f32,
    step: f32,
    default: f32,
    call: Arc<dyn Fn(usize, f32) + Send + Sync>,
) -> Arc<Slider> {
    let name_bin = basalt.interface_ref().new_bin();

    name_bin
        .style_update(BinStyle {
            pad_t: Some(7.0),
            pos_from_l: Some(10.0),
            pos_from_b: Some(pos_from_b),
            height: Some(30.0),
            width: Some(65.0),
            text: name,
            text_color: Some(bin::Color::srgb_hex("ffffff")),
            text_height: Some(13.0),
            ..BinStyle::default()
        })
        .debug();

    let slider = Slider::new(basalt.clone(), None);
    slider.container.keep_alive(name_bin);

    slider
        .container
        .style_update(BinStyle {
            pos_from_l: Some(75.0),
            pos_from_b: Some(pos_from_b),
            height: Some(30.0),
            width: Some(300.0),
            ..slider.container.style_copy()
        })
        .debug();

    slider
        .input_box
        .style_update(BinStyle {
            pad_t: Some(7.0),
            back_color: None,
            text_color: Some(bin::Color::srgb_hex("ffffff")),
            border_size_t: Some(0.0),
            border_size_b: Some(0.0),
            border_size_l: Some(0.0),
            border_size_r: Some(0.0),
            ..slider.input_box.style_copy()
        })
        .debug();

    slider.set_min_max(min, max);
    slider.set_step_size(step);
    slider.set(default);
    slider.set_method(slider::Method::RoundToStep);

    slider.on_change(move |val| {
        call(axis, val);
    });

    slider
}

fn render_line<T: AsRef<str>>(
    basalt: &Arc<Basalt>,
    font: &Font,
    rasterizer: &GpuRasterizer,
    text: T,
    size: f32,
    coords: &[f32],
    pos_from_t: f32,
) -> Vec<Arc<Bin>> {
    let bin_count = text
        .as_ref()
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .count();
    let mut empty_bins = basalt.interface_ref().new_bins(bin_count);
    let mut used_bins = Vec::with_capacity(bin_count);

    let scaler = (1.0 / font.head_table().units_per_em as f32) * size;
    let max_y = (font.head_table().y_max as f32 * scaler).ceil();
    let mut x = 0.0;
    let mut last_x_max = 0.0;

    let mut info: Vec<(f32, f32)> = Vec::with_capacity(bin_count);
    let mut glyphs = Vec::with_capacity(bin_count);

    for c in text.as_ref().chars() {
        let index = font.cmap_table().encoding_records[0]
            .subtable
            .glyph_id_map
            .get(&(c as u16))
            .unwrap();

        let scaled = ScaledGlyph::evaluate(&font, Some(&coords), true, *index, size).unwrap();

        if scaled.outline.is_none() {
            x += scaled.advance_w as f32;
            continue;
        }

        let mut adv = scaled.advance_w as f32;
        let glyph_y = pos_from_t + max_y - scaled.height as f32 - scaled.bearing_y as f32;
        let mut glyph_x = x + scaled.bearing_x as f32;

        if glyph_x < last_x_max {
            let diff = last_x_max - glyph_x;
            adv += diff;
            glyph_x += diff;
        }

        info.push((pos_from_t + glyph_y, 10.0 + glyph_x));
        x += adv;
        last_x_max = glyph_x + scaled.width as f32;
        glyphs.push(scaled);
    }

    let glyphs = rasterizer.process(&glyphs);

    for ((t, l), rastered) in info.into_iter().zip(glyphs.into_iter()) {
        let disp = empty_bins.pop().unwrap();

        disp.style_update(BinStyle {
            pos_from_t: Some(t),
            pos_from_l: Some(l),
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

        used_bins.push(disp);
    }

    used_bins
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
