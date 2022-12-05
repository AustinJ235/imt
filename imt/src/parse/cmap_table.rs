use std::collections::BTreeMap;

use crate::error::*;

#[derive(Debug, Clone)]
pub struct CmapTable {
    pub version: u16,
    pub encoding_records: Vec<EncodingRecord>,
}

impl CmapTable {
    pub fn try_parse(bytes: &[u8], base_offset: usize) -> Result<Self, ImtError> {
        if base_offset + 4 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::CmapTable,
            });
        }

        let version = u16::from_be_bytes(bytes[base_offset..(base_offset + 2)].try_into().unwrap());
        let num_tables = u16::from_be_bytes(
            bytes[(base_offset + 2)..(base_offset + 4)]
                .try_into()
                .unwrap(),
        );

        if (base_offset + 4) + (num_tables as usize * 8) > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::CmapTable,
            });
        }

        let mut encoding_records = Vec::with_capacity(num_tables as usize);

        for table_i in 0..(num_tables as usize) {
            encoding_records.push(EncodingRecord::try_parse(
                bytes,
                base_offset,
                base_offset + 4 + (table_i * 8),
            )?);
        }

        Ok(Self {
            version,
            encoding_records,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EncodingRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub subtable: CmapSubtable,
}

impl EncodingRecord {
    pub fn try_parse(
        bytes: &[u8],
        table_offset: usize,
        base_offset: usize,
    ) -> Result<Self, ImtError> {
        if base_offset + 8 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::EncodingRecord,
            });
        }

        let platform_id =
            u16::from_be_bytes(bytes[base_offset..(base_offset + 2)].try_into().unwrap());
        let encoding_id = u16::from_be_bytes(
            bytes[(base_offset + 2)..(base_offset + 4)]
                .try_into()
                .unwrap(),
        );
        let subtable_offset = u32::from_be_bytes(
            bytes[(base_offset + 4)..(base_offset + 8)]
                .try_into()
                .unwrap(),
        );
        let subtable = CmapSubtable::try_parse(bytes, table_offset + subtable_offset as usize)?;

        Ok(Self {
            platform_id,
            encoding_id,
            subtable,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CmapSubtable {
    pub language: u16,
    pub glyph_id_map: BTreeMap<u16, u16>,
}

impl CmapSubtable {
    pub fn try_parse(bytes: &[u8], base_offset: usize) -> Result<Self, ImtError> {
        if base_offset + 2 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::CmapSubtable,
            });
        }

        let format = u16::from_be_bytes(bytes[base_offset..(base_offset + 2)].try_into().unwrap());

        match format {
            4 => {
                if base_offset + 14 > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::CmapSubtable,
                    });
                }

                // 2..4 length
                let language = u16::from_be_bytes(
                    bytes[(base_offset + 4)..(base_offset + 6)]
                        .try_into()
                        .unwrap(),
                );
                let seg_count = (u16::from_be_bytes(
                    bytes[(base_offset + 6)..(base_offset + 8)]
                        .try_into()
                        .unwrap(),
                ) / 2) as usize;
                // 8..10 searchRange
                // 10..12 entrySelector
                // 12..14 rangeShift

                if seg_count == 0 {
                    return Err(ImtError {
                        kind: ImtErrorKind::Malformed,
                        source: ImtErrorSource::CmapSubtable,
                    });
                }

                if base_offset + 16 + (seg_count * 8) > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::CmapSubtable,
                    });
                }

                #[derive(Debug)]
                struct Segment {
                    end_code: u16,
                    start_code: u16,
                    id_delta: i16,
                    id_range_offset: u16,
                }

                let end_code_offset = base_offset + 14;
                let start_code_offset = end_code_offset + (seg_count * 2) + 2;
                let id_delta_offset = start_code_offset + (seg_count * 2);
                let id_range_offset_offset = id_delta_offset + (seg_count * 2);
                let mut segments = Vec::with_capacity(seg_count);

                for i in 0..seg_count {
                    let end_code = u16::from_be_bytes(
                        bytes[(end_code_offset + (i * 2))..(end_code_offset + 2 + (i * 2))]
                            .try_into()
                            .unwrap(),
                    );
                    let start_code = u16::from_be_bytes(
                        bytes[(start_code_offset + (i * 2))..(start_code_offset + 2 + (i * 2))]
                            .try_into()
                            .unwrap(),
                    );
                    let id_delta = i16::from_be_bytes(
                        bytes[(id_delta_offset + (i * 2))..(id_delta_offset + 2 + (i * 2))]
                            .try_into()
                            .unwrap(),
                    );
                    let id_range_offset = u16::from_be_bytes(
                        bytes[(id_range_offset_offset + (i * 2))
                            ..(id_range_offset_offset + 2 + (i * 2))]
                            .try_into()
                            .unwrap(),
                    );

                    segments.push(Segment {
                        end_code,
                        start_code,
                        id_delta,
                        id_range_offset,
                    });
                }

                match segments.pop() {
                    Some(last_segment) => {
                        if last_segment.start_code != 0xFFFF || last_segment.end_code != 0xFFFF {
                            return Err(ImtError {
                                kind: ImtErrorKind::Malformed,
                                source: ImtErrorSource::CmapSubtable,
                            });
                        }
                    },
                    None => {
                        return Err(ImtError {
                            kind: ImtErrorKind::Malformed,
                            source: ImtErrorSource::CmapSubtable,
                        })
                    },
                }

                let mut glyph_id_map = BTreeMap::new();
                let mut previous_code = 0;

                for i in 0..segments.len() {
                    let mut s = segments[i].start_code;
                    let e = segments[i].end_code;

                    if s > e {
                        return Err(ImtError {
                            kind: ImtErrorKind::Malformed,
                            source: ImtErrorSource::CmapSubtable,
                        });
                    }

                    if s <= previous_code {
                        s = previous_code + 1;
                    }

                    if s > e {
                        continue;
                    }

                    for code in s..=e {
                        if segments[i].id_range_offset == 0 {
                            let glyph_id =
                                ((code as i32 + segments[i].id_delta as i32) & 0xFFFF) as u16;
                            glyph_id_map.insert(code, glyph_id);
                        } else {
                            // NOTE: This is magic
                            let glyph_id_offset = 2
                                + id_range_offset_offset
                                + ((i
                                    + (((code - segments[i].start_code)
                                        + segments[i].id_range_offset)
                                        as usize
                                        / 2))
                                    * 2);

                            if glyph_id_offset + 2 > bytes.len() {
                                return Err(ImtError {
                                    kind: ImtErrorKind::Malformed,
                                    source: ImtErrorSource::CmapSubtable,
                                });
                            }

                            let glyph_id_value = u16::from_be_bytes(
                                bytes[glyph_id_offset..(glyph_id_offset + 2)]
                                    .try_into()
                                    .unwrap(),
                            );

                            let glyph_id = ((glyph_id_value as i32 + segments[i].id_delta as i32)
                                & 0xFFFF) as u16;
                            glyph_id_map.insert(code, glyph_id);
                        }
                    }

                    previous_code = segments[i].end_code;
                }

                Ok(CmapSubtable {
                    language,
                    glyph_id_map,
                })
            },
            _ => {
                Err(ImtError {
                    kind: ImtErrorKind::FormatNotSupported,
                    source: ImtErrorSource::CmapSubtable,
                })
            },
        }
    }
}
