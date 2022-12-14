use std::collections::BTreeMap;

use crate::error::*;
use crate::parse::{read_f2dot14, read_u16, read_u32, GlyfTable};

/// Corresponds to the `gvar` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/gvar>
#[derive(Debug, Clone)]
pub struct GvarTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub axis_count: usize,
    pub glyph_variations: BTreeMap<u16, GlyphVariation>,
}

#[derive(Debug, Clone)]
pub struct GlyphVariation {
    pub tuples: Vec<TupleVariation>,
}

#[derive(Debug, Clone)]
pub struct TupleVariation {
    /// Length equal to axis count
    pub peak: Vec<f32>,
    pub interm: Option<IntermediateTuples>,
    /// If points is empty all points are used
    pub points: Vec<u16>,
    /// Length equal to points length or num of packed points from glyf
    pub deltas: Vec<[i16; 2]>,
}

#[derive(Debug, Clone)]
pub struct IntermediateTuples {
    /// Length equal to axis count
    pub start: Vec<f32>,
    /// Length equal to axis count
    pub end: Vec<f32>,
}

const TRUNCATED: ImtError = ImtError {
    kind: ImtErrorKind::Truncated,
    source: ImtErrorSource::GvarTable,
};

const MALFORMED: ImtError = ImtError {
    kind: ImtErrorKind::Malformed,
    source: ImtErrorSource::GvarTable,
};

impl GvarTable {
    pub fn try_parse(
        bytes: &[u8],
        table_offset: usize,
        glyf_table: &GlyfTable,
    ) -> Result<Self, ImtError> {
        if table_offset + 20 > bytes.len() {
            return Err(TRUNCATED);
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::GvarTable,
            });
        }

        let axis_count = read_u16(bytes, table_offset + 4) as usize;
        let share_tuple_count = read_u16(bytes, table_offset + 6) as usize;
        let shared_tuples_offset = read_u32(bytes, table_offset + 8) as usize + table_offset;
        let glyph_count = read_u16(bytes, table_offset + 12) as usize;
        let flags = read_u16(bytes, table_offset + 14);
        let glyph_variation_data_array_offset =
            read_u32(bytes, table_offset + 16) as usize + table_offset;
        let mut glyph_variation_data_offsets = Vec::with_capacity(glyph_count);

        if flags & 1 == 1 {
            if table_offset + 20 + ((glyph_count + 1) * 4) > bytes.len() {
                return Err(TRUNCATED);
            }

            for i in 0..=glyph_count {
                let glyph_variation_data_offset =
                    read_u32(bytes, table_offset + 20 + (i * 4)) as usize;
                glyph_variation_data_offsets
                    .push(glyph_variation_data_array_offset + glyph_variation_data_offset);
            }
        } else {
            if table_offset + 20 + ((glyph_count + 1) * 2) > bytes.len() {
                return Err(TRUNCATED);
            }

            for i in 0..=glyph_count {
                let glyph_variation_data_offset =
                    read_u16(bytes, table_offset + 20 + (i * 2)) as usize * 2;
                glyph_variation_data_offsets
                    .push(glyph_variation_data_array_offset + glyph_variation_data_offset);
            }
        }

        if shared_tuples_offset + (share_tuple_count * 2 * axis_count) > bytes.len() {
            return Err(TRUNCATED);
        }

        let mut shared_tuples: Vec<f32> = Vec::with_capacity(share_tuple_count);

        for i in 0..(share_tuple_count * axis_count) {
            shared_tuples.push(read_f2dot14(bytes, shared_tuples_offset + (i * 2)));
        }

        let mut glyph_variations = BTreeMap::new();

        for i in 0..glyph_count {
            let outline = match glyf_table.outlines.get(&(i as u16)) {
                Some(outline) => outline,
                None => continue,
            };

            // set & check glyph variation data

            let s = glyph_variation_data_offsets[i];
            let e = glyph_variation_data_offsets[i + 1];

            if s > bytes.len() || e > bytes.len() || s > e {
                return Err(MALFORMED);
            }

            if s == e {
                continue;
            }

            let glyph_variation_data = &bytes[s..e];

            // read glyph variation header

            if 4 > glyph_variation_data.len() {
                return Err(TRUNCATED);
            }

            let tuple_variation_count = read_u16(glyph_variation_data, 0);
            let mut serialized_offset = read_u16(glyph_variation_data, 2) as usize;
            let has_shared_point_numbers = tuple_variation_count & 0x8000 != 0;
            let tuple_variation_count = (tuple_variation_count & 0x0fff) as usize;

            // set & check serialized data

            if serialized_offset >= glyph_variation_data.len() {
                return Err(TRUNCATED);
            }

            let serialized_data = &glyph_variation_data[serialized_offset..];
            serialized_offset = 0;

            // read shared point numbers

            let mut shared_point_numbers: Vec<u16> = Vec::new();

            if has_shared_point_numbers {
                serialized_offset +=
                    parse_packed_points(serialized_data, &mut shared_point_numbers)?;
            }

            // read tuple variations

            let mut tuple_variation_header_offset = 4;
            let mut tuple_variations: Vec<TupleVariation> =
                Vec::with_capacity(tuple_variation_count);

            for _ in 0..tuple_variation_count {
                if tuple_variation_header_offset + 4 > glyph_variation_data.len() {
                    return Err(TRUNCATED);
                }

                let variation_data_size =
                    read_u16(glyph_variation_data, tuple_variation_header_offset) as usize;
                let tuple_index = read_u16(glyph_variation_data, tuple_variation_header_offset + 2);
                tuple_variation_header_offset += 4;
                let has_embedded_peak_tuple = tuple_index & 0x8000 != 0;
                let has_intermediate_region = tuple_index & 0x4000 != 0;
                let has_private_point_numbers = tuple_index & 0x2000 != 0;
                let mut tuple_index = (tuple_index & 0x0FFF) as usize;

                let peak_tuple = if has_embedded_peak_tuple {
                    if tuple_variation_header_offset + (2 * axis_count) > glyph_variation_data.len()
                    {
                        return Err(TRUNCATED);
                    }

                    let mut peak_tuple = Vec::with_capacity(axis_count);

                    for _ in 0..axis_count {
                        peak_tuple.push(read_f2dot14(
                            glyph_variation_data,
                            tuple_variation_header_offset,
                        ));
                        tuple_variation_header_offset += 2;
                    }

                    peak_tuple
                } else {
                    tuple_index *= axis_count;

                    if tuple_index + axis_count > shared_tuples.len() {
                        return Err(MALFORMED);
                    }

                    shared_tuples[tuple_index..(tuple_index + axis_count)].to_vec()
                };

                let intermediate_tuples = if has_intermediate_region {
                    if tuple_variation_header_offset + (4 * axis_count) > glyph_variation_data.len()
                    {
                        return Err(TRUNCATED);
                    }

                    let mut start_tuple = Vec::with_capacity(axis_count);

                    for _ in 0..axis_count {
                        start_tuple.push(read_f2dot14(
                            glyph_variation_data,
                            tuple_variation_header_offset,
                        ));
                        tuple_variation_header_offset += 2;
                    }

                    let mut end_tuple = Vec::with_capacity(axis_count);

                    for _ in 0..axis_count {
                        end_tuple.push(read_f2dot14(
                            glyph_variation_data,
                            tuple_variation_header_offset,
                        ));
                        tuple_variation_header_offset += 2;
                    }

                    Some(IntermediateTuples {
                        start: start_tuple,
                        end: end_tuple,
                    })
                } else {
                    None
                };

                let mut point_numbers = Vec::new();

                let delta_offset = if has_private_point_numbers {
                    if serialized_offset >= serialized_data.len() {
                        return Err(TRUNCATED);
                    }

                    parse_packed_points(
                        &serialized_data
                            [serialized_offset..(serialized_offset + variation_data_size)],
                        &mut point_numbers,
                    )?
                } else {
                    point_numbers.extend_from_slice(&shared_point_numbers);
                    0
                };

                if point_numbers.last().copied().unwrap_or(0) as usize
                    > outline.num_packed_points + 4
                {
                    return Err(MALFORMED);
                }

                if serialized_offset + delta_offset >= serialized_data.len() {
                    return Err(MALFORMED);
                }

                let delta_count = if point_numbers.is_empty() {
                    outline.num_packed_points + 4
                } else {
                    point_numbers.len()
                };

                let deltas = parse_packed_deltas(
                    &serialized_data[(serialized_offset + delta_offset)
                        ..(serialized_offset + variation_data_size)],
                    delta_count,
                )?;

                serialized_offset += variation_data_size;

                // Sanity Checks

                if let Some(interm) = intermediate_tuples.as_ref() {
                    for (p, (s, e)) in peak_tuple
                        .iter()
                        .zip(interm.start.iter().zip(interm.end.iter()))
                    {
                        if *s > *e
                            || *p < *s
                            || *p > *e
                            || *s < -1.0
                            || *s > 1.0
                            || *e < -1.0
                            || *e > 1.0
                        {
                            return Err(MALFORMED);
                        }
                    }
                }

                for p in peak_tuple.iter() {
                    if *p < -1.0 || *p > 1.0 {
                        return Err(MALFORMED);
                    }
                }

                for point in point_numbers.iter() {
                    if *point as usize >= outline.num_packed_points + 4 {
                        return Err(MALFORMED);
                    }
                }

                tuple_variations.push(TupleVariation {
                    peak: peak_tuple,
                    interm: intermediate_tuples,
                    points: point_numbers,
                    deltas,
                });
            }

            glyph_variations.insert(
                i as u16,
                GlyphVariation {
                    tuples: tuple_variations,
                },
            );
        }

        Ok(Self {
            major_version,
            minor_version,
            axis_count,
            glyph_variations,
        })
    }
}

fn parse_packed_deltas(bytes: &[u8], count: usize) -> Result<Vec<[i16; 2]>, ImtError> {
    let count_x2 = count * 2;
    let mut deltas = Vec::with_capacity(count_x2);
    let mut are_zero = false;
    let mut are_words = false;
    let mut remaining = 0;
    let mut offset = 0;

    loop {
        if remaining == 0 {
            if deltas.len() > count_x2 {
                return Err(MALFORMED);
            }

            if deltas.len() == count_x2 {
                break;
            }

            if offset + 1 > bytes.len() {
                return Err(TRUNCATED);
            }

            are_zero = bytes[offset] & 0x80 != 0;
            are_words = bytes[offset] & 0x40 != 0;
            remaining = (bytes[offset] & 0x3F) as usize + 1;
            offset += 1;
        } else if are_zero {
            deltas.push(0);
            remaining -= 1;
        } else if are_words {
            if offset + 2 > bytes.len() {
                return Err(TRUNCATED);
            }

            deltas.push(i16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
            remaining -= 1;
            offset += 2;
        } else {
            if offset + 1 > bytes.len() {
                return Err(TRUNCATED);
            }

            deltas.push(i8::from_be_bytes([bytes[offset]]) as i16);
            remaining -= 1;
            offset += 1;
        }
    }

    debug_assert!(deltas.len() == count_x2);
    let y_deltas = deltas.split_off(count);

    Ok(deltas
        .into_iter()
        .zip(y_deltas.into_iter())
        .map(|(x, y)| [x, y])
        .collect())
}

fn parse_packed_points(bytes: &[u8], points: &mut Vec<u16>) -> Result<usize, ImtError> {
    let mut offset = 0;

    if 1 > bytes.len() {
        return Err(TRUNCATED);
    }

    let total = if bytes[0] & 0x80 != 0 {
        if 2 > bytes.len() {
            return Err(TRUNCATED);
        }

        u16::from_be_bytes([bytes[0] & 0x7F, bytes[1]]) as usize
    } else {
        offset += 1;
        bytes[0] as usize
    };

    if total == 0 {
        return Ok(offset);
    }

    points.reserve_exact(total);
    let mut remaining = 0;
    let mut points_are_words = false;
    let mut last_point = 0;

    loop {
        if remaining == 0 {
            if points.len() == total {
                break;
            }

            if offset >= bytes.len() {
                return Err(TRUNCATED);
            }

            remaining = (bytes[offset] & 0x7F) as usize + 1;
            points_are_words = bytes[offset] & 0x80 != 0;
            offset += 1;

            if points.len() + remaining > total {
                return Err(MALFORMED);
            }
        } else if points_are_words {
            if offset + 2 > bytes.len() {
                return Err(TRUNCATED);
            }

            last_point += read_u16(bytes, offset);
            points.push(last_point);
            offset += 2;
            remaining -= 1;
        } else {
            if offset >= bytes.len() {
                return Err(TRUNCATED);
            }

            last_point += bytes[offset] as u16;
            points.push(last_point);
            offset += 1;
            remaining -= 1;
        }
    }

    Ok(offset)
}
