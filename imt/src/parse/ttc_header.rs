use crate::error::*;
use crate::parse::tag;

/// Corresponds to the *"TTC Header"*
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/otff>
/// # Notes
/// - If the version is 1.0 `dsig_tag`, `dsig_length`, `dsig_offset` will *always* be zero.
#[derive(Debug, Clone)]
pub struct TTCHeader {
    pub ttc_tag: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub num_fonts: u32,
    pub table_directory_offsets: Vec<u32>,
    pub dsig_tag: u32,
    pub dsig_length: u32,
    pub dsig_offset: u32,
}

impl TTCHeader {
    pub fn try_parse(bytes: &[u8]) -> Result<Self, ImtError> {
        let ttc_tag = u32::from_be_bytes(bytes[0..4].try_into().unwrap());

        if tag(b"ttcf") != ttc_tag {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedTag,
                source: ImtErrorSource::TTCHeader,
            });
        }

        if bytes.len() < 12 {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::TTCHeader,
            });
        }

        let major_version = u16::from_be_bytes(bytes[4..6].try_into().unwrap());
        let minor_version = u16::from_be_bytes(bytes[6..8].try_into().unwrap());
        let num_fonts = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let table_directory_offsets_end = 12 + (num_fonts as usize * 4);

        dbg!(ttc_tag, major_version, minor_version, num_fonts);

        if bytes.len() < table_directory_offsets_end {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::TTCHeader,
            });
        }

        let mut table_directory_offsets: Vec<u32> = Vec::with_capacity(num_fonts as usize);

        for chunk in bytes[12..table_directory_offsets_end].chunks_exact(4) {
            table_directory_offsets.push(u32::from_be_bytes(chunk.try_into().unwrap()));
        }

        if major_version == 2 {
            if bytes.len() < table_directory_offsets_end + 12 {
                return Err(ImtError {
                    kind: ImtErrorKind::Truncated,
                    source: ImtErrorSource::TTCHeader,
                });
            }

            let dsig_tag = u32::from_be_bytes(
                bytes[table_directory_offsets_end..(table_directory_offsets_end + 4)]
                    .try_into()
                    .unwrap(),
            );
            let dsig_length = u32::from_be_bytes(
                bytes[(table_directory_offsets_end + 4)..(table_directory_offsets_end + 8)]
                    .try_into()
                    .unwrap(),
            );
            let dsig_offset = u32::from_be_bytes(
                bytes[(table_directory_offsets_end + 8)..(table_directory_offsets_end + 12)]
                    .try_into()
                    .unwrap(),
            );

            Ok(Self {
                ttc_tag,
                major_version,
                minor_version,
                num_fonts,
                table_directory_offsets,
                dsig_tag,
                dsig_length,
                dsig_offset,
            })
        } else {
            Ok(Self {
                ttc_tag,
                major_version,
                minor_version,
                num_fonts,
                table_directory_offsets,
                dsig_tag: 0,
                dsig_length: 0,
                dsig_offset: 0,
            })
        }
    }
}
