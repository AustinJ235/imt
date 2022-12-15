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

    let mut point_deltas = vec![[0.0, 0.0]; outline.points.len() + 4];

    'tuple: for tuple in glyph_variation.tuples.iter() {
        let mut tuple_scaler = 1.0;
        let mut tuple_applies = false;

        for (axis_i, axis_coord) in coords.iter().enumerate() {
            let peak = tuple.peak[0];

            // If the peak is at zero it is ignored.
            if peak == 0.0 {
                continue;
            }

            // If the axis coord equals the peak the scaler is one
            if peak == *axis_coord {
                tuple_applies = true;
                continue;
            }

            if let Some(interm) = &tuple.interm {
                let start = interm.start[axis_i];
                let end = interm.end[axis_i];

                // Out of range
                if *axis_coord < start || *axis_coord > end {
                    continue 'tuple;
                }

                // Scaler will be zero
                if *axis_coord == start || *axis_coord == end {
                    continue 'tuple;
                }

                tuple_applies = true;

                if *axis_coord < peak {
                    tuple_scaler *= (*axis_coord - start) / (peak - start);
                } else {
                    tuple_scaler *= (end - *axis_coord) / (end - peak);
                }
            } else {
                // Out of range
                if *axis_coord == 0.0 || *axis_coord < peak.min(0.0) || *axis_coord > peak.max(0.0)
                {
                    continue 'tuple;
                }

                tuple_applies = true;
                tuple_scaler *= *axis_coord / peak;
            }
        }

        // All axes were ignored, so delta does not apply
        if !tuple_applies {
            continue;
        }

        if tuple.points.is_empty() {
            for (i, [x, y]) in tuple.deltas.iter().enumerate() {
                point_deltas[i][0] += *x as f32 * tuple_scaler;
                point_deltas[i][1] += *y as f32 * tuple_scaler;
            }
        } else {
            // TODO: Interpolate

            for (i, [x, y]) in tuple.points.iter().zip(tuple.deltas.iter()) {
                point_deltas[*i as usize][0] += *x as f32 * tuple_scaler;
                point_deltas[*i as usize][1] += *y as f32 * tuple_scaler;
            }
        }
    }

    println!("Deltas: {:?}", point_deltas);

    // TODO: Apply deltas
    Ok(())
}
