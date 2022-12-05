use crate::error::*;
use crate::parse::{read_i16, read_u16, HheaTable, MaxpTable};

/// Corresponds to the `hmtx` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/hmtx>
#[derive(Debug, Clone)]
pub struct HmtxTable {
    pub hor_metric: Vec<HorMetric>,
    pub left_side_bearings: Vec<i16>,
}

impl HmtxTable {
    pub fn try_parse(
        bytes: &[u8],
        table_offset: usize,
        maxp_table: &MaxpTable,
        hhea_table: &HheaTable,
    ) -> Result<Self, ImtError> {
        if maxp_table.num_glyphs < hhea_table.number_of_h_metrics {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::HmtxTable,
            });
        }

        let hor_metric_len = hhea_table.number_of_h_metrics as usize;
        let left_side_bearings_len = maxp_table.num_glyphs as usize - hor_metric_len;

        if table_offset + (hor_metric_len * 4) + (left_side_bearings_len * 2) > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::HmtxTable,
            });
        }

        let mut hor_metric = Vec::with_capacity(hor_metric_len);

        for i in 0..hor_metric_len {
            let offset = table_offset + (i * 4);

            hor_metric.push(HorMetric {
                advance_width: read_u16(bytes, offset),
                lsb: read_i16(bytes, offset + 2),
            });
        }

        let left_side_bearings_offset = table_offset + (hor_metric_len * 4);
        let mut left_side_bearings = Vec::with_capacity(left_side_bearings_len);

        for i in 0..left_side_bearings_len {
            let offset = left_side_bearings_offset + (i * 2);
            left_side_bearings.push(read_i16(bytes, offset));
        }

        Ok(Self {
            hor_metric,
            left_side_bearings,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HorMetric {
    pub advance_width: u16,
    pub lsb: i16,
}
