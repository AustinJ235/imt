use crate::error::*;
use crate::parse::{read_i16, read_u16};

/// Corresponds to the `hhea` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/hhea>
#[derive(Debug, Clone)]
pub struct HheaTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub advance_width_max: u16,
    pub min_left_side_bearing: i16,
    pub min_right_side_bearing: i16,
    pub x_map_extent: i16,
    pub caret_slope_rise: i16,
    pub caret_slow_run: i16,
    pub caret_offset: i16,
    pub metric_data_format: i16,
    pub number_of_h_metrics: u16,
}

impl HheaTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 36 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::HheaTable,
            });
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::HheaTable,
            });
        }

        let ascender = read_i16(bytes, table_offset + 4);
        let descender = read_i16(bytes, table_offset + 6);
        let line_gap = read_i16(bytes, table_offset + 8);
        let advance_width_max = read_u16(bytes, table_offset + 10);
        let min_left_side_bearing = read_i16(bytes, table_offset + 12);
        let min_right_side_bearing = read_i16(bytes, table_offset + 14);
        let x_map_extent = read_i16(bytes, table_offset + 16);
        let caret_slope_rise = read_i16(bytes, table_offset + 18);
        let caret_slow_run = read_i16(bytes, table_offset + 20);
        let caret_offset = read_i16(bytes, table_offset + 22);

        if read_i16(bytes, table_offset + 24) != 0
            || read_i16(bytes, table_offset + 26) != 0
            || read_i16(bytes, table_offset + 28) != 0
            || read_i16(bytes, table_offset + 30) != 0
        {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::HheaTable,
            });
        }

        let metric_data_format = read_i16(bytes, table_offset + 32);
        let number_of_h_metrics = read_u16(bytes, table_offset + 34);

        Ok(Self {
            major_version,
            minor_version,
            ascender,
            descender,
            line_gap,
            advance_width_max,
            min_left_side_bearing,
            min_right_side_bearing,
            x_map_extent,
            caret_slope_rise,
            caret_slow_run,
            caret_offset,
            metric_data_format,
            number_of_h_metrics,
        })
    }
}
