use crate::error::*;
use crate::parse::read_u16;

/// Corresponds to the `maxp` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/maxp>
#[derive(Debug, Clone)]
pub struct MaxpTable {
    pub version: u32,
    pub num_glyphs: u16,
    pub max_points: u16,
    pub max_countours: u16,
    pub max_composite_points: u16,
    pub max_composite_contours: u16,
    pub max_zones: u16,
    pub max_twilight_points: u16,
    pub max_storage: u16,
    pub max_function_defs: u16,
    pub max_instruction_defs: u16,
    pub max_stack_elements: u16,
    pub max_size_of_instructions: u16,
    pub max_component_elements: u16,
    pub max_component_depth: u16,
}

impl MaxpTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 6 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::MaxpTable,
            });
        }

        let version =
            u32::from_be_bytes(bytes[table_offset..(table_offset + 4)].try_into().unwrap());
        let num_glyphs = read_u16(bytes, table_offset + 4);

        match version {
            0x00005000 => {
                Ok(Self {
                    version,
                    num_glyphs,
                    max_points: 0,
                    max_countours: 0,
                    max_composite_points: 0,
                    max_composite_contours: 0,
                    max_zones: 0,
                    max_twilight_points: 0,
                    max_storage: 0,
                    max_function_defs: 0,
                    max_instruction_defs: 0,
                    max_stack_elements: 0,
                    max_size_of_instructions: 0,
                    max_component_elements: 0,
                    max_component_depth: 0,
                })
            },
            0x00010000 => {
                if table_offset + 32 > bytes.len() {
                    Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::MaxpTable,
                    })
                } else {
                    Ok(Self {
                        version,
                        num_glyphs,
                        max_points: read_u16(bytes, table_offset + 6),
                        max_countours: read_u16(bytes, table_offset + 8),
                        max_composite_points: read_u16(bytes, table_offset + 10),
                        max_composite_contours: read_u16(bytes, table_offset + 12),
                        max_zones: read_u16(bytes, table_offset + 14),
                        max_twilight_points: read_u16(bytes, table_offset + 16),
                        max_storage: read_u16(bytes, table_offset + 18),
                        max_function_defs: read_u16(bytes, table_offset + 20),
                        max_instruction_defs: read_u16(bytes, table_offset + 22),
                        max_stack_elements: read_u16(bytes, table_offset + 24),
                        max_size_of_instructions: read_u16(bytes, table_offset + 26),
                        max_component_elements: read_u16(bytes, table_offset + 28),
                        max_component_depth: read_u16(bytes, table_offset + 30),
                    })
                }
            },
            _ => {
                Err(ImtError {
                    kind: ImtErrorKind::UnexpectedVersion,
                    source: ImtErrorSource::MaxpTable,
                })
            },
        }
    }
}
