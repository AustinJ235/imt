use crate::parse::Font;

pub struct BasicGlyphRender {
    pub width: u32,
    pub height: u32,
    pub bearing_y: i16,
    pub data: Vec<u8>,
}

pub fn basic_render(
    font: &Font,
    glyph_index: u16,
    size: f32,
) -> BasicGlyphRender {
    let outline = font
        .glyf_table()
        .outlines
        .get(&glyph_index)
        .unwrap()
        .clone();
    
    let scaler = (1.0 / font.head_table().units_per_em as f32) * size;
    let width_f = (outline.x_max as f32 - outline.x_min as f32) * scaler;
    let width_r = width_f.ceil();
    let image_w = width_r as u32;
    let scaler_hori = (width_r / width_f) * scaler;

    let (image_h, bearing_y, scaler_above, scaler_below) = if outline.y_max <= 0 || outline.y_min >= 0 {
        // Everything above or below baseline
        let height_f = (outline.y_max as f32 - outline.y_min as f32) * scaler;
        let y_max_r = (outline.y_max as f32 * scaler).ceil();
        let y_min_r = (outline.y_min as f32 * scaler).ceil();
        let height_r = y_max_r - y_min_r;
        let image_h = height_r as u32;
        let scaler_vert = (height_r / height_f) * scaler;
        (image_h, y_min_r as i16, scaler_vert, scaler_vert)
    } else {
        // Some above and below baseline
        let above_f = outline.y_max as f32 * scaler;
        let below_f = outline.y_min as f32 * scaler;
        let above_r = above_f.ceil();
        let below_r = below_f.ceil();
        let image_h = (above_r - below_r) as u32;
        let scaler_above = (above_r / above_f) * scaler;
        let scaler_below = (below_r / below_f) * scaler;
        (image_h, below_r as i16, scaler_above, scaler_below)
    };

    let trans_x = |x: f32| {
        (x - outline.x_min as f32) * scaler_hori
    };

    let trans_y = |y: f32| {
        if y > 0.0 {
            (y - outline.y_min as f32) * scaler_above
        } else {
            (y - outline.y_min as f32) * scaler_below
        }
    };

    let mut segments = Vec::new();

    for contour in outline.contours.iter() {
        let mut sum = 0.0;

        for i in 0..contour.points.len() {
            let j = (i + 1) % contour.points.len();
            sum += (contour.points[j].x - contour.points[i].x) * (contour.points[j].y - contour.points[i].y);
        }

        let cw = sum > 0.0;

        for segment in contour.segments.iter() {
            segments.push(Segment {
                x1: trans_x(segment.p1.x),
                y1: trans_y(segment.p1.y),
                x2: trans_x(segment.p2.x),
                y2: trans_x(segment.p2.y),
                cw,
            });
        }

        for curve in outline.curves() {
            for i in 0..16 {
                let [x1, y1] = curve.evaluate(i as f32 / 16.0);
                let [x2, y2] = curve.evaluate((i + 1) as f32 / 16.0);

                segments.push(Segment {
                    x1: trans_x(x1),
                    y1: trans_y(y1),
                    x2: trans_x(x2),
                    y2: trans_x(y2),
                    cw,
                });
            }
        }
    }

    let mut data: Vec<u8> = Vec::with_capacity(image_w as usize * image_h as usize);

    let ray_angles = [
        20.0_f32.to_radians(),
        50.0_f32.to_radians(),
        80.0_f32.to_radians(),
        110.0_f32.to_radians(),
        140.0_f32.to_radians(),
        170.0_f32.to_radians(),
        200.0_f32.to_radians(),
        230.0_f32.to_radians(),
        260.0_f32.to_radians(),
        290.0_f32.to_radians(),
        320.0_f32.to_radians(),
        350.0_f32.to_radians(),

        //90.0_f32.to_radians(),
        //270.0_f32.to_radians(),
        //0.0_f32.to_radians(),
        //180.0_f32.to_radians(),
    ];

    let ray_dirs: Vec<_> = ray_angles.into_iter().map(|angle| {
        [
            angle.cos() * 1000.0,
            angle.sin() * 1000.0,
        ]
    }).collect();

    //for x in 0..image_w {
    for y in (0..image_h).into_iter().rev() {
        //for y in 0..image_h {
        for x in 0..image_w {
            let ry1 = y as f32;
            let rx1 = x as f32;
            let mut fill = 0;
            

            for dir in ray_dirs.iter() {
                let rx2 = rx1 + dir[0];
                let ry2 = ry1 + dir[1];
                let mut sum = 0_u8;

                for segment in segments.iter() {
                    if line_intersects([segment.x1, segment.y1], [segment.x2, segment.y2], [rx1, ry1], [rx2, ry2]).is_some() {
                    //if ray_intersects(rx1, ry1, rx2, ry2, segment) {
                        if segment.cw {
                            sum += 1;
                        } else {
                            sum += 1;
                        }
                    }
                }

                if sum % 2 != 0 {
                    fill += 1;
                }
            }

            if fill >= ray_dirs.len() / 2 {
                data.push(255);
            } else {
                data.push(0);
            }
        }
    }

    BasicGlyphRender {
        width: image_w,
        height: image_h,
        bearing_y,
        data,
    }
}

pub trait SloppyEq: Copy {
    fn s_eq(self, other: Self) -> bool;
    fn s_lt(self, other: Self) -> bool;
    fn s_gt(self, other: Self) -> bool;

    fn s_lte(self, other: Self) -> bool {
        self.s_eq(other) || self.s_lt(other)
    }

    fn s_gte(self, other: Self) -> bool {
        self.s_eq(other) || self.s_gt(other)
    }
}

const F32_S_TOL: f32 = f32::EPSILON;

impl SloppyEq for f32 {
    fn s_eq(self, other: Self) -> bool {
        (self - other).abs() < F32_S_TOL
    }

    fn s_lt(self, other: Self) -> bool {
        self + F32_S_TOL < other
    }

    fn s_gt(self, other: Self) -> bool {
        self - F32_S_TOL > other
    }
}

struct Segment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    cw: bool,
}

#[allow(dead_code)]
fn ray_intersects(rx1: f32, ry1: f32, rx2: f32, ry2: f32, segment: &Segment) -> bool {
    // https://gamedev.stackexchange.com/questions/116422/best-way-to-find-line-segment-intersection

    let rn = ((segment.y1 - ry1) * (rx2 - rx1)) - ((segment.x1 - rx1) * (ry2 - ry1));
    let rd = ((segment.x2 - segment.x1) * (ry2 - ry1)) - ((segment.y2 - segment.y1) * (rx2 - rx1));
    let r = rn / rd;
    let sn = ((segment.y1 - ry1) * (segment.x2 - segment.x1)) - ((segment.x1 - rx1) * (segment.y2 - segment.y1));
    let sd = ((segment.x2 - segment.x1) * (ry2 - ry1)) - ((segment.y2 - segment.y1) * (rx2 - rx1));
    let s = sn / sd;

    if r.s_lt(0.0) || r.s_gt(1.0) || s.s_lt(0.0) || s.s_gt(1.0) {
        false
    } else if rd.s_eq(0.0) {
        if rn.s_eq(0.0) {
            false
            //true
        } else {
            false
        }
    } else {
        // Px=Ax+r(Bx-Ax)
        // Py=Ay+r(By-Ay)
        true
    }
}

pub fn line_intersects(l1p1: [f32; 2], l1p2: [f32; 2], l2p1: [f32; 2], l2p2: [f32; 2]) -> Option<[f32; 2]> {
    let r = [l1p2[0] - l1p1[0], l1p2[1] - l1p1[1]];
    let s = [l2p2[0] - l2p1[0], l2p2[1] - l2p1[1]];
    let det = (r[0] * s[1]) - (r[1] * s[0]);
    let u = (((l2p1[0] - l1p1[0]) * r[1]) - ((l2p1[1] - l1p1[1]) * r[0])) / det;
    let t = (((l2p1[0] - l1p1[0]) * s[1]) - ((l2p1[1] - l1p1[1]) * s[0])) / det;

    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        Some([(l1p1[0] + r[0]) * t, (l1p1[1] + r[1]) * t])
    } else {
        None
    }
}
