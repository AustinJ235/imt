use std::cmp::Ord;

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

        if *coord < fvar.axes[i].default_value {
            *coord = (*coord - fvar.axes[i].default_value)
                / (fvar.axes[i].default_value - fvar.axes[i].min_value);
        } else if *coord > fvar.axes[i].default_value {
            *coord = (*coord - fvar.axes[i].default_value)
                / (fvar.axes[i].max_value - fvar.axes[i].default_value);
        } else {
            *coord = 0.0;
            continue;
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
            let peak = tuple.peak[axis_i];

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
            for range in outline.contours.clone() {
                // (Delta/Point Index, Outline Point Index)
                let points_in_range: Vec<(usize, usize)> = tuple
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, j)| (i, *j as usize))
                    .filter(|(_, j)| range.contains(j))
                    .collect();

                // No deltas for this contour
                if points_in_range.is_empty() {
                    continue;
                }

                // All deltas are the same
                if points_in_range.len() == 1 {
                    let dx = tuple.deltas[points_in_range[0].0][0] as f32 * tuple_scaler;
                    let dy = tuple.deltas[points_in_range[0].0][1] as f32 * tuple_scaler;

                    for i in range {
                        point_deltas[i][0] += dx;
                        point_deltas[i][1] += dy;
                    }

                    continue;
                }

                // Interpolation
                for i in range {
                    match points_in_range.binary_search_by(|(_, j)| j.cmp(&i)) {
                        // Explicit Delta
                        Ok(pir_i) => {
                            let delta_i = points_in_range[pir_i].0;
                            point_deltas[i][0] += tuple.deltas[delta_i][0] as f32 * tuple_scaler;
                            point_deltas[i][1] += tuple.deltas[delta_i][1] as f32 * tuple_scaler;
                        },
                        // Inferred Delta
                        Err(pir_i) => {
                            let (prec_pir_i, foll_pir_i) =
                                if pir_i == 0 || pir_i == points_in_range.len() {
                                    (points_in_range.len() - 1, 0)
                                } else {
                                    (pir_i - 1, pir_i)
                                };

                            let (prec_delta_i, prec_point_i) = points_in_range[prec_pir_i];
                            let (foll_delta_i, foll_point_i) = points_in_range[foll_pir_i];

                            // X & Y Deltas are treated seperate

                            point_deltas[i][0] += infer_delta(
                                outline.points[prec_point_i].x,
                                outline.points[i].x,
                                outline.points[foll_point_i].x,
                                tuple.deltas[prec_delta_i][0] as f32,
                                tuple.deltas[foll_delta_i][0] as f32,
                            ) * tuple_scaler;

                            point_deltas[i][1] += infer_delta(
                                outline.points[prec_point_i].y,
                                outline.points[i].y,
                                outline.points[foll_point_i].y,
                                tuple.deltas[prec_delta_i][1] as f32,
                                tuple.deltas[foll_delta_i][1] as f32,
                            ) * tuple_scaler;
                        },
                    }
                }
            }
        }
    }

    for (i, [dx, dy]) in point_deltas.into_iter().enumerate() {
        // TODO: Should these be retained in case of the 'hvar' table is missing? The code above
        //       will have to infer these also.

        // Phantom points are ignored
        if i >= outline.points.len() {
            break;
        }

        outline.points[i].x += dx;
        outline.points[i].y += dy;
    }

    outline
        .rebuild()
        .map_err(|_| ImtUtilError::MalformedOutline)
}

// impl pseudo-code from:
// https://learn.microsoft.com/en-us/typography/opentype/spec/gvar#inferred-deltas-for-un-referenced-point-numbers
fn infer_delta(px: f32, tx: f32, fx: f32, pd: f32, fd: f32) -> f32 {
    if px == fx {
        if pd == fd {
            pd
        } else {
            0.0
        }
    } else {
        if tx <= px.min(fx) {
            if px < fx {
                pd
            } else {
                fd
            }
        } else if tx >= px.max(fx) {
            if px > fx {
                pd
            } else {
                fd
            }
        } else {
            let p = (tx - px) / (fx - px);
            ((1.0 - p) * pd) + (p * fd)
        }
    }
}
