use crate::parse::{Font, Outline};
use crate::util::variation::*;
use crate::util::ImtUtilError;

pub mod gpu;

/// A glyph outline that is scaled with bearings and advance.
///
/// # Notes
/// - `width`, `height`, `bearing_x`, `bearing_y` will be *zero* if the glyph does not have an
///   outline. `outline` will also be `None`. In this case the pen location should be advanced by
///   the amount specified by `advance_w`.
#[derive(Debug, Clone)]
pub struct ScaledGlyph {
    /// Width the image should be
    pub width: u32,
    /// Height the image should be
    pub height: u32,
    /// Left offset from pen location (does not effect location)
    pub bearing_x: i16,
    /// Distance from baseline
    pub bearing_y: i16,
    /// Amount to advance pen location
    pub advance_w: i16,
    /// Outline point values will be between `0..=1` with `Y` down.
    pub outline: Option<Outline>,
    /// An ID that is unique to the paramters used.
    /// **Note**: This will not be unique across `Font`'s
    pub unique_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaledGlyphErr {
    /// Glyph data is missing
    Missing,
    /// Coordinates are invalid
    InvalidCoords,
    /// Font is malformed
    Malformed,
}

impl ScaledGlyph {
    pub fn evaluate(
        font: &Font,
        coords: Option<&[f32]>,
        coords_normalized: bool,
        glyph_id: u16,
        size: f32,
    ) -> Result<Self, ScaledGlyphErr> {
        let coords = match coords {
            Some(coords) => {
                let mut coords = coords.to_vec();

                if !coords_normalized {
                    normalize_axis_coords(font, &mut coords)
                        .map_err(|_| ScaledGlyphErr::InvalidCoords)?;
                }

                Some(coords)
            },
            None => None,
        };

        let unique_id = match coords.as_ref() {
            Some(coords) => unique_id(glyph_id, size, Some(coords), 0),
            None => {
                unique_id(
                    glyph_id,
                    size,
                    None,
                    match font.fvar_table() {
                        Some(fvar) => fvar.axes.len(),
                        None => 0,
                    },
                )
            },
        };

        let mut advance_w = font
            .hmtx_table()
            .hor_metric
            .get(glyph_id as usize)
            .ok_or(ScaledGlyphErr::Missing)?
            .advance_width as f32;

        if let Some(coords) = coords.as_ref() {
            advance_w +=
                advance_width(font, glyph_id, coords).map_err(|_| ScaledGlyphErr::InvalidCoords)?;
        }

        let scaler = (1.0 / font.head_table().units_per_em as f32) * size;
        advance_w *= scaler;

        let mut outline = match font.glyf_table().outlines.get(&glyph_id) {
            Some(some) => some.clone(),
            None => {
                return Ok(Self {
                    width: 0,
                    height: 0,
                    bearing_x: 0,
                    bearing_y: 0,
                    advance_w: advance_w.ceil() as i16,
                    outline: None,
                    unique_id,
                });
            },
        };

        if let Some(coords) = coords.as_ref() {
            match outline_apply_gvar(font, glyph_id, &mut outline, coords) {
                Err(ImtUtilError::InvalidCoords) => return Err(ScaledGlyphErr::InvalidCoords),
                Err(ImtUtilError::MalformedOutline) => return Err(ScaledGlyphErr::Malformed),
                _ => (),
            }
        }

        // Horizonal

        let width_f = (outline.x_max - outline.x_min) * scaler;
        let width_r = width_f.ceil();
        let width = width_r as u32;
        let scaler_hori = ((width_r / width_f) * scaler) / width_r;
        let bearing_x = (outline.x_min as f32 * scaler).round() as i16;
        advance_w += width_r - width_f;
        let transform_x = |x: f32| (x - outline.x_min) * scaler_hori;

        // Vertical

        let (height, bearing_y, scaler_above, scaler_below) =
            if outline.y_max <= 0.0 || outline.y_min >= 0.0 {
                // Everything above or below baseline
                let height_f = (outline.y_max - outline.y_min) * scaler;
                let y_max_r = (outline.y_max * scaler).ceil();
                let y_min_r = (outline.y_min * scaler).floor(); // Ceil or Floor?
                let height_r = y_max_r - y_min_r;
                let image_h = height_r as u32;
                let scaler_vert = ((height_r / height_f) * scaler) / image_h as f32;
                (image_h, y_min_r as i16, scaler_vert, scaler_vert)
            } else {
                // Some above and below baseline
                let above_f = outline.y_max * scaler;
                let below_f = outline.y_min * scaler;
                let above_r = above_f.ceil();
                let below_r = below_f.round(); // Ideally floor, but on small text round works better
                let image_h = (above_r - below_r) as u32;
                let scaler_above = ((above_r / above_f) * scaler) / image_h as f32;
                let scaler_below = ((below_r / below_f) * scaler) / image_h as f32;
                (image_h, below_r as i16, scaler_above, scaler_below)
            };

        let transform_y = |y: f32| -> f32 {
            if y > 0.0 {
                1.0 - ((y - outline.y_min) * scaler_above)
            } else {
                1.0 - ((y - outline.y_min) * scaler_below)
            }
        };

        //

        for point in outline.points.iter_mut() {
            point.x = transform_x(point.x);
            point.y = transform_y(point.y);
        }

        outline.rebuild().unwrap();

        Ok(Self {
            width,
            height,
            bearing_x,
            bearing_y,
            advance_w: advance_w.ceil() as i16,
            outline: Some(outline),
            unique_id,
        })
    }
}

fn unique_id(glyph_id: u16, size: f32, coords: Option<&[f32]>, axis_count: usize) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::default();
    hasher.write_u16(glyph_id);
    hasher.write_u32(size.to_bits());

    match coords {
        Some(coords) => {
            for coord in coords.iter() {
                hasher.write_u32(coord.to_bits());
            }
        },
        None => {
            for _ in 0..axis_count {
                hasher.write_u32(0);
            }
        },
    }

    hasher.finish()
}
