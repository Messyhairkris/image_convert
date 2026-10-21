//! Amplitude-modulated halftone dots.

use std::f64::consts::SQRT_2;

/// One black dot per cell, its area following the cell's darkness.
///
/// `gray` is row-major (`gray[y * width + x]`) with `gray.len() == width * height`;
/// the result has the same layout and every byte is 0 (black) or 255 (white).
pub fn halftone(gray: &[u8], width: usize, height: usize, cell: usize) -> Vec<u8> {
    // Rule 1: cells narrower than 2 are widened to 2.
    let c = if cell < 2 { 2 } else { cell };
    let mut out = vec![255u8; width * height];
    let nx = width.div_ceil(c);
    let ny = height.div_ceil(c);
    for cy in 0..ny {
        for cx in 0..nx {
            // Rule 2: the cell is clipped to the image at the right and bottom edges.
            let x0 = cx * c;
            let y0 = cy * c;
            let x1 = ((cx + 1) * c).min(width);
            let y1 = ((cy + 1) * c).min(height);
            // Rule 3: mean over the pixels inside the image, u32 truncating division.
            let mut sum: u32 = 0;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += gray[y * width + x] as u32;
                }
            }
            let count = ((x1 - x0) * (y1 - y0)) as u32;
            let mean = sum / count;
            // Rule 4: full darkness reaches the cell's corners.
            let darkness = (255 - mean) as f64 / 255.0;
            let r = c as f64 * 0.5 * SQRT_2 * darkness.sqrt();
            let r2 = r * r;
            // Rule 5: centre of the FULL cell, even when the cell is clipped.
            let centre_x = (cx * c) as f64 + c as f64 / 2.0;
            let centre_y = (cy * c) as f64 + c as f64 / 2.0;
            // Rule 6: strict comparison, so r = 0 leaves every pixel white.
            for y in y0..y1 {
                for x in x0..x1 {
                    let dx = x as f64 + 0.5 - centre_x;
                    let dy = y as f64 + 0.5 - centre_y;
                    if dx * dx + dy * dy < r2 {
                        out[y * width + x] = 0;
                    }
                }
            }
        }
    }
    out
}
