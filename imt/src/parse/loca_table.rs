use crate::error::*;
use crate::parse::{read_u16, read_u32, HeadTable, MaxpTable};

#[derive(Debug, Clone)]
pub struct LocaTable {
    pub offsets: Vec<u32>,
}

impl LocaTable {
    pub fn try_parse(
        bytes: &[u8],
        table_offset: usize,
        head_table: &HeadTable,
        maxp_table: &MaxpTable,
    ) -> Result<Self, ImtError> {
        let num_glyphs = maxp_table.num_glyphs as usize;

        match head_table.index_to_loc_format {
            0 => {
                if table_offset + ((num_glyphs + 1) * 2) > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::LocaTable,
                    });
                }

                let mut offsets = Vec::with_capacity(num_glyphs + 1);

                for i in 0..=num_glyphs {
                    offsets.push(read_u16(bytes, table_offset + (i * 2)) as u32 * 2);
                }

                Ok(Self {
                    offsets,
                })
            },
            1 => {
                if table_offset + ((num_glyphs + 1) * 4) > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::LocaTable,
                    });
                }

                let mut offsets = Vec::with_capacity(num_glyphs + 1);

                for i in 0..=num_glyphs {
                    offsets.push(read_u32(bytes, table_offset + (i * 4)));
                }

                Ok(Self {
                    offsets,
                })
            },
            _ => {
                Err(ImtError {
                    kind: ImtErrorKind::FormatNotSupported,
                    source: ImtErrorSource::LocaTable,
                })
            },
        }
    }
}
