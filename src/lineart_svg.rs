//! Line art as SVG centerline strokes.

use image::DynamicImage;

use crate::lineart;
use crate::options::{Error, LineArtOptions};
use crate::select::Mask;
use crate::smooth::{simplify, Pt};
use crate::strokes::trace;
use crate::svg::fmt_num;
use crate::thin::thin;

/// How the strokes are drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeOptions {
    /// Simplification tolerance in pixels; 0 draws straight lines through every corner
    /// (only points in a straight run are dropped, so nothing is lost).
    pub tolerance: f64,
    /// Stroke width in pixels.
    pub width: f64,
    /// Chains with fewer distinct pixels are dropped (specks).
    pub min_pixels: usize,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0,
            width: 1.0,
            min_pixels: 3,
        }
    }
}

impl StrokeOptions {
    /// Checks in contract order: tolerance, then width, then min_pixels.
    pub fn validate(&self) -> Result<(), Error> {
        if !self.tolerance.is_finite() || self.tolerance < 0.0 || self.tolerance > 10.0 {
            return Err(Error::InvalidOption {
                name: "tolerance",
                reason: "must be a number from 0 to 10".to_string(),
            });
        }
        if !self.width.is_finite() || self.width <= 0.0 || self.width > 20.0 {
            return Err(Error::InvalidOption {
                name: "stroke-width",
                reason: "must be above 0 and at most 20".to_string(),
            });
        }
        if self.min_pixels > 10000 {
            return Err(Error::InvalidOption {
                name: "min-pixels",
                reason: "must be at most 10000".to_string(),
            });
        }
        Ok(())
    }
}

/// One chain as SVG path data: pixel centres, simplified by `tolerance`, then straight
/// lines (tolerance 0) or Catmull-Rom cubics; a closed chain ends with Z.
pub fn chain_path(chain: &[(u32, u32)], tolerance: f64) -> String {
    // Rule 1: pixel centres, simplified.
    let pts: Vec<Pt> = chain
        .iter()
        .map(|&(x, y)| (x as f64 + 0.5, y as f64 + 0.5))
        .collect();
    let q = simplify(&pts, tolerance);

    // Rule 2: the M command.
    let (x0, y0) = q[0];
    let mut d = format!("M {} {}", fmt_num(x0), fmt_num(y0));

    // Rule 3: one point is a dot; round caps draw it.
    if q.len() == 1 {
        d.push_str(&format!(" L {} {}", fmt_num(x0), fmt_num(y0)));
        return d;
    }

    // Rule 4: a closed chain repeats its first point at the end.
    let closed = q[0] == q[q.len() - 1] && q.len() > 2;
    let m = q.len() - 1;

    // Rule 5: one command per segment.
    for i in 0..m {
        let a = q[i];
        let b = q[i + 1];
        if tolerance == 0.0 {
            d.push_str(&format!(" L {} {}", fmt_num(b.0), fmt_num(b.1)));
            continue;
        }
        let p0 = if i > 0 {
            q[i - 1]
        } else if closed {
            q[m - 1]
        } else {
            a
        };
        let p3 = if i + 1 < m {
            q[i + 2]
        } else if closed {
            q[1]
        } else {
            b
        };
        let c1 = (a.0 + (b.0 - p0.0) / 6.0, a.1 + (b.1 - p0.1) / 6.0);
        let c2 = (b.0 - (p3.0 - a.0) / 6.0, b.1 - (p3.1 - a.1) / 6.0);
        d.push_str(&format!(
            " C {} {} {} {} {} {}",
            fmt_num(c1.0),
            fmt_num(c1.1),
            fmt_num(c2.0),
            fmt_num(c2.1),
            fmt_num(b.0),
            fmt_num(b.1)
        ));
    }

    // Rule 6: close the loop.
    if closed {
        d.push_str(" Z");
    }
    d
}

/// Line art (black on white), thinned and traced into stroked centerlines. Only
/// pixels inside `keep` are traced; the document has no background (transparent).
pub fn render_svg(
    img: &DynamicImage,
    opts: &LineArtOptions,
    stroke: &StrokeOptions,
    keep: Option<&Mask>,
) -> Result<String, Error> {
    // Rule 1: validate before anything else.
    opts.validate()?;
    stroke.validate()?;

    // Rule 2: line art; also refuses a 0 x 0 image.
    let gray = lineart::render(img, opts)?;
    let w = img.width();
    let h = img.height();

    // Rule 3: the selection must cover exactly the image.
    if let Some(m) = keep {
        if m.width() != w || m.height() != h {
            return Err(Error::InvalidOption {
                name: "selection",
                reason: format!(
                    "is {}x{} but the image is {}x{}",
                    m.width(),
                    m.height(),
                    w,
                    h
                ),
            });
        }
    }

    // Rule 4: black line pixels inside the selection, row-major.
    let fg: Vec<bool> = match keep {
        Some(m) => gray
            .as_raw()
            .iter()
            .zip(m.bits())
            .map(|(&v, &sel)| v == 0 && sel)
            .collect(),
        None => gray.as_raw().iter().map(|&v| v == 0).collect(),
    };

    // Rules 5 and 6: thin, trace, drop specks, one path per kept chain.
    let wu = w as usize;
    let hu = h as usize;
    let chains = trace(&thin(&fg, wu, hu), wu, hu);
    let mut paths: Vec<String> = Vec::new();
    for chain in &chains {
        let distinct = if chain.len() > 1 && chain[0] == chain[chain.len() - 1] {
            chain.len() - 1
        } else {
            chain.len()
        };
        if distinct < stroke.min_pixels {
            continue;
        }
        paths.push(chain_path(chain, stroke.tolerance));
    }

    // Rule 7: the document; no background rectangle.
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        w, h, w, h
    );
    if !paths.is_empty() {
        out.push_str("<path fill=\"none\" stroke=\"#000000\" ");
        out.push_str(&format!("stroke-width=\"{}\" ", fmt_num(stroke.width)));
        out.push_str("stroke-linecap=\"round\" stroke-linejoin=\"round\" ");
        out.push_str(&format!("d=\"{}\"/>\n", paths.join(" ")));
    }
    out.push_str("</svg>\n");
    Ok(out)
}
