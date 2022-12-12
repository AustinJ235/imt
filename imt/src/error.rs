#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImtError {
    pub kind: ImtErrorKind,
    pub source: ImtErrorSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImtErrorKind {
    UnexpectedTag,
    Truncated,
    CFFNotSupported,
    InvalidSfntVersion,
    FormatNotSupported,
    Malformed,
    UnexpectedVersion,
    CollectionNotSupported,
    MissingTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImtErrorSource {
    TTCHeader,
    TableDirectory,
    TableRecord,
    HeadTable,
    CmapTable,
    EncodingRecord,
    CmapSubtable,
    HheaTable,
    MaxpTable,
    HmtxTable,
    LocaTable,
    GlyfTable,
    FontData,
    FvarTable,
    NameTable,
    NameRecord,
    NameTagRecord,
    GvarTable,
}
