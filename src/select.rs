//! Selections: lasso and polyline polygons, round brush strokes, and the pixel mask
//! they make.

use image::GrayImage;

/// One selection shape, in image pixel coordinates (pixel (x, y) covers x..x+1, y..y+1).
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A closed polygon (lasso or polyline): the last point joins the first.
    Polygon(Vec<[f32; 2]>),
    /// A round brush of `radius` dragged through `points` in order.
    Stroke { points: Vec<[f32; 2]>, radius: f32 },
}

/// How a shape changes the selection: no key, Shift, Ctrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Replace,
    Add,
    Subtract,
}

/// One bool per pixel, row-major; true = selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl Mask {
    /// A `width` x `height` mask with every bit set to `value`.
    pub fn new(width: u32, height: u32, value: bool) -> Mask {
        let len = width as usize * height as usize;
        Mask {
            width,
            height,
            bits: vec![value; len],
        }
    }

    /// A mask from existing bits; `None` when the length does not match the size.
    pub fn from_bits(width: u32, height: u32, bits: Vec<bool>) -> Option<Mask> {
        if bits.len() != width as usize * height as usize {
            return None;
        }
        Some(Mask {
            width,
            height,
            bits,
        })
    }

    /// A mask the size of `img`; a pixel is selected when its gray value is >= 128.
    pub fn from_luma(img: &GrayImage) -> Mask {
        let width = img.width();
        let height = img.height();
        let bits = img.pixels().map(|p| p[0] >= 128).collect();
        Mask {
            width,
            height,
            bits,
        }
    }

    /// The mask width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The mask height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The row-major bits.
    pub fn bits(&self) -> &[bool] {
        &self.bits
    }

    /// The bit at `(x, y)`; `false` outside the mask.
    pub fn get(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.bits[y as usize * self.width as usize + x as usize]
    }

    /// The number of `true` bits.
    pub fn count(&self) -> usize {
        self.bits.iter().filter(|b| **b).count()
    }

    /// Apply `shape` with `op`: Replace -> r, Add -> bit || r, Subtract -> bit && !r.
    pub fn apply(&mut self, op: Op, shape: &Shape) {
        let r = rasterize(shape, self.width, self.height);
        for (bit, r) in self.bits.iter_mut().zip(r) {
            *bit = match op {
                Op::Replace => r,
                Op::Add => *bit || r,
                Op::Subtract => *bit && !r,
            };
        }
    }
}

/// Rasterize `shape` into `width * height` row-major bools, judged at pixel centres.
pub fn rasterize(shape: &Shape, width: u32, height: u32) -> Vec<bool> {
    let len = width as usize * height as usize;
    if !valid(shape) {
        return vec![false; len];
    }
    match shape {
        Shape::Polygon(points) => rasterize_polygon(points, width, height),
        Shape::Stroke { points, radius } => rasterize_stroke(points, *radius, width, height),
    }
}

/// Build a mask by applying `ops` in order; `None` when there are no operations.
pub fn build(width: u32, height: u32, ops: &[(Op, Shape)]) -> Option<Mask> {
    let first = ops.first()?;
    let value = first.0 == Op::Subtract;
    let mut mask = Mask::new(width, height, value);
    for (op, shape) in ops {
        mask.apply(*op, shape);
    }
    Some(mask)
}

/// Rule 1: reject degenerate shapes and non-finite coordinates or radii.
fn valid(shape: &Shape) -> bool {
    match shape {
        Shape::Polygon(points) => {
            points.len() >= 3 && points.iter().all(|p| p[0].is_finite() && p[1].is_finite())
        }
        Shape::Stroke { points, radius } => {
            !points.is_empty()
                && radius.is_finite()
                && *radius > 0.0
                && points.iter().all(|p| p[0].is_finite() && p[1].is_finite())
        }
    }
}

/// Rules 2 and 3: nonzero winding via a sorted crossing sweep per row.
fn rasterize_polygon(points: &[[f32; 2]], width: u32, height: u32) -> Vec<bool> {
    let w = width as usize;
    let mut bits = vec![false; w * height as usize];
    let n = points.len();
    for y in 0..height {
        let py = y as f64 + 0.5;
        let mut crossings: Vec<(f64, i32)> = Vec::new();
        for i in 0..n {
            let a = (points[i][0] as f64, points[i][1] as f64);
            let b = (points[(i + 1) % n][0] as f64, points[(i + 1) % n][1] as f64);
            let contribution = if a.1 <= py && b.1 > py {
                1
            } else if b.1 <= py && a.1 > py {
                -1
            } else {
                continue;
            };
            let xi = a.0 + (py - a.1) * (b.0 - a.0) / (b.1 - a.1);
            crossings.push((xi, contribution));
        }
        if crossings.is_empty() {
            continue;
        }
        crossings.sort_by(|c, d| c.0.partial_cmp(&d.0).unwrap_or(std::cmp::Ordering::Equal));
        let total: i32 = crossings.iter().map(|c| c.1).sum();
        let mut before = 0i32;
        let mut idx = 0usize;
        for x in 0..width {
            let px = x as f64 + 0.5;
            while idx < crossings.len() && crossings[idx].0 <= px {
                before += crossings[idx].1;
                idx += 1;
            }
            bits[y as usize * w + x as usize] = total - before != 0;
        }
    }
    bits
}

/// Rule 4: a pixel is in when its centre is within `radius` of any segment.
fn rasterize_stroke(points: &[[f32; 2]], radius: f32, width: u32, height: u32) -> Vec<bool> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let w = width as i64;
    let h = height as i64;
    let mut bits = vec![false; w as usize * h as usize];
    let radius = radius as f64;
    let r2 = radius * radius;
    let grow = radius + 1.0;
    let segments = if points.len() == 1 {
        1
    } else {
        points.len() - 1
    };
    for i in 0..segments {
        let a = (points[i][0] as f64, points[i][1] as f64);
        let b = if points.len() == 1 {
            a
        } else {
            (points[i + 1][0] as f64, points[i + 1][1] as f64)
        };
        let x0 = (a.0.min(b.0) - grow).ceil() as i64;
        let x1 = (a.0.max(b.0) + grow).floor() as i64;
        let y0 = (a.1.min(b.1) - grow).ceil() as i64;
        let y1 = (a.1.max(b.1) + grow).floor() as i64;
        let x0 = x0.clamp(0, w - 1);
        let x1 = x1.clamp(-1, w - 1);
        let y0 = y0.clamp(0, h - 1);
        let y1 = y1.clamp(-1, h - 1);
        if x1 < x0 || y1 < y0 {
            continue;
        }
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let len2 = dx * dx + dy * dy;
        for y in y0..=y1 {
            let py = y as f64 + 0.5;
            let row = y as usize * w as usize;
            for x in x0..=x1 {
                if bits[row + x as usize] {
                    continue;
                }
                let px = x as f64 + 0.5;
                let t = if len2 == 0.0 {
                    0.0
                } else {
                    (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0)
                };
                let qx = a.0 + t * dx - px;
                let qy = a.1 + t * dy - py;
                if qx * qx + qy * qy <= r2 {
                    bits[row + x as usize] = true;
                }
            }
        }
    }
    bits
}
