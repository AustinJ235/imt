use std::collections::BTreeMap;
use std::ops::Range;

use crate::error::*;
use crate::parse::{read_i16, read_u16, LocaTable};

const MALFORMED: ImtError = ImtError {
    kind: ImtErrorKind::Malformed,
    source: ImtErrorSource::GlyfTable,
};

const TRUNCATED: ImtError = ImtError {
    kind: ImtErrorKind::Truncated,
    source: ImtErrorSource::GlyfTable,
};

#[derive(Debug, Clone)]
pub struct GlyfTable {
    pub outlines: BTreeMap<u16, Outline>,
}

#[derive(Debug, Clone)]
pub struct Outline {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    /// Raw points parsed from font data.
    pub points: Vec<OutlineRawPoint>,
    /// Ranges in points that belong to a specific contour
    pub contours: Vec<Range<usize>>,
    /// Points that have been processed into segments and curves
    pub geometry: Vec<OutlineGeometry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutlineGeometry {
    Segment {
        p1: OutlinePoint,
        p2: OutlinePoint,
    },
    QuadraticCurve {
        p1: OutlinePoint,
        p2: OutlinePoint,
        p3: OutlinePoint,
    },
}

impl OutlineGeometry {
    pub fn is_curve(&self) -> bool {
        matches!(self, Self::QuadraticCurve { .. })
    }

    pub fn evaluate(&self, t: f32) -> OutlinePoint {
        match self {
            Self::Segment {
                p1,
                p2,
            } => {
                OutlinePoint {
                    x: (t * (p2.x - p1.x)) + p1.x,
                    y: (t * (p2.y - p1.y)) + p1.y,
                }
            },
            Self::QuadraticCurve {
                p1,
                p2,
                p3,
            } => {
                OutlinePoint {
                    x: ((1.0 - t).powi(2) * p1.x)
                        + (2.0 * (1.0 - t) * t * p2.x)
                        + (t.powi(2) * p3.x),
                    y: ((1.0 - t).powi(2) * p1.y)
                        + (2.0 * (1.0 - t) * t * p2.y)
                        + (t.powi(2) * p3.y),
                }
            },
        }
    }
}

/// A struct referencing the raw point parsed from font data.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineRawPoint {
    /// Index of the contour
    pub c: u16,
    pub x: f32,
    pub y: f32,
    /// Whether or not this is a control point.
    pub control: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlinePoint {
    pub x: f32,
    pub y: f32,
}

impl Outline {
    pub(crate) fn rebuild(&mut self) -> Result<(), ImtError> {
        let mut x_min = f32::INFINITY;
        let mut x_max = f32::NEG_INFINITY;
        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let mut geometry = Vec::new();

        for range in self.contours.iter().cloned() {
            if range.len() < 3 || self.points[range.start].control {
                return Err(MALFORMED);
            }

            let mut points = Vec::new();

            for i in range.clone() {
                points.push((self.points[i].x, self.points[i].y, self.points[i].control));

                if i != range.start
                    && i != range.end - 1
                    && self.points[i].control
                    && self.points[i + 1].control
                {
                    points.push((
                        (self.points[i].x + self.points[i + 1].x) / 2.0,
                        (self.points[i].y + self.points[i + 1].y) / 2.0,
                        false,
                    ));
                }
            }

            for point in points.iter() {
                if point.0 < x_min {
                    x_min = point.0;
                }

                if point.0 > x_max {
                    x_max = point.0;
                }

                if point.1 < y_min {
                    y_min = point.1;
                }

                if point.1 > y_max {
                    y_max = point.1;
                }
            }

            let mut contour_geo = Vec::new();

            for i in 0..points.len() {
                let j = (i + 1) % points.len();

                if points[i].2 {
                    contour_geo.push(OutlineGeometry::QuadraticCurve {
                        p1: OutlinePoint {
                            x: points[i - 1].0,
                            y: points[i - 1].1,
                        },
                        p2: OutlinePoint {
                            x: points[i].0,
                            y: points[i].1,
                        },
                        p3: OutlinePoint {
                            x: points[j].0,
                            y: points[j].1,
                        },
                    });
                } else if !points[j].2 {
                    contour_geo.push(OutlineGeometry::Segment {
                        p1: OutlinePoint {
                            x: points[i].0,
                            y: points[i].1,
                        },
                        p2: OutlinePoint {
                            x: points[j].0,
                            y: points[j].1,
                        },
                    });
                }
            }

            geometry.append(&mut contour_geo);
        }

        self.x_min = x_min;
        self.x_max = x_max;
        self.y_min = y_min;
        self.y_max = y_max;
        self.geometry = geometry;
        Ok(())
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
            return Err(MALFORMED);
        }

        for i in 0..(loca_table.offsets.len() - 1) {
            if loca_table.offsets[i] == loca_table.offsets[i + 1] {
                // No Outline
                continue;
            }

            let glyph_offset = table_offset + loca_table.offsets[i] as usize;

            if glyph_offset + 10 > bytes.len() {
                return Err(TRUNCATED);
            }

            let number_of_contours = read_i16(bytes, glyph_offset);
            // Bytes +2 to +10 contain the bounding box. It is automatically computed, so ignored.

            if number_of_contours > 0 {
                let number_of_contours = number_of_contours as usize;
                let end_pts_of_contours_end_offset = glyph_offset + 10 + (number_of_contours * 2);

                if end_pts_of_contours_end_offset + 2 > bytes.len() {
                    return Err(TRUNCATED);
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
                        return Err(TRUNCATED);
                    }

                    let flag = SimpleFlags(bytes[flag_offset]);
                    flag_offset += 1;
                    let mut flag_count = 1;

                    if flag.repeat_flag() {
                        if flag_offset >= bytes.len() {
                            return Err(TRUNCATED);
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
                            return Err(TRUNCATED);
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
                                return Err(TRUNCATED);
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
                            return Err(TRUNCATED);
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
                                return Err(TRUNCATED);
                            }

                            let dy = read_i16(bytes, coordinate_offset);
                            coordinate_offset += 2;
                            let y = previous_y + dy;
                            previous_y = y;
                            y_coordinates.push(y);
                        }
                    }
                }

                let mut points = Vec::with_capacity(flags.len());
                let mut contours = Vec::with_capacity(number_of_contours);

                for j in 0..number_of_contours {
                    let range_start = if j == 0 {
                        0
                    } else {
                        end_pts_of_contours[j - 1] + 1
                    };

                    let range_end = end_pts_of_contours[j] + 1;

                    if range_start >= range_end {
                        return Err(MALFORMED);
                    }

                    contours.push(range_start..range_end);

                    for k in range_start..range_end {
                        points.push(OutlineRawPoint {
                            c: j as u16,
                            x: x_coordinates[k] as f32,
                            y: y_coordinates[k] as f32,
                            control: !flags[k].on_curve_point(),
                        });
                    }
                }

                if x_coordinates.len() != y_coordinates.len() || x_coordinates.len() != points.len()
                {
                    return Err(MALFORMED);
                }

                let mut outline = Outline {
                    x_min: 0.0,
                    y_min: 0.0,
                    x_max: 0.0,
                    y_max: 0.0,
                    points,
                    contours,
                    geometry: Vec::new(),
                };

                outline.rebuild()?;
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
