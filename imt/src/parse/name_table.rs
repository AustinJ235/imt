use crate::error::*;
use crate::parse::{read_u16, read_utf16be};

/// Corresponds to the `name` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/name>
#[derive(Debug, Clone)]
pub struct NameTable {
    pub version: u16,
    pub name_records: Vec<NameRecord>,
    pub lang_tag_records: Vec<LangTagRecord>,
}

impl NameTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 6 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::NameTable,
            });
        }

        let version = read_u16(bytes, table_offset);

        if version != 0 && version != 1 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::NameTable,
            });
        }

        let name_count = read_u16(bytes, table_offset + 2) as usize;
        let storage_offset = read_u16(bytes, table_offset + 4) as usize + table_offset;
        let mut record_offset = table_offset + 6;

        if record_offset + (name_count * 12) > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::NameTable,
            });
        }

        let mut name_records = Vec::with_capacity(name_count);

        for _ in 0..name_count {
            name_records.push(NameRecord::try_parse(bytes, record_offset, storage_offset)?);

            record_offset += 12;
        }

        let lang_tag_records = if version == 1 {
            if record_offset + 2 > bytes.len() {
                return Err(ImtError {
                    kind: ImtErrorKind::Truncated,
                    source: ImtErrorSource::NameTable,
                });
            }

            let lang_tag_count = read_u16(bytes, record_offset) as usize;
            record_offset += 2;

            if record_offset + (lang_tag_count * 4) > bytes.len() {
                return Err(ImtError {
                    kind: ImtErrorKind::Truncated,
                    source: ImtErrorSource::NameTable,
                });
            }

            let mut lang_tag_records = Vec::with_capacity(lang_tag_count);

            for _ in 0..lang_tag_count {
                lang_tag_records.push(LangTagRecord::try_parse(
                    bytes,
                    record_offset,
                    storage_offset,
                )?);

                record_offset += 4;
            }

            lang_tag_records
        } else {
            Vec::new()
        };

        Ok(Self {
            version,
            name_records,
            lang_tag_records,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NameRecord {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    pub name_id: u16,
    pub name: String,
}

impl NameRecord {
    pub fn try_parse(
        bytes: &[u8],
        record_offset: usize,
        storage_offset: usize,
    ) -> Result<Self, ImtError> {
        if record_offset + 12 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::NameRecord,
            });
        }

        let platform_id = read_u16(bytes, record_offset);
        let encoding_id = read_u16(bytes, record_offset + 2);
        let language_id = read_u16(bytes, record_offset + 4);
        let name_id = read_u16(bytes, record_offset + 6);
        let length = read_u16(bytes, record_offset + 8) as usize;
        let string_offset = read_u16(bytes, record_offset + 10) as usize + storage_offset;
        let name = read_utf16be(bytes, string_offset, length, ImtErrorSource::NameRecord)?;

        Ok(Self {
            platform_id,
            encoding_id,
            language_id,
            name_id,
            name,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LangTagRecord(pub String);

impl LangTagRecord {
    pub fn try_parse(
        bytes: &[u8],
        record_offset: usize,
        storage_offset: usize,
    ) -> Result<Self, ImtError> {
        if record_offset + 4 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::NameTagRecord,
            });
        }

        let length = read_u16(bytes, record_offset) as usize;
        let lang_tag_offset = read_u16(bytes, record_offset + 2) as usize + storage_offset;

        Ok(Self(read_utf16be(
            bytes,
            lang_tag_offset,
            length,
            ImtErrorSource::NameTagRecord,
        )?))
    }
}
