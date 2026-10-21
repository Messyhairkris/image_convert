//! Raster to flat-colour SVG.

use image::DynamicImage;

use crate::contour::trace;
use crate::despeckle::despeckle;
use crate::kmeans::quantize;
use crate::options::{Error, VectorOptions};
use crate::smooth::{smooth_loop, Cubic};
use crate::svg::{document, path_data};

/// Render `img` as a flat-colour SVG, running the vector pipeline in order.
pub fn render(img: &DynamicImage, opts: &VectorOptions) -> Result<String, Error> {
    // rule 1: validate before anything else.
    opts.validate()?;

    let width = img.width();
    let height = img.height();
    // rule 2: no pixels at all.
    if width == 0 || height == 0 {
        return Err(Error::EmptyImage);
    }
    let (w, h) = (width as usize, height as usize);

    // rule 3: quantize the colours.
    let rgb = img.to_rgb8();
    let palette = quantize(
        rgb.as_raw(),
        opts.colors as usize,
        opts.max_iterations as usize,
    );

    // rule 4: merge specks.
    let labels = despeckle(&palette.labels, w, h, opts.speckle as usize);

    // rule 5: the background is the most common label's colour.
    let background = background_color(&labels, &palette.colors);

    // rule 6: trace the region outlines.
    let loops = trace(&labels, w, h);

    // rule 7: one shape per palette label, in label order; a label with no
    // loops still yields a shape whose `d` is empty.
    let mut shapes: Vec<([u8; 3], String)> = Vec::with_capacity(palette.colors.len());
    for (i, color) in palette.colors.iter().enumerate() {
        let mut curves: Vec<Vec<[f64; 8]>> = Vec::new();
        for lp in loops.iter().filter(|l| l.label as usize == i) {
            let cubics = smooth_loop(&lp.points, &lp.junction, opts.smoothness as f64);
            curves.push(cubics.iter().map(cubic_row).collect());
        }
        shapes.push((*color, path_data(&curves)));
    }

    // rule 8: the document, with the image's u32 size.
    Ok(document(width, height, background, &shapes))
}

/// Rule 5: the palette colour of the label with the largest pixel count; ties
/// go to the smaller label.
fn background_color(labels: &[u8], colors: &[[u8; 3]]) -> [u8; 3] {
    let mut counts = vec![0u64; colors.len()];
    for &l in labels {
        let i = l as usize;
        if i < counts.len() {
            counts[i] += 1;
        }
    }
    // Scanning upward with a strict `>` makes the smaller label take ties.
    let mut best = 0usize;
    let mut best_count = 0u64;
    for (i, &count) in counts.iter().enumerate() {
        if count > best_count {
            best_count = count;
            best = i;
        }
    }
    colors.get(best).copied().unwrap_or([0, 0, 0])
}

/// One cubic as the eight numbers `path_data` expects.
fn cubic_row(c: &Cubic) -> [f64; 8] {
    [
        c.from.0, c.from.1, c.c1.0, c.c1.1, c.c2.0, c.c2.1, c.to.0, c.to.1,
    ]
}
