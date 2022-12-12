use std::collections::BTreeMap;

use crate::error::*;
use crate::parse::{read_f2dot14, read_fixed, read_u16, read_u32, GlyfTable};

/// Corresponds to the `gvar` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/gvar>
#[derive(Debug, Clone)]
pub struct GvarTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub glyph_variation_data: BTreeMap<u16, GlyphVariationData>,
}

#[derive(Debug, Clone)]
pub struct GlyphVariationData {}

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

        for i in 0..glyph_count {
            let num_points = match glyf_table.outlines.get(&(i as u16)) {
                Some(outline) => outline.points().count() + 4,
                None => continue,
            };

            let s = glyph_variation_data_offsets[i];
            let e = glyph_variation_data_offsets[i + 1];

            if s > bytes.len() || e > bytes.len() || s > e {
                return Err(MALFORMED);
            }

            if s == e {
                continue;
            }

            let glyph_variation_data = &bytes[s..e];

            if 4 > glyph_variation_data.len() {
                return Err(TRUNCATED);
            }

            let tuple_variation_count = read_u16(glyph_variation_data, 0);
            let mut serialized_offset = read_u16(glyph_variation_data, 2) as usize;
            let has_shared_point_numbers = tuple_variation_count & 0x8000 != 0;
            let tuple_variation_count = (tuple_variation_count & 0x0fff) as usize;

            if serialized_offset >= glyph_variation_data.len() {
                return Err(TRUNCATED);
            }

            let serialized_data = &glyph_variation_data[serialized_offset..];
            serialized_offset = 0;
            let mut shared_point_numbers: Vec<u16> = Vec::new();

            if has_shared_point_numbers {
                if 1 > serialized_data.len() {
                    return Err(TRUNCATED);
                }

                let total = if serialized_data[0] & 0x80 != 0 {
                    if 2 > serialized_data.len() {
                        return Err(TRUNCATED);
                    }

                    serialized_offset += 2;
                    ((((serialized_data[0] & 0x7F) as u16) << 8) | serialized_data[1] as u16)
                        as usize
                } else {
                    serialized_offset += 1;
                    serialized_data[0] as usize
                };

                shared_point_numbers.reserve_exact(total);
                let mut remaining = 0;
                let mut points_are_words = false;

                loop {
                    if remaining == 0 {
                        if shared_point_numbers.len() == total {
                            break;
                        }

                        if serialized_offset >= serialized_data.len() {
                            return Err(TRUNCATED);
                        }

                        remaining = (serialized_data[serialized_offset] & 0x7F) as usize + 1;
                        points_are_words = serialized_data[serialized_offset] & 0x80 != 0;
                        serialized_offset += 1;

                        if shared_point_numbers.len() + remaining > total {
                            return Err(MALFORMED);
                        }
                    } else if points_are_words {
                        if serialized_offset + 2 > serialized_data.len() {
                            return Err(TRUNCATED);
                        }

                        shared_point_numbers.push(read_u16(serialized_data, serialized_offset));
                        serialized_offset += 2;
                        remaining -= 1;
                    } else {
                        if serialized_offset >= serialized_data.len() {
                            return Err(TRUNCATED);
                        }

                        shared_point_numbers.push(serialized_data[serialized_offset] as u16);
                        serialized_offset += 1;
                        remaining -= 1;
                    }
                }
            }

            println!("{}: {:?}", i, shared_point_numbers);
        }

        Ok(Self {
            major_version,
            minor_version,
            glyph_variation_data: BTreeMap::new(),
        })
    }
}
