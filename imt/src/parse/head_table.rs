use crate::error::*;
use crate::parse::{read_i16, read_i64, read_u16, read_u32};

/// Corresponds to the `head` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/head>
///
/// # Notes
/// - `font_revision` is not parsed correctly and is in bytes form.
#[derive(Debug, Clone)]
pub struct HeadTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub font_revision: [u8; 4],
    pub checksum_adjustment: u32,
    pub magic_number: u32,
    pub flags: u16,
    pub units_per_em: u16,
    pub created: i64,
    pub modified: i64,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub mac_style: u16,
    pub lowest_rec_ppem: u16,
    pub font_direction_hint: i16,
    pub index_to_loc_format: i16,
    pub glyph_data_format: i16,
}

impl HeadTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 54 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::HeadTable,
            });
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::HeadTable,
            });
        }

        let font_revision = bytes[(table_offset + 4)..(table_offset + 8)]
            .try_into()
            .unwrap();

        let checksum_adjustment = read_u32(bytes, table_offset + 8);
        let magic_number = read_u32(bytes, table_offset + 12);

        if magic_number != 0x5f0f3cf5 {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::HeadTable,
            });
        }

        let flags = read_u16(bytes, table_offset + 16);
        let units_per_em = read_u16(bytes, table_offset + 18);
        let created = read_i64(bytes, table_offset + 20);
        let modified = read_i64(bytes, table_offset + 28);
        let x_min = read_i16(bytes, table_offset + 36);
        let y_min = read_i16(bytes, table_offset + 38);
        let x_max = read_i16(bytes, table_offset + 40);
        let y_max = read_i16(bytes, table_offset + 42);
        let mac_style = read_u16(bytes, table_offset + 44);
        let lowest_rec_ppem = read_u16(bytes, table_offset + 46);
        let font_direction_hint = read_i16(bytes, table_offset + 48);
        let index_to_loc_format = read_i16(bytes, table_offset + 50);
        let glyph_data_format = read_i16(bytes, table_offset + 52);

        Ok(Self {
            major_version,
            minor_version,
            font_revision,
            checksum_adjustment,
            magic_number,
            flags,
            units_per_em,
            created,
            modified,
            x_min,
            y_min,
            x_max,
            y_max,
            mac_style,
            lowest_rec_ppem,
            font_direction_hint,
            index_to_loc_format,
            glyph_data_format,
        })
    }
}
