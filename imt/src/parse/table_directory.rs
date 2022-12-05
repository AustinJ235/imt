use crate::error::*;
use crate::parse::tag;

/// Corresponds to the *"Table Directory"*
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/otff>
#[derive(Debug, Clone)]
pub struct TableDirectory {
    pub sfnt_version: u32,
    pub table_records: Vec<TableRecord>,
}

impl TableDirectory {
    pub fn try_parse(bytes: &[u8], base_offset: usize) -> Result<Self, ImtError> {
        if bytes.len() < base_offset + 12 {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::TableDirectory,
            });
        }

        let sfnt_version =
            u32::from_be_bytes(bytes[base_offset..(base_offset + 4)].try_into().unwrap());

        if sfnt_version == tag(b"OTTO") {
            return Err(ImtError {
                kind: ImtErrorKind::CFFNotSupported,
                source: ImtErrorSource::TableDirectory,
            });
        }

        if sfnt_version != 65536 {
            return Err(ImtError {
                kind: ImtErrorKind::InvalidSfntVersion,
                source: ImtErrorSource::TableDirectory,
            });
        }

        let num_tables = u16::from_be_bytes(
            bytes[(base_offset + 4)..(base_offset + 6)]
                .try_into()
                .unwrap(),
        );
        // 6..8 searchRange
        // 8..10 entrySelector
        // 10..12 rangeShift

        if (base_offset + 12) + (num_tables as usize * 16) > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::TableDirectory,
            });
        }

        let mut table_records = Vec::with_capacity(num_tables as usize);

        for table_i in 0..(num_tables as usize) {
            table_records.push(TableRecord::try_parse(
                bytes,
                base_offset + 12 + (table_i * 16),
            )?);
        }

        Ok(Self {
            sfnt_version,
            table_records,
        })
    }
}

/// Corresponds to the *"Table Record"*
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/otff>
#[derive(Debug, Clone)]
pub struct TableRecord {
    pub table_tag: u32,
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
}

impl TableRecord {
    pub fn try_parse(bytes: &[u8], base_offset: usize) -> Result<Self, ImtError> {
        if bytes.len() < base_offset + 16 {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::TableRecord,
            });
        }

        let table_tag =
            u32::from_be_bytes(bytes[base_offset..(base_offset + 4)].try_into().unwrap());
        let checksum = u32::from_be_bytes(
            bytes[(base_offset + 4)..(base_offset + 8)]
                .try_into()
                .unwrap(),
        );
        let offset = u32::from_be_bytes(
            bytes[(base_offset + 8)..(base_offset + 12)]
                .try_into()
                .unwrap(),
        );
        let length = u32::from_be_bytes(
            bytes[(base_offset + 12)..(base_offset + 16)]
                .try_into()
                .unwrap(),
        );

        Ok(Self {
            table_tag,
            checksum,
            offset,
            length,
        })
    }
}
