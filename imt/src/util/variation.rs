use crate::parse::{Font, Outline};
use crate::util::ImtUtilError;

pub fn normalize_axis_coords(font: &Font, coords: &mut Vec<f32>) -> Result<(), ImtUtilError> {
    let fvar = font.fvar_table().ok_or(ImtUtilError::MissingTable)?;

    if coords.len() != fvar.axes.len() {
        return Err(ImtUtilError::InvalidCoords);
    }

    for (i, coord) in coords.iter_mut().enumerate() {
        if *coord < fvar.axes[i].min_value || *coord > fvar.axes[i].max_value {
            // TODO: Clamp Instead?
            return Err(ImtUtilError::InvalidCoords);
        }

        *coord = if *coord < fvar.axes[i].default_value {
            (*coord - fvar.axes[i].default_value)
                / (fvar.axes[i].default_value - fvar.axes[i].min_value)
        } else if *coord > fvar.axes[i].default_value {
            (*coord - fvar.axes[i].default_value)
                / (fvar.axes[i].max_value - fvar.axes[i].default_value)
        } else {
            0.0
        };

        if let Some(avar) = font.avar_table() {
            if avar.segment_maps[i].axis_value_maps.len() > 3 {
                let maps = &avar.segment_maps[i].axis_value_maps;
                let mut k = None;

                for (j, value_map) in maps.iter().enumerate() {
                    if *coord > value_map.from_coord {
                        k = Some(j);
                    }
                }

                if k.is_none() {
                    return Err(ImtUtilError::MalformedFont);
                }

                let k = k.unwrap();

                if k == maps.len() - 1 {
                    return Err(ImtUtilError::MalformedFont);
                }

                *coord = (((maps[k + 1].from_coord - *coord)
                    / (maps[k + 1].from_coord / maps[k].from_coord))
                    * (maps[k + 1].to_coord - maps[k].to_coord))
                    + maps[k].to_coord;
            }
        }
    }

    Ok(())
}

pub fn outline_apply_gvar(
    font: &Font,
    glyph_index: u16,
    outline: &mut Outline,
    coords: &Vec<f32>,
) -> Result<(), ImtUtilError> {
    if coords.iter().any(|coord| *coord < -1.0 || *coord > 1.0) {
        return Err(ImtUtilError::InvalidCoords);
    }

    let gvar = font.gvar_table().ok_or(ImtUtilError::MissingTable)?;

    if coords.len() != gvar.axis_count {
        return Err(ImtUtilError::InvalidCoords);
    }

    let glyph_variation = gvar
        .glyph_variations
        .get(&glyph_index)
        .ok_or(ImtUtilError::NoData)?;

    let mut point_deltas = vec![0.0; outline.num_packed_points];

    for tuple in glyph_variation.tuples.iter() {
        for (axis_i, axis_coord) in coords.iter().enumerate() {}
    }

    todo!()
}
