use crate::error::*;
use crate::parse::{read_f2dot14, read_u16};

const TRUNCATED: ImtError = ImtError {
    kind: ImtErrorKind::Truncated,
    source: ImtErrorSource::AvarTable,
};

const MALFORMED: ImtError = ImtError {
    kind: ImtErrorKind::Malformed,
    source: ImtErrorSource::AvarTable,
};

#[derive(Debug, Clone)]
pub struct AvarTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub segment_maps: Vec<SegmentMap>,
}

#[derive(Debug, Clone)]
pub struct SegmentMap {
    pub axis_value_maps: Vec<AxisValueMap>,
}

#[derive(Debug, Clone)]
pub struct AxisValueMap {
    pub from_coord: f32,
    pub to_coord: f32,
}

impl AvarTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 8 > bytes.len() {
            return Err(TRUNCATED);
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::AvarTable,
            });
        }

        if read_u16(bytes, table_offset + 4) != 0 {
            return Err(MALFORMED);
        }

        let axis_count = read_u16(bytes, table_offset + 6) as usize;
        let mut segment_maps = Vec::with_capacity(axis_count);
        let mut segment_map_offset = table_offset + 8;

        for _ in 0..axis_count {
            if segment_map_offset + 2 > bytes.len() {
                return Err(TRUNCATED);
            }

            let position_map_count = read_u16(bytes, segment_map_offset) as usize;

            if segment_map_offset + 2 + (position_map_count * 4) > bytes.len() {
                return Err(TRUNCATED);
            }

            let mut axis_value_maps = Vec::with_capacity(position_map_count);

            for i in 0..position_map_count {
                axis_value_maps.push(AxisValueMap {
                    from_coord: read_f2dot14(bytes, segment_map_offset + 2 + (i * 4)),
                    to_coord: read_f2dot14(bytes, segment_map_offset + 4 + (i * 4)),
                });
            }

            segment_maps.push(SegmentMap {
                axis_value_maps,
            });

            segment_map_offset += 2 + (position_map_count * 4);
        }

        let avar = Self {
            major_version,
            minor_version,
            segment_maps,
        };

        for segment_map in avar.segment_maps.iter() {
            if !segment_map.axis_value_maps.is_empty() {
                if segment_map.axis_value_maps.len() < 3 {
                    return Err(MALFORMED);
                }

                let mut has_zero_to_zero = false;

                for i in 0..segment_map.axis_value_maps.len() {
                    if i == 0 {
                        if segment_map.axis_value_maps[i].from_coord != -1.0
                            || segment_map.axis_value_maps[i].to_coord != -1.0
                        {
                            return Err(MALFORMED);
                        }
                    } else if i == segment_map.axis_value_maps.len() - 1 {
                        if segment_map.axis_value_maps[i].from_coord != 1.0
                            || segment_map.axis_value_maps[i].to_coord != 1.0
                        {
                            return Err(MALFORMED);
                        }
                    } else {
                        has_zero_to_zero |= segment_map.axis_value_maps[i].from_coord == 0.0
                            && segment_map.axis_value_maps[i].to_coord == 0.0;

                        if segment_map.axis_value_maps[i].from_coord
                            < segment_map.axis_value_maps[i - 1].to_coord
                            || segment_map.axis_value_maps[i].to_coord
                                > segment_map.axis_value_maps[i + 1].from_coord
                        {
                            return Err(MALFORMED);
                        }
                    }
                }

                if !has_zero_to_zero {
                    return Err(MALFORMED);
                }
            }
        }

        Ok(avar)
    }
}
