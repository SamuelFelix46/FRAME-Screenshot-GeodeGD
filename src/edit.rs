use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
pub fn crop_from_drag(from: Point, to: Point, dimensions: [u32; 2]) -> Option<Crop> {
    let [w, h] = dimensions;
    if w == 0 || h == 0 || ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) {
        return None;
    }
    let x = from.x.min(to.x).floor().clamp(0.0, (w - 1) as f32) as u32;
    let y = from.y.min(to.y).floor().clamp(0.0, (h - 1) as f32) as u32;
    let right = from.x.max(to.x).ceil().clamp(0.0, w as f32) as u32;
    let bottom = from.y.max(to.y).ceil().clamp(0.0, h as f32) as u32;
    Some(Crop {
        x,
        y,
        width: right.saturating_sub(x).max(1).min(w - x),
        height: bottom.saturating_sub(y).max(1).min(h - y),
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Annotation {
    Stroke {
        points: Vec<Point>,
        color: [u8; 4],
        width: f32,
    },
    Arrow {
        from: Point,
        to: Point,
        color: [u8; 4],
        width: f32,
    },
    Text {
        at: Point,
        text: String,
        color: [u8; 4],
        size: f32,
    },
    Frame {
        from: Point,
        to: Point,
        color: [u8; 4],
        width: f32,
        #[serde(default)]
        filled: bool,
    },
    Ellipse {
        from: Point,
        to: Point,
        color: [u8; 4],
        width: f32,
        #[serde(default)]
        filled: bool,
    },
}

impl Annotation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Stroke { .. } => crate::i18n::text("Dessin", "Drawing"),
            Self::Arrow { .. } => crate::i18n::text("Flèche", "Arrow"),
            Self::Text { .. } => crate::i18n::text("Texte", "Text"),
            Self::Frame { .. } => crate::i18n::text("Cadre", "Frame"),
            Self::Ellipse { .. } => crate::i18n::text("Cercle", "Ellipse"),
        }
    }
    pub fn color_mut(&mut self) -> &mut [u8; 4] {
        match self {
            Self::Stroke { color, .. }
            | Self::Arrow { color, .. }
            | Self::Text { color, .. }
            | Self::Frame { color, .. }
            | Self::Ellipse { color, .. } => color,
        }
    }
    pub fn bounds(&self) -> Option<[Point; 2]> {
        let points: Vec<Point> = match self {
            Self::Stroke { points, .. } => points.clone(),
            Self::Arrow {
                from, to, width, ..
            } => vec![
                *from,
                *to,
                arrow_wings(*from, *to, *width)[0],
                arrow_wings(*from, *to, *width)[1],
            ],
            Self::Frame { from, to, .. } | Self::Ellipse { from, to, .. } => vec![*from, *to],
            Self::Text { at, text, size, .. } => {
                use ab_glyph::{Font, ScaleFont};
                if !size.is_finite() {
                    return None;
                }
                let font =
                    ab_glyph::FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).ok()?;
                let scaled = font.as_scaled(size.clamp(6.0, 256.0));
                let text: String = text.chars().take(1024).collect();
                let mut width: f32 = 0.0;
                for line in text.split('\n') {
                    let mut previous = None;
                    let mut advance = 0.0;
                    for c in line.chars() {
                        let id = scaled.glyph_id(c);
                        advance +=
                            scaled.h_advance(id) + previous.map_or(0.0, |p| scaled.kern(p, id));
                        previous = Some(id);
                    }
                    width = width.max(advance);
                }
                vec![
                    *at,
                    Point {
                        x: at.x + width.max(1.0),
                        y: at.y
                            + size.clamp(6.0, 256.0)
                                * (1.0 + 1.2 * (text.split('\n').count() - 1) as f32),
                    },
                ]
            }
        };
        if points.is_empty() || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return None;
        }
        Some([
            Point {
                x: points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                y: points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
            },
            Point {
                x: points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                y: points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
            },
        ])
    }
    fn map_points(&mut self, f: impl Fn(Point) -> Point) {
        match self {
            Self::Stroke { points, .. } => points.iter_mut().for_each(|p| *p = f(*p)),
            Self::Arrow { from, to, .. }
            | Self::Frame { from, to, .. }
            | Self::Ellipse { from, to, .. } => {
                *from = f(*from);
                *to = f(*to);
            }
            Self::Text { at, .. } => *at = f(*at),
        }
    }
    pub fn translated_clamped(&self, dx: f32, dy: f32, dimensions: [u32; 2]) -> Self {
        let mut next = self.clone();
        if !dx.is_finite() || !dy.is_finite() || dimensions.contains(&0) {
            return next;
        }
        if let Some([min, max]) = self.bounds() {
            let dx = dx.clamp(-min.x, (dimensions[0] as f32 - 1.0 - max.x).max(-min.x));
            let dy = dy.clamp(-min.y, (dimensions[1] as f32 - 1.0 - max.y).max(-min.y));
            next.map_points(|p| Point {
                x: p.x + dx,
                y: p.y + dy,
            });
        }
        next
    }
    pub fn resized(&self, width: f32, height: f32, dimensions: [u32; 2]) -> Self {
        let mut next = self.clone();
        if !width.is_finite() || !height.is_finite() || dimensions.contains(&0) {
            return next;
        }
        if let Some([min, max]) = self.bounds() {
            let width = width
                .max(1.0)
                .min((dimensions[0] as f32 - 1.0 - min.x).max(1.0));
            let height = height
                .max(1.0)
                .min((dimensions[1] as f32 - 1.0 - min.y).max(1.0));
            let sx = if max.x > min.x {
                width / (max.x - min.x)
            } else {
                1.0
            };
            let sy = if max.y > min.y {
                height / (max.y - min.y)
            } else {
                1.0
            };
            next.map_points(|p| Point {
                x: min.x + (p.x - min.x) * sx,
                y: min.y + (p.y - min.y) * sy,
            });
        }
        next
    }
    /// Resize around the opposite corner; the pointer's movement is measured from the original handle.
    pub fn resized_corner(
        &self,
        corner: usize,
        dx: f32,
        dy: f32,
        square: bool,
        dimensions: [u32; 2],
    ) -> Self {
        let Some([min, max]) = self.bounds() else {
            return self.clone();
        };
        if !dx.is_finite() || !dy.is_finite() || dimensions.contains(&0) || corner > 3 {
            return self.clone();
        }
        let left = corner == 0 || corner == 2;
        let top = corner < 2;
        let anchor = Point {
            x: if left { max.x } else { min.x },
            y: if top { max.y } else { min.y },
        };
        let target = Point {
            x: (if left { min.x } else { max.x } + dx).clamp(0.0, dimensions[0] as f32 - 1.0),
            y: (if top { min.y } else { max.y } + dy).clamp(0.0, dimensions[1] as f32 - 1.0),
        };
        let mut width = (if left {
            anchor.x - target.x
        } else {
            target.x - anchor.x
        })
        .max(1.0);
        let mut height = (if top {
            anchor.y - target.y
        } else {
            target.y - anchor.y
        })
        .max(1.0);
        if square {
            let side = width.min(height);
            width = side;
            height = side;
        }
        let origin = Point {
            x: if left { anchor.x - width } else { anchor.x },
            y: if top { anchor.y - height } else { anchor.y },
        };
        let mut next = self.clone();
        next.map_points(|p| Point {
            x: origin.x + (p.x - min.x) * width / (max.x - min.x).max(1.0),
            y: origin.y + (p.y - min.y) * height / (max.y - min.y).max(1.0),
        });
        next
    }
    pub fn hit_test(&self, point: Point, tolerance: f32) -> bool {
        if !point.x.is_finite() || !point.y.is_finite() || !tolerance.is_finite() {
            return false;
        }
        let tolerance = tolerance.max(0.0);
        let distance = |a: Point, b: Point| {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let denom = dx * dx + dy * dy;
            let t = if denom > 0.0 {
                (((point.x - a.x) * dx + (point.y - a.y) * dy) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (point.x - a.x - t * dx).hypot(point.y - a.y - t * dy)
        };
        match self {
            Self::Stroke { points, width, .. } => {
                let radius = tolerance + width.clamp(1.0, 64.0) * 0.5;
                points.windows(2).any(|p| distance(p[0], p[1]) <= radius)
                    || points
                        .first()
                        .is_some_and(|p| points.len() == 1 && distance(*p, *p) <= radius)
            }
            Self::Arrow {
                from, to, width, ..
            } => {
                let radius = tolerance + width.clamp(1.0, 64.0) * 0.5;
                distance(*from, *to) <= radius
                    || arrow_wings(*from, *to, *width)
                        .iter()
                        .any(|wing| distance(*to, *wing) <= radius)
            }
            Self::Text { .. } => self.bounds().is_some_and(|[a, b]| {
                point.x >= a.x - tolerance
                    && point.x <= b.x + tolerance
                    && point.y >= a.y - tolerance
                    && point.y <= b.y + tolerance
            }),
            Self::Frame {
                from,
                to,
                width,
                filled,
                ..
            }
            | Self::Ellipse {
                from,
                to,
                width,
                filled,
                ..
            } => {
                let Some([a, b]) = shape_bounds(*from, *to) else {
                    return false;
                };
                let radius = tolerance + width.clamp(1.0, 64.0) * 0.5;
                if matches!(self, Self::Frame { .. }) {
                    let inside =
                        point.x >= a.x && point.x <= b.x && point.y >= a.y && point.y <= b.y;
                    (*filled && inside)
                        || [
                            (a, Point { x: b.x, y: a.y }),
                            (Point { x: b.x, y: a.y }, b),
                            (b, Point { x: a.x, y: b.y }),
                            (Point { x: a.x, y: b.y }, a),
                        ]
                        .iter()
                        .any(|(a, b)| distance(*a, *b) <= radius)
                } else {
                    if point.x < a.x - radius
                        || point.x > b.x + radius
                        || point.y < a.y - radius
                        || point.y > b.y + radius
                    {
                        return false;
                    }
                    let cx = (a.x + b.x) * 0.5;
                    let cy = (a.y + b.y) * 0.5;
                    let rx = (b.x - a.x) * 0.5;
                    let ry = (b.y - a.y) * 0.5;
                    let normalized = ((point.x - cx) / rx).hypot((point.y - cy) / ry);
                    if *filled && normalized <= 1.0 {
                        return true;
                    }
                    let contour = ellipse_points(a, b);
                    (0..contour.len())
                        .any(|i| distance(contour[i], contour[(i + 1) % contour.len()]) <= radius)
                }
            }
        }
    }
}

pub fn centered_crop(dimensions: [u32; 2], numerator: u32, denominator: u32) -> Option<Crop> {
    let [w, h] = dimensions;
    if w == 0 || h == 0 || numerator == 0 || denominator == 0 {
        return None;
    }
    let ratio = numerator as f64 / denominator as f64;
    let (width, height) = if w as f64 / h as f64 > ratio {
        ((h as f64 * ratio).floor().max(1.0) as u32, h)
    } else {
        (w, (w as f64 / ratio).floor().max(1.0) as u32)
    };
    Some(Crop {
        x: (w - width) / 2,
        y: (h - height) / 2,
        width,
        height,
    })
}

/// Source coordinates shared by the live preview and PNG renderer.
pub fn shape_bounds(from: Point, to: Point) -> Option<[Point; 2]> {
    if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) {
        return None;
    }
    let min = Point {
        x: from.x.min(to.x),
        y: from.y.min(to.y),
    };
    let max = Point {
        x: from.x.max(to.x),
        y: from.y.max(to.y),
    };
    (max.x - min.x >= 1.0 && max.y - min.y >= 1.0).then_some([min, max])
}

pub fn square_endpoint(from: Point, to: Point, dimensions: [u32; 2]) -> Point {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let sx = if dx < 0.0 { -1.0 } else { 1.0 };
    let sy = if dy < 0.0 { -1.0 } else { 1.0 };
    let available_x = if sx > 0.0 {
        dimensions[0].saturating_sub(1) as f32 - from.x
    } else {
        from.x
    };
    let available_y = if sy > 0.0 {
        dimensions[1].saturating_sub(1) as f32 - from.y
    } else {
        from.y
    };
    let side = dx
        .abs()
        .max(dy.abs())
        .min(available_x)
        .min(available_y)
        .max(0.0);
    Point {
        x: from.x + side * sx,
        y: from.y + side * sy,
    }
}

pub fn ellipse_points(from: Point, to: Point) -> Vec<Point> {
    let Some([min, max]) = shape_bounds(from, to) else {
        return Vec::new();
    };
    let rx = (max.x - min.x) * 0.5;
    let ry = (max.y - min.y) * 0.5;
    let cx = min.x + rx;
    let cy = min.y + ry;
    let count = ((rx + ry) * 2.0).ceil().clamp(32.0, 4096.0) as usize;
    (0..count)
        .map(|i| {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            Point {
                x: cx + rx * angle.cos(),
                y: cy + ry * angle.sin(),
            }
        })
        .collect()
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub crop: Option<Crop>,
    pub annotations: Vec<Annotation>,
}
impl Document {
    pub fn pick(&self, point: Point, tolerance: f32) -> Option<usize> {
        self.annotations.iter().rposition(|a| {
            if a.hit_test(point, tolerance) {
                return true;
            }
            let Some([min, max]) = a.bounds() else {
                return false;
            };
            match a {
                Annotation::Frame { .. } => {
                    point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
                }
                Annotation::Ellipse { .. } if max.x > min.x && max.y > min.y => {
                    let x = (point.x - (min.x + max.x) * 0.5) / ((max.x - min.x) * 0.5);
                    let y = (point.y - (min.y + max.y) * 0.5) / ((max.y - min.y) * 0.5);
                    x * x + y * y <= 1.0
                }
                _ => false,
            }
        })
    }
}
pub fn render_edit(original: &RgbaImage, doc: &Document) -> Result<RgbaImage, String> {
    use image::Rgba;
    use imageproc::drawing::{draw_filled_circle_mut, draw_text_mut};
    let mut image = original.clone();
    let font = ab_glyph::FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT)
        .map_err(|e| e.to_string())?;
    let w = image.width() as f32;
    let h = image.height() as f32;
    let safe = |p: Point| {
        p.x.is_finite()
            && p.y.is_finite()
            && p.x >= -w
            && p.x <= w * 2.0
            && p.y >= -h
            && p.y <= h * 2.0
    };
    for annotation in doc.annotations.iter().take(2048) {
        match annotation {
            Annotation::Stroke {
                points,
                color,
                width,
            } => {
                for pair in points.windows(2).take(100_000) {
                    if safe(pair[0]) && safe(pair[1]) {
                        thick_line(&mut image, pair[0], pair[1], *color, *width);
                    }
                }
                if points.len() == 1 && safe(points[0]) {
                    draw_filled_circle_mut(
                        &mut image,
                        (points[0].x as i32, points[0].y as i32),
                        width.clamp(1.0, 64.0).ceil() as i32 / 2,
                        Rgba(*color),
                    );
                }
            }
            Annotation::Arrow {
                from,
                to,
                color,
                width,
            } if safe(*from) && safe(*to) => {
                thick_line(&mut image, *from, *to, *color, *width);
                for p in arrow_wings(*from, *to, *width) {
                    thick_line(&mut image, *to, p, *color, *width);
                }
            }
            Annotation::Text {
                at,
                text,
                color,
                size,
            } if safe(*at) && size.is_finite() => {
                let text: String = text.chars().take(1024).collect();
                for (line_index, line) in text.split('\n').enumerate() {
                    draw_text_mut(
                        &mut image,
                        Rgba(*color),
                        at.x as i32,
                        (at.y + line_index as f32 * size.clamp(6.0, 256.0) * 1.2) as i32,
                        size.clamp(6.0, 256.0),
                        &font,
                        line,
                    );
                }
            }
            Annotation::Frame {
                from,
                to,
                color,
                width,
                filled,
            }
            | Annotation::Ellipse {
                from,
                to,
                color,
                width,
                filled,
            } if safe(*from) && safe(*to) => {
                let Some([min, max]) = shape_bounds(*from, *to) else {
                    continue;
                };
                let rectangle = matches!(annotation, Annotation::Frame { .. });
                if *filled {
                    if rectangle {
                        imageproc::drawing::draw_filled_rect_mut(
                            &mut image,
                            imageproc::rect::Rect::at(min.x.round() as i32, min.y.round() as i32)
                                .of_size(
                                    (max.x - min.x).round() as u32 + 1,
                                    (max.y - min.y).round() as u32 + 1,
                                ),
                            Rgba(*color),
                        );
                    } else {
                        imageproc::drawing::draw_filled_ellipse_mut(
                            &mut image,
                            (
                                ((min.x + max.x) * 0.5).round() as i32,
                                ((min.y + max.y) * 0.5).round() as i32,
                            ),
                            ((max.x - min.x) * 0.5).round() as i32,
                            ((max.y - min.y) * 0.5).round() as i32,
                            Rgba(*color),
                        );
                    }
                }
                let points = if rectangle {
                    vec![
                        min,
                        Point { x: max.x, y: min.y },
                        max,
                        Point { x: min.x, y: max.y },
                    ]
                } else {
                    ellipse_points(min, max)
                };
                for i in 0..points.len() {
                    thick_line(
                        &mut image,
                        points[i],
                        points[(i + 1) % points.len()],
                        *color,
                        *width,
                    );
                }
            }
            _ => {}
        }
    }
    if let Some(c) = doc.crop {
        if c.width == 0
            || c.height == 0
            || c.x.checked_add(c.width).is_none_or(|x| x > image.width())
            || c.y.checked_add(c.height).is_none_or(|y| y > image.height())
        {
            return Err(crate::i18n::text(
                "Le recadrage sort de l’image.",
                "The crop extends outside the image.",
            )
            .into());
        }
        image = image::imageops::crop_imm(&image, c.x, c.y, c.width, c.height).to_image();
    }
    Ok(image)
}
fn thick_line(image: &mut RgbaImage, from: Point, to: Point, color: [u8; 4], width: f32) {
    use imageproc::drawing::draw_filled_circle_mut;
    let width = if width.is_finite() {
        width.clamp(1.0, 64.0)
    } else {
        3.0
    };
    let n = ((to.x - from.x).hypot(to.y - from.y).ceil() as u32).clamp(1, 32768);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        draw_filled_circle_mut(
            image,
            (
                (from.x + (to.x - from.x) * t).round() as i32,
                (from.y + (to.y - from.y) * t).round() as i32,
            ),
            (width / 2.0).ceil() as i32,
            image::Rgba(color),
        );
    }
}
pub fn arrow_wings(from: Point, to: Point, width: f32) -> [Point; 2] {
    let a = (to.y - from.y).atan2(to.x - from.x);
    let len = (width * 4.0).max(12.0);
    [-0.5_f32, 0.5].map(|offset| Point {
        x: to.x - len * (a + offset).cos(),
        y: to.y - len * (a + offset).sin(),
    })
}
#[derive(Default)]
pub struct History {
    pub current: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
}
impl History {
    pub fn commit(&mut self, next: Document) {
        let previous = std::mem::replace(&mut self.current, next);
        self.checkpoint(previous);
    }
    pub fn checkpoint(&mut self, previous: Document) {
        if previous == self.current {
            return;
        }
        self.undo.push(previous);
        if self.undo.len() > 40 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.current, previous));
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.current, next));
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
