//! This `mod` contains the raw parsed data of a font file.

use crate::error::*;

pub mod cmap_table;
pub mod font;
pub mod glyf_table;
pub mod head_table;
pub mod hhea_table;
pub mod hmtx_table;
pub mod loca_table;
pub mod maxp_table;
pub mod table_directory;
pub mod ttc_header;

pub use cmap_table::{CmapSubtable, CmapTable, EncodingRecord};
pub use font::Font;
pub use glyf_table::GlyfTable;
pub use head_table::HeadTable;
pub use hhea_table::HheaTable;
pub use hmtx_table::HmtxTable;
pub use loca_table::LocaTable;
pub use maxp_table::MaxpTable;
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
}

#[allow(warnings)]
pub fn test() {
    let bytes = include_bytes!("../RobotoFlex.ttf");

    let start = std::time::Instant::now();
    let font = Font::from_bytes(bytes).unwrap();
    println!(
        "Elapsed: {:.3} ms",
        start.elapsed().as_micros() as f32 / 1000.0
    );
}
