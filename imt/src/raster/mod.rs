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

#[inline(always)]
fn round_left(v: f32) -> f32 {
    v.trunc() - v.is_sign_negative() as i8 as f32
}

#[inline(always)]
fn round_right(v: f32) -> f32 {
    v.trunc() + v.is_sign_positive() as i8 as f32
}

#[inline]
fn f32_to_dimension(v: f32) -> Option<u32> {
    if v < 0.0 {
        None
    } else {
        let int = v as u32;

        if int == 0 {
            None
        } else {
            Some(int)
        }
    }
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
            let width_before = outline.x_max - outline.x_min;

            match outline_apply_gvar(font, glyph_id, &mut outline, coords) {
                Err(ImtUtilError::InvalidCoords) => return Err(ScaledGlyphErr::InvalidCoords),
                Err(ImtUtilError::MalformedOutline) => return Err(ScaledGlyphErr::Malformed),
                _ => (),
            }

            advance_w += ((outline.x_max - outline.x_min) - width_before) * scaler;
        }

        // Horizonal

        let x_max_raw = outline.x_max * scaler;
        let x_min_raw = outline.x_min * scaler;
        let width_raw = x_max_raw - x_min_raw;
        let x_max_whole = round_right(x_max_raw);
        let x_min_whole = round_left(x_min_raw);
        let width_whole = x_max_whole - x_min_whole;
        let x_offset = (x_min_raw - x_min_whole) - x_min_raw;
        let width = f32_to_dimension(width_whole).ok_or(ScaledGlyphErr::Malformed)?;
        let bearing_x = x_min_whole as i16;
        advance_w -= width_whole - width_raw;

        // Vertical

        let y_max_raw = outline.y_max * scaler;
        let y_min_raw = outline.y_min * scaler;
        let y_max_whole = round_right(y_max_raw);
        let y_min_whole = round_left(y_min_raw);
        let height_whole = y_max_whole - y_min_whole;
        let y_offset = (y_min_raw - y_min_whole) - y_min_raw;
        let height = f32_to_dimension(height_whole).ok_or(ScaledGlyphErr::Malformed)?;
        let bearing_y = y_min_whole as i16;

        // Apply scaling transformations

        for point in outline.points.iter_mut() {
            point.x = ((point.x * scaler) + x_offset) / width_whole;
            point.y = (height_whole - ((point.y * scaler) + y_offset)) / height_whole;
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
