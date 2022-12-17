use crate::error::*;
use crate::parse::{read_f2dot14, read_i16, read_i32, read_i8, read_u16, read_u32};

const TRUNCATED: ImtError = ImtError {
    kind: ImtErrorKind::Truncated,
    source: ImtErrorSource::HvarTable,
};

const MALFORMED: ImtError = ImtError {
    kind: ImtErrorKind::Malformed,
    source: ImtErrorSource::HvarTable,
};

/// Corresponds to the `hvar` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/hvar>
#[derive(Debug, Clone)]
pub struct HvarTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub item_variation_store: ItemVariationStore,
    pub advance_map: Option<DeltaSetIndexMap>,
    pub lsb_map: Option<DeltaSetIndexMap>,
    pub rsb_map: Option<DeltaSetIndexMap>,
}

impl HvarTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        // Read Header

        if table_offset + 20 > bytes.len() {
            return Err(TRUNCATED);
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::HvarTable,
            });
        }

        let var_store_offset = read_u32(bytes, table_offset + 4) as usize + table_offset;

        let adv_mapping_offset = match read_u32(bytes, table_offset + 8) {
            0 => None,
            offset => Some(offset as usize + table_offset),
        };

        let lsb_mapping_offset = match read_u32(bytes, table_offset + 12) {
            0 => None,
            offset => Some(offset as usize + table_offset),
        };

        let rsb_mapping_offset = match read_u32(bytes, table_offset + 16) {
            0 => None,
            offset => Some(offset as usize + table_offset),
        };

        // Parse variation table and delta index maps.

        let item_variation_store = ItemVariationStore::try_parse(bytes, var_store_offset)?;

        let advance_map = match adv_mapping_offset {
            Some(offset) => Some(DeltaSetIndexMap::try_parse(bytes, offset)?),
            None => None,
        };

        let lsb_map = match lsb_mapping_offset {
            Some(offset) => Some(DeltaSetIndexMap::try_parse(bytes, offset)?),
            None => None,
        };

        let rsb_map = match rsb_mapping_offset {
            Some(offset) => Some(DeltaSetIndexMap::try_parse(bytes, offset)?),
            None => None,
        };

        Ok(Self {
            major_version,
            minor_version,
            item_variation_store,
            advance_map,
            lsb_map,
            rsb_map,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ItemVariationStore {
    pub axis_count: usize,
    pub regions: Vec<VariationRegion>,
    pub item_data: Vec<ItemVariationData>,
}

#[derive(Debug, Clone)]
pub struct VariationRegion {
    /// Length equal to that of `axis_count` of `ItemVariationStore`.
    pub axes: Vec<RegionAxisCoordinates>,
}

#[derive(Debug, Clone)]
pub struct RegionAxisCoordinates {
    pub start: f32,
    pub peak: f32,
    pub end: f32,
}

#[derive(Debug, Clone)]
pub struct ItemVariationData {
    pub region_indexes: Vec<usize>,
    pub delta_sets: Vec<DeltaSet>,
}

#[derive(Debug, Clone)]
pub struct DeltaSet {
    /// Length equal to `region_indexes` of `ItemVariationData`
    pub data: Vec<DeltaData>,
}

#[derive(Debug, Clone, Copy)]
pub enum DeltaData {
    I8(i8),
    I16(i16),
    I32(i32),
}

impl DeltaData {
    pub fn as_f32(&self) -> f32 {
        match self {
            Self::I8(v) => *v as f32,
            Self::I16(v) => *v as f32,
            Self::I32(v) => *v as f32,
        }
    }
}

impl ItemVariationStore {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        // Read ItemVariationStore

        if table_offset + 8 > bytes.len() {
            return Err(TRUNCATED);
        }

        let format = read_u16(bytes, table_offset);
        let region_list_offset = read_u32(bytes, table_offset + 2) as usize + table_offset;
        let item_data_count = read_u16(bytes, table_offset + 6) as usize;

        if format != 1 {
            return Err(MALFORMED);
        }

        if table_offset + 8 + (item_data_count * 4) > bytes.len() {
            return Err(TRUNCATED);
        }

        let mut item_data_offsets = Vec::with_capacity(item_data_count);

        for i in 0..item_data_count {
            item_data_offsets
                .push(read_u32(bytes, table_offset + 8 + (i * 4)) as usize + table_offset);
        }

        // Read VariationRegionList

        if region_list_offset + 4 > bytes.len() {
            return Err(TRUNCATED);
        }

        let axis_count = read_u16(bytes, region_list_offset) as usize;
        let region_count = read_u16(bytes, region_list_offset + 2) as usize;

        // Read VariationRegion's

        if region_list_offset + 4 + (region_count * axis_count * 6) > bytes.len() {
            return Err(TRUNCATED);
        }

        let mut regions = Vec::with_capacity(region_count);

        for i in 0..region_count {
            let region_offset = region_list_offset + 4 + (i * axis_count * 6);
            let mut axes = Vec::with_capacity(axis_count);

            for j in 0..axis_count {
                let axis_offset = region_offset + (j * 6);

                axes.push(RegionAxisCoordinates {
                    start: read_f2dot14(bytes, axis_offset),
                    peak: read_f2dot14(bytes, axis_offset + 2),
                    end: read_f2dot14(bytes, axis_offset + 4),
                });
            }

            regions.push(VariationRegion {
                axes,
            });
        }

        // VariationRegion's Sanity Checks

        for region in regions.iter() {
            for axis in region.axes.iter() {
                // The three values must all be within the range -1.0 to +1.0. startCoord must be
                // less than or equal to peakCoord, and peakCoord must be less than or equal to
                // endCoord. The three values must be either all non-positive or all non-negative
                // with one possible exception: if peakCoord is zero, then startCoord can be
                // negative or 0 while endCoord can be positive or zero.

                if axis.start < -1.0
                    || axis.start > 1.0
                    || axis.peak < -1.0
                    || axis.peak > 1.0
                    || axis.end < -1.0
                    || axis.end > 1.0
                    || axis.start > axis.peak
                    || axis.end < axis.peak
                {
                    return Err(MALFORMED);
                }

                if axis.peak == 0.0 {
                    if axis.start > 0.0 || axis.end < 0.0 {
                        return Err(MALFORMED);
                    }
                } else if axis.peak < 0.0 {
                    if axis.start > 0.0 || axis.end > 0.0 {
                        return Err(MALFORMED);
                    }
                } else {
                    if axis.start < 0.0 || axis.end < 0.0 {
                        return Err(MALFORMED);
                    }
                }
            }
        }

        // Read ItemVariationData's

        let mut item_data = Vec::with_capacity(item_data_offsets.len());

        for item_data_offset in item_data_offsets {
            if item_data_offset + 6 > bytes.len() {
                return Err(TRUNCATED);
            }

            let item_count = read_u16(bytes, item_data_offset) as usize;

            let (long_words, word_delta_count) = {
                let world_delta_count = read_u16(bytes, item_data_offset + 2);

                (
                    world_delta_count & 0x8000 != 0,
                    (world_delta_count & 0x7FFF) as usize,
                )
            };

            let region_index_count = read_u16(bytes, item_data_offset + 4) as usize;

            if word_delta_count > region_index_count {
                return Err(MALFORMED);
            }

            if item_data_offset + 6 + (region_index_count * 2) > bytes.len() {
                return Err(TRUNCATED);
            }

            let mut region_indexes = Vec::with_capacity(region_index_count);

            for i in 0..region_index_count {
                region_indexes.push(read_u16(bytes, item_data_offset + 6 + (i * 2)) as usize);
            }

            // TODO: Is it valid to have an index greater than regions?
            for index in region_indexes.iter() {
                if *index >= regions.len() {
                    return Err(MALFORMED);
                }
            }

            // Read DeltaSet's

            let mut delta_sets_offset = item_data_offset + 6 + (region_index_count * 2);
            let mut delta_sets = Vec::with_capacity(item_count);

            for _ in 0..item_count {
                let mut data = Vec::with_capacity(region_index_count);

                for i in 0..region_index_count {
                    if i < word_delta_count {
                        if long_words {
                            if delta_sets_offset + 4 > bytes.len() {
                                return Err(TRUNCATED);
                            }

                            data.push(DeltaData::I32(read_i32(bytes, delta_sets_offset)));
                            delta_sets_offset += 4;
                        } else {
                            if delta_sets_offset + 2 > bytes.len() {
                                return Err(TRUNCATED);
                            }

                            data.push(DeltaData::I16(read_i16(bytes, delta_sets_offset)));
                            delta_sets_offset += 2;
                        }
                    } else {
                        if long_words {
                            if delta_sets_offset + 2 > bytes.len() {
                                return Err(TRUNCATED);
                            }

                            data.push(DeltaData::I16(read_i16(bytes, delta_sets_offset)));
                            delta_sets_offset += 2;
                        } else {
                            if delta_sets_offset + 1 > bytes.len() {
                                return Err(TRUNCATED);
                            }

                            data.push(DeltaData::I8(read_i8(bytes, delta_sets_offset)));
                            delta_sets_offset += 1;
                        }
                    }
                }

                delta_sets.push(DeltaSet {
                    data,
                });
            }

            item_data.push(ItemVariationData {
                region_indexes,
                delta_sets,
            });
        }

        Ok(Self {
            axis_count,
            regions,
            item_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeltaSetIndexMap {
    pub map_data: Vec<[usize; 2]>,
}

impl DeltaSetIndexMap {
    pub fn try_parse(bytes: &[u8], map_offset: usize) -> Result<Self, ImtError> {
        if map_offset + 2 > bytes.len() {
            return Err(TRUNCATED);
        }

        let format = bytes[map_offset];
        let entry_format = bytes[map_offset + 1];

        let (map_count, mut map_data_offset) = match format {
            0 => {
                if map_offset + 4 > bytes.len() {
                    return Err(TRUNCATED);
                }

                (read_u16(bytes, map_offset + 2) as usize, 4)
            },
            1 => {
                if map_offset + 6 > bytes.len() {
                    return Err(TRUNCATED);
                }

                (read_u32(bytes, map_offset + 2) as usize, 6)
            },
            _ => return Err(MALFORMED),
        };

        let entry_size = (((entry_format & 0x30) >> 4) + 1) as usize;

        // NOTE: Although the spec makes it seem it is possible to use more than 4 bytes, it
        //       wouldn't make any sense since itemVariationDataCount is 2 bytes and itemCount
        //       is also 2 bytes; therefore, return malformed.

        if entry_size == 0 || entry_size > 4 {
            return Err(MALFORMED);
        }

        if map_data_offset + (map_count * entry_size) > bytes.len() {
            return Err(TRUNCATED);
        }

        let inner_index_bit_count = (entry_format & 0x0F) + 1;
        let mut map_data = Vec::with_capacity(map_count);

        for _ in 0..map_count {
            let mut v = 0_u32;

            for i in 0..entry_size {
                v = (v << 8) + bytes[map_data_offset + i] as u32;
            }

            map_data_offset += entry_size;

            map_data.push([
                (v >> inner_index_bit_count) as usize,
                (v & ((1 << inner_index_bit_count) - 1)) as usize,
            ]);
        }

        Ok(Self {
            map_data,
        })
    }
}
