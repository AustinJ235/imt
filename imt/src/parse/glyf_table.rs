use std::collections::BTreeMap;

use crate::error::*;
use crate::parse::{read_i16, read_u16, LocaTable};

#[derive(Debug, Clone)]
pub struct GlyfTable {
    pub outlines: BTreeMap<u16, Outline>,
}

#[derive(Debug, Clone)]
pub struct Outline {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub contours: Vec<OutlineContour>,
    /// The number of points before they were unpacked.
    pub num_packed_points: usize,
}

impl Outline {
    pub fn scale(&mut self, scale: f32) {
        // TODO: Adjust Bounding Box
        self.contours
            .iter_mut()
            .for_each(|contour| contour.scale(scale));
    }

    pub fn points(&self) -> impl Iterator<Item = &OutlinePoint> {
        self.contours
            .iter()
            .flat_map(|contour| contour.points.iter())
    }

    pub fn points_mut(&mut self) -> impl Iterator<Item = &mut OutlinePoint> {
        self.contours
            .iter_mut()
            .flat_map(|contour| contour.points.iter_mut())
    }

    pub fn curves(&self) -> impl Iterator<Item = &OutlineCurve> {
        self.contours
            .iter()
            .flat_map(|contour| contour.curves.iter())
    }

    pub fn curves_mut(&mut self) -> impl Iterator<Item = &mut OutlineCurve> {
        self.contours
            .iter_mut()
            .flat_map(|contour| contour.curves.iter_mut())
    }

    pub fn segments(&self) -> impl Iterator<Item = &OutlineSegment> {
        self.contours
            .iter()
            .flat_map(|contour| contour.segments.iter())
    }

    pub fn segments_mut(&mut self) -> impl Iterator<Item = &mut OutlineSegment> {
        self.contours
            .iter_mut()
            .flat_map(|contour| contour.segments.iter_mut())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineContour {
    pub points: Vec<OutlinePoint>,
    pub curves: Vec<OutlineCurve>,
    pub segments: Vec<OutlineSegment>,
}

impl OutlineContour {
    pub fn scale(&mut self, scale: f32) {
        self.points.iter_mut().for_each(|point| point.scale(scale));
        self.curves.iter_mut().for_each(|curve| curve.scale(scale));
        self.segments
            .iter_mut()
            .for_each(|segment| segment.scale(scale));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineCurve {
    pub p1: OutlinePoint,
    pub p2: OutlinePoint,
    pub p3: OutlinePoint,
}

impl OutlineCurve {
    pub fn scale(&mut self, scale: f32) {
        self.p1.scale(scale);
        self.p2.scale(scale);
        self.p3.scale(scale);
    }

    pub fn evaluate(&self, t: f32) -> [f32; 2] {
        [
            ((1.0 - t).powi(2) * self.p1.x)
                + (2.0 * (1.0 - t) * t * self.p2.x)
                + (t.powi(2) * self.p3.x),
            ((1.0 - t).powi(2) * self.p1.y)
                + (2.0 * (1.0 - t) * t * self.p2.y)
                + (t.powi(2) * self.p3.y),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineSegment {
    pub p1: OutlinePoint,
    pub p2: OutlinePoint,
}

impl OutlineSegment {
    pub fn scale(&mut self, scale: f32) {
        self.p1.scale(scale);
        self.p2.scale(scale);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlinePoint {
    pub x: f32,
    pub y: f32,
    pub control: bool,
}

impl OutlinePoint {
    pub fn scale(&mut self, scale: f32) {
        self.x *= scale;
        self.y *= scale;
    }
}

#[derive(Clone, Copy)]
struct SimpleFlags(u8);

impl SimpleFlags {
    fn on_curve_point(&self) -> bool {
        self.0 & 0x01 != 0
    }

    fn x_short_vector(&self) -> bool {
        self.0 & 0x02 != 0
    }

    fn y_short_vector(&self) -> bool {
        self.0 & 0x04 != 0
    }

    fn repeat_flag(&self) -> bool {
        self.0 & 0x08 != 0
    }

    fn x_is_same_or_positive_x_short_vector(&self) -> bool {
        self.0 & 0x10 != 0
    }

    fn y_is_same_or_positive_y_short_vector(&self) -> bool {
        self.0 & 0x20 != 0
    }
}

impl std::fmt::Debug for SimpleFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleFlags")
            .field("ON_CURVE_POINT", &self.on_curve_point())
            .field("X_SHORT_VECTOR", &self.x_short_vector())
            .field("Y_SHORT_VECTOR", &self.y_short_vector())
            .field("REPEAT_FLAG", &self.repeat_flag())
            .field(
                "X_IS_SAME_OR_POSITIVE_X_SHORT_VECTOR",
                &self.x_is_same_or_positive_x_short_vector(),
            )
            .field(
                "Y_IS_SAME_OR_POSITIVE_Y_SHORT_VECTOR",
                &self.y_is_same_or_positive_y_short_vector(),
            )
            .finish()
    }
}

impl GlyfTable {
    pub fn try_parse(
        bytes: &[u8],
        table_offset: usize,
        loca_table: &LocaTable,
    ) -> Result<Self, ImtError> {
        let mut outlines = BTreeMap::new();

        if loca_table.offsets.len() < 2 {
            return Err(ImtError {
                kind: ImtErrorKind::Malformed,
                source: ImtErrorSource::LocaTable,
            });
        }

        for i in 0..(loca_table.offsets.len() - 1) {
            if loca_table.offsets[i] == loca_table.offsets[i + 1] {
                // No Outline
                continue;
            }

            let glyph_offset = table_offset + loca_table.offsets[i] as usize;

            if glyph_offset + 10 > bytes.len() {
                return Err(ImtError {
                    kind: ImtErrorKind::Truncated,
                    source: ImtErrorSource::GlyfTable,
                });
            }

            let number_of_contours = read_i16(bytes, glyph_offset);
            let x_min = read_i16(bytes, glyph_offset + 2);
            let y_min = read_i16(bytes, glyph_offset + 4);
            let x_max = read_i16(bytes, glyph_offset + 6);
            let y_max = read_i16(bytes, glyph_offset + 8);

            if number_of_contours > 0 {
                let number_of_contours = number_of_contours as usize;
                let end_pts_of_contours_end_offset = glyph_offset + 10 + (number_of_contours * 2);

                if end_pts_of_contours_end_offset + 2 > bytes.len() {
                    return Err(ImtError {
                        kind: ImtErrorKind::Truncated,
                        source: ImtErrorSource::GlyfTable,
                    });
                }

                let mut end_pts_of_contours = Vec::with_capacity(number_of_contours);

                for j in 0..number_of_contours {
                    end_pts_of_contours.push(read_u16(bytes, glyph_offset + 10 + (j * 2)) as usize);
                }

                let instruction_length = read_u16(bytes, end_pts_of_contours_end_offset);
                let instructions_end_offset =
                    end_pts_of_contours_end_offset + 2 + (instruction_length as usize * 2);
                let number_of_points = *end_pts_of_contours.last().unwrap() + 1;
                let mut flags = Vec::with_capacity(number_of_points);
                let mut flag_offset = instructions_end_offset;

                while flags.len() < number_of_points {
                    if flag_offset >= bytes.len() {
                        return Err(ImtError {
                            kind: ImtErrorKind::Truncated,
                            source: ImtErrorSource::GlyfTable,
                        });
                    }

                    let flag = SimpleFlags(bytes[flag_offset]);
                    flag_offset += 1;
                    let mut flag_count = 1;

                    if flag.repeat_flag() {
                        if flag_offset >= bytes.len() {
                            return Err(ImtError {
                                kind: ImtErrorKind::Truncated,
                                source: ImtErrorSource::GlyfTable,
                            });
                        }

                        flag_count = bytes[flag_offset] + 1;
                        flag_offset += 1;
                    }

                    for _ in 0..flag_count {
                        flags.push(flag);
                    }
                }

                let mut coordinate_offset = flag_offset;
                let mut x_coordinates = Vec::with_capacity(number_of_points);
                let mut previous_x = 0;

                for flag in flags.iter() {
                    if flag.x_short_vector() {
                        if coordinate_offset >= bytes.len() {
                            return Err(ImtError {
                                kind: ImtErrorKind::Truncated,
                                source: ImtErrorSource::GlyfTable,
                            });
                        }

                        let dx = if flag.x_is_same_or_positive_x_short_vector() {
                            bytes[coordinate_offset] as i16
                        } else {
                            -(bytes[coordinate_offset] as i16)
                        };

                        coordinate_offset += 1;
                        let x = previous_x + dx;
                        previous_x = x;
                        x_coordinates.push(x);
                    } else {
                        if flag.x_is_same_or_positive_x_short_vector() {
                            x_coordinates.push(previous_x);
                        } else {
                            if coordinate_offset + 2 > bytes.len() {
                                return Err(ImtError {
                                    kind: ImtErrorKind::Truncated,
                                    source: ImtErrorSource::GlyfTable,
                                });
                            }

                            let dx = read_i16(bytes, coordinate_offset);
                            coordinate_offset += 2;
                            let x = previous_x + dx;
                            previous_x = x;
                            x_coordinates.push(x);
                        }
                    }
                }

                let mut y_coordinates = Vec::with_capacity(number_of_points);
                let mut previous_y = 0;

                for flag in flags.iter() {
                    if flag.y_short_vector() {
                        if coordinate_offset >= bytes.len() {
                            return Err(ImtError {
                                kind: ImtErrorKind::Truncated,
                                source: ImtErrorSource::GlyfTable,
                            });
                        }

                        let dy = if flag.y_is_same_or_positive_y_short_vector() {
                            bytes[coordinate_offset] as i16
                        } else {
                            -(bytes[coordinate_offset] as i16)
                        };

                        coordinate_offset += 1;
                        let y = previous_y + dy;
                        previous_y = y;
                        y_coordinates.push(y);
                    } else {
                        if flag.y_is_same_or_positive_y_short_vector() {
                            y_coordinates.push(previous_y);
                        } else {
                            if coordinate_offset + 2 > bytes.len() {
                                return Err(ImtError {
                                    kind: ImtErrorKind::Truncated,
                                    source: ImtErrorSource::GlyfTable,
                                });
                            }

                            let dy = read_i16(bytes, coordinate_offset);
                            coordinate_offset += 2;
                            let y = previous_y + dy;
                            previous_y = y;
                            y_coordinates.push(y);
                        }
                    }
                }

                let mut contours = Vec::with_capacity(number_of_contours);

                for j in 0..number_of_contours {
                    let range_start = if j == 0 {
                        0
                    } else {
                        end_pts_of_contours[j - 1] + 1
                    };

                    let range_end = end_pts_of_contours[j] + 1;

                    if range_start >= range_end {
                        return Err(ImtError {
                            kind: ImtErrorKind::Malformed,
                            source: ImtErrorSource::GlyfTable,
                        });
                    }

                    let mut points = Vec::new();

                    for k in range_start..range_end {
                        if k > range_start
                            && k < range_end - 1
                            && !flags[k].on_curve_point()
                            && !flags[k + 1].on_curve_point()
                        {
                            points.push(OutlinePoint {
                                x: x_coordinates[k] as f32,
                                y: y_coordinates[k] as f32,
                                control: true,
                            });

                            points.push(OutlinePoint {
                                x: (x_coordinates[k] as f32 + x_coordinates[k + 1] as f32) / 2.0,
                                y: (y_coordinates[k] as f32 + y_coordinates[k + 1] as f32) / 2.0,
                                control: false,
                            });
                        } else {
                            points.push(OutlinePoint {
                                x: x_coordinates[k] as f32,
                                y: y_coordinates[k] as f32,
                                control: !flags[k].on_curve_point(),
                            });
                        }
                    }

                    let mut segments = Vec::new();
                    let mut curves = Vec::new();

                    for k in 0..points.len() {
                        if points[k].control {
                            if k == 0 {
                                return Err(ImtError {
                                    kind: ImtErrorKind::Malformed,
                                    source: ImtErrorSource::GlyfTable,
                                });
                            } else if k + 1 >= points.len() {
                                curves.push(OutlineCurve {
                                    p1: points[k - 1].clone(),
                                    p2: points[k].clone(),
                                    p3: points[0].clone(),
                                });
                            } else {
                                curves.push(OutlineCurve {
                                    p1: points[k - 1].clone(),
                                    p2: points[k].clone(),
                                    p3: points[k + 1].clone(),
                                });
                            }
                        } else {
                            let l = (k + 1) % points.len();

                            if l == k {
                                return Err(ImtError {
                                    kind: ImtErrorKind::Malformed,
                                    source: ImtErrorSource::GlyfTable,
                                });
                            }

                            if !points[l].control {
                                segments.push(OutlineSegment {
                                    p1: points[k].clone(),
                                    p2: points[l].clone(),
                                });
                            }
                        }
                    }

                    contours.push(OutlineContour {
                        points,
                        curves,
                        segments,
                    });
                }

                let outline = Outline {
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    contours,
                    num_packed_points: number_of_points,
                };

                outlines.insert(i as u16, outline);
            } else if number_of_contours < 0 {
                // TODO: Composite
            } else {
                // Empty
            }
        }

        Ok(Self {
            outlines,
        })
    }
}
