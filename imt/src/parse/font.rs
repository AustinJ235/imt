use crate::parse::*;

// TODO: Tables not currently parsed in RobotoFlex: GDEF, GPOS, GSUB, HVAR, OS/2, STAT, avar,
//       gasp, gvar, name, post, prep

#[derive(Debug, Clone)]
pub struct Font {
    cmap: CmapTable,
    head: HeadTable,
    hhea: HheaTable,
    hmtx: HmtxTable,
    maxp: MaxpTable,
    name: NameTable,
    glyf: GlyfTable,
    fvar: Option<FvarTable>,
    gvar: Option<GvarTable>,
}

impl Font {
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Result<Self, ImtError> {
        let bytes = bytes.as_ref();

        match TTCHeader::try_parse(bytes) {
            Err(ImtError {
                kind: ImtErrorKind::UnexpectedTag,
                ..
            }) => (),
            _ => {
                return Err(ImtError {
                    kind: ImtErrorKind::CollectionNotSupported,
                    source: ImtErrorSource::FontData,
                })
            },
        }

        let table_directory = TableDirectory::try_parse(bytes, 0)?;

        // TODO: Verify Table Checksums

        let mut cmap_table_index = None;
        let mut head_table_index = None;
        let mut hhea_table_index = None;
        let mut hmtx_table_index = None;
        let mut maxp_table_index = None;
        let mut name_table_index = None;
        let mut loca_table_index = None;
        let mut glyf_table_index = None;
        let mut fvar_table_index = None;
        let mut gvar_table_index = None;

        for (i, table_record) in table_directory.table_records.iter().enumerate() {
            match table_record.table_tag {
                table_tag::CMAP => cmap_table_index = Some(i),
                table_tag::HEAD => head_table_index = Some(i),
                table_tag::HHEA => hhea_table_index = Some(i),
                table_tag::HMTX => hmtx_table_index = Some(i),
                table_tag::MAXP => maxp_table_index = Some(i),
                table_tag::LOCA => loca_table_index = Some(i),
                table_tag::GLYF => glyf_table_index = Some(i),
                table_tag::FVAR => fvar_table_index = Some(i),
                table_tag::NAME => name_table_index = Some(i),
                table_tag::GVAR => gvar_table_index = Some(i),
                _ => (),
            }
        }

        let cmap = match cmap_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::CmapTable,
                    });
                }

                CmapTable::try_parse(&bytes[start..end], 0)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::CmapTable,
                })
            },
        };

        let head = match head_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::HeadTable,
                    });
                }

                HeadTable::try_parse(&bytes[start..end], 0)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::HeadTable,
                })
            },
        };

        let hhea = match hhea_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::HheaTable,
                    });
                }

                HheaTable::try_parse(&bytes[start..end], 0)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::HheaTable,
                })
            },
        };

        let maxp = match maxp_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::MaxpTable,
                    });
                }

                MaxpTable::try_parse(&bytes[start..end], 0)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::MaxpTable,
                })
            },
        };

        let name = match name_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::NameTable,
                    });
                }

                NameTable::try_parse(&bytes[start..end], 0)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::NameTable,
                })
            },
        };

        let hmtx = match hmtx_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::HmtxTable,
                    });
                }

                HmtxTable::try_parse(&bytes[start..end], 0, &maxp, &hhea)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::HmtxTable,
                })
            },
        };

        let loca = match loca_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::LocaTable,
                    });
                }

                LocaTable::try_parse(&bytes[start..end], 0, &head, &maxp)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::LocaTable,
                })
            },
        };

        let glyf = match glyf_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::GlyfTable,
                    });
                }

                GlyfTable::try_parse(&bytes[start..end], 0, &loca)?
            },
            None => {
                return Err(ImtError {
                    kind: ImtErrorKind::MissingTable,
                    source: ImtErrorSource::GlyfTable,
                })
            },
        };

        let fvar = match fvar_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::FvarTable,
                    });
                }

                Some(FvarTable::try_parse(&bytes[start..end], 0)?)
            },
            None => None,
        };

        let gvar = match gvar_table_index {
            Some(table_index) => {
                let table_record = &table_directory.table_records[table_index];
                let start = table_record.offset as usize;
                let end = start + table_record.length as usize;

                if end > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::GvarTable,
                    });
                }

                Some(GvarTable::try_parse(&bytes[start..end], 0, &glyf)?)
            },
            None => None,
        };

        Ok(Self {
            cmap,
            head,
            hhea,
            hmtx,
            maxp,
            name,
            glyf,
            fvar,
            gvar,
        })
    }

    pub fn cmap_table(&self) -> &CmapTable {
        &self.cmap
    }

    pub fn head_table(&self) -> &HeadTable {
        &self.head
    }

    pub fn hhea_table(&self) -> &HheaTable {
        &self.hhea
    }

    pub fn hmtx_table(&self) -> &HmtxTable {
        &self.hmtx
    }

    pub fn maxp_table(&self) -> &MaxpTable {
        &self.maxp
    }

    pub fn name_table(&self) -> &NameTable {
        &self.name
    }

    pub fn glyf_table(&self) -> &GlyfTable {
        &self.glyf
    }

    pub fn fvar_table(&self) -> Option<&FvarTable> {
        self.fvar.as_ref()
    }

    pub fn gvar_table(&self) -> Option<&GvarTable> {
        self.gvar.as_ref()
    }
}
