use crate::error::*;
use crate::parse::{read_fixed, read_u16, read_u32};

/// Corresponds to the `fvar` table.
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/fvar>
#[derive(Debug, Clone)]
pub struct FvarTable {
    pub major_version: u16,
    pub minor_version: u16,
    pub axes: Vec<VariationAxisRecord>,
    pub instances: Vec<InstanceRecord>,
}

impl FvarTable {
    pub fn try_parse(bytes: &[u8], table_offset: usize) -> Result<Self, ImtError> {
        if table_offset + 16 > bytes.len() {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::FvarTable,
            });
        }

        let major_version = read_u16(bytes, table_offset);
        let minor_version = read_u16(bytes, table_offset + 2);

        if major_version != 1 || minor_version != 0 {
            return Err(ImtError {
                kind: ImtErrorKind::UnexpectedVersion,
                source: ImtErrorSource::FvarTable,
            });
        }

        let axes_array_offset = read_u16(bytes, table_offset + 4) as usize;

        if read_u16(bytes, table_offset + 6) != 2 {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::FvarTable,
            });
        }

        let axis_count = read_u16(bytes, table_offset + 8) as usize;
        let axis_size = read_u16(bytes, table_offset + 10) as usize;

        if axis_size != 20 {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::FvarTable,
            });
        }

        let instance_count = read_u16(bytes, table_offset + 12) as usize;
        let instance_size = read_u16(bytes, table_offset + 14) as usize;
        let size_without_ps_name = (axis_count * 4) + 4;
        let size_with_ps_name = (axis_count * 4) + 6;

        if instance_size != size_without_ps_name && instance_size != size_with_ps_name {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::FvarTable,
            });
        }

        let mut record_offset = table_offset + axes_array_offset;

        if record_offset + (axis_count * axis_size) + (instance_count * instance_size) > bytes.len()
        {
            return Err(ImtError {
                kind: ImtErrorKind::Truncated,
                source: ImtErrorSource::FvarTable,
            });
        }

        let mut axes = Vec::with_capacity(axis_count);

        for _ in 0..axis_count {
            axes.push(VariationAxisRecord::parse(bytes, record_offset));
            record_offset += axis_size;
        }

        let mut instances = Vec::with_capacity(axis_count);

        for _ in 0..instance_count {
            instances.push(InstanceRecord::parse(
                bytes,
                record_offset,
                axis_count,
                instance_size == size_with_ps_name,
            ));

            record_offset += instance_size;
        }

        Ok(Self {
            major_version,
            minor_version,
            axes,
            instances,
        })
    }
}

#[derive(Debug, Clone)]
pub struct VariationAxisRecord {
    pub axis_tag: u32,
    pub min_value: f32,
    pub default_value: f32,
    pub max_value: f32,
    pub flags: u16,
    pub axis_name_id: u16,
}

impl VariationAxisRecord {
    pub fn parse(bytes: &[u8], record_offset: usize) -> Self {
        Self {
            axis_tag: read_u32(bytes, record_offset),
            min_value: read_fixed(bytes, record_offset + 4),
            default_value: read_fixed(bytes, record_offset + 8),
            max_value: read_fixed(bytes, record_offset + 12),
            flags: read_u16(bytes, record_offset + 16),
            axis_name_id: read_u16(bytes, record_offset + 18),
        }
    }

    pub fn hidden_axis(&self) -> bool {
        self.flags & 0x0001 == 0x0001
    }
}

#[derive(Debug, Clone)]
pub struct InstanceRecord {
    pub sub_family_name_id: u16,
    pub flags: u16,
    pub coordinates: Vec<f32>,
    pub post_script_name_id: Option<u16>,
}

impl InstanceRecord {
    pub fn parse(
        bytes: &[u8],
        record_offset: usize,
        axis_count: usize,
        post_script_name: bool,
    ) -> Self {
        let sub_family_name_id = read_u16(bytes, record_offset);
        let flags = read_u16(bytes, record_offset + 2);

        let coordinates = (0..axis_count)
            .into_iter()
            .map(|i| read_fixed(bytes, record_offset + 4 + (i * 4)))
            .collect();

        let post_script_name_id = if post_script_name {
            Some(read_u16(bytes, record_offset + 4 + (axis_count * 4)))
        } else {
            None
        };

        Self {
            sub_family_name_id,
            flags,
            coordinates,
            post_script_name_id,
        }
    }
}
