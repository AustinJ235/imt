//! This `mod` contains the raw parsed data of a font file.

use crate::error::*;

pub mod avar_table;
pub mod cmap_table;
pub mod font;
pub mod fvar_table;
pub mod glyf_table;
pub mod gvar_table;
pub mod head_table;
pub mod hhea_table;
pub mod hmtx_table;
pub mod loca_table;
pub mod maxp_table;
pub mod name_table;
pub mod table_directory;
pub mod ttc_header;

pub use avar_table::{AvarTable, AxisValueMap, SegmentMap};
pub use cmap_table::{CmapSubtable, CmapTable, EncodingRecord};
pub use font::Font;
pub use fvar_table::{FvarTable, InstanceRecord, VariationAxisRecord};
pub use glyf_table::GlyfTable;
pub use gvar_table::GvarTable;
pub use head_table::HeadTable;
pub use hhea_table::HheaTable;
pub use hmtx_table::HmtxTable;
pub use loca_table::LocaTable;
pub use maxp_table::MaxpTable;
pub use name_table::{LangTagRecord, NameRecord, NameTable};
pub use table_directory::{TableDirectory, TableRecord};
pub use ttc_header::TTCHeader;

#[inline(always)]
fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(bytes[offset..(offset + 2)].try_into().unwrap())
}

#[inline(always)]
fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes(bytes[offset..(offset + 2)].try_into().unwrap())
}

#[inline(always)]
fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..(offset + 4)].try_into().unwrap())
}

#[inline(always)]
fn read_i64(bytes: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(bytes[offset..(offset + 8)].try_into().unwrap())
}

#[inline(always)]
fn read_fixed(bytes: &[u8], offset: usize) -> f32 {
    i32::from_be_bytes(bytes[offset..(offset + 4)].try_into().unwrap()) as f32 / 65536.0
}

#[inline(always)]
fn read_f2dot14(bytes: &[u8], offset: usize) -> f32 {
    i16::from_be_bytes(bytes[offset..(offset + 2)].try_into().unwrap()) as f32 / 16384.0
}

fn read_utf16be(
    bytes: &[u8],
    offset: usize,
    length: usize,
    source: ImtErrorSource,
) -> Result<String, ImtError> {
    if length % 2 != 0 {
        return Err(ImtError {
            kind: ImtErrorKind::Malformed,
            source,
        });
    }

    if offset + length > bytes.len() {
        return Err(ImtError {
            kind: ImtErrorKind::Truncated,
            source,
        });
    }

    let utf16 = bytes[offset..(offset + length)]
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<u16>>();

    String::from_utf16(&utf16).map_err(|_| {
        ImtError {
            kind: ImtErrorKind::Malformed,
            source,
        }
    })
}

const fn tag(bytes: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*bytes)
}

pub mod table_tag {
    use super::tag;
    pub const CMAP: u32 = tag(b"cmap");
    pub const HEAD: u32 = tag(b"head");
    pub const HHEA: u32 = tag(b"hhea");
    pub const HMTX: u32 = tag(b"hmtx");
    pub const MAXP: u32 = tag(b"maxp");
    pub const LOCA: u32 = tag(b"loca");
    pub const GLYF: u32 = tag(b"glyf");
    pub const FVAR: u32 = tag(b"fvar");
    pub const NAME: u32 = tag(b"name");
    pub const GVAR: u32 = tag(b"gvar");
    pub const AVAR: u32 = tag(b"avar");
}
