//! Flat-colour SVG of the selected area only; the rest is left transparent.

use image::DynamicImage;

use crate::contour::trace;
use crate::despeckle::despeckle;
use crate::kmeans::quantize;
use crate::options::{Error, VectorOptions};
use crate::select::Mask;
use crate::smooth::smooth_loop;
use crate::svg::{hex, path_data};

/// Like vector::render, but the palette is fitted to the selected pixels only and
/// nothing outside the selection is drawn (no background rectangle).
pub fn render_selected(
    img: &DynamicImage,
    opts: &VectorOptions,
    keep: &Mask,
) -> Result<String, Error> {
    // rule 1: validate before anything else.
    opts.validate()?;

    let width = img.width();
    let height = img.height();
    // rule 2: no pixels at all.
    if width == 0 || height == 0 {
        return Err(Error::EmptyImage);
    }
    let (w, h) = (width as usize, height as usize);

    // rule 3: the selection must be the size of the image.
    let (kw, kh) = (keep.width(), keep.height());
    if kw != width || kh != height {
        return Err(Error::InvalidOption {
            name: "selection",
            reason: format!("is {}x{} but the image is {}x{}", kw, kh, width, height),
        });
    }

    // rule 4: the header, and an empty document when nothing is selected.
    let mut out = String::new();
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    out.push_str(&format!(" width=\"{}\" height=\"{}\"", width, height));
    out.push_str(&format!(" viewBox=\"0 0 {} {}\">\n", width, height));
    if keep.count() == 0 {
        out.push_str("</svg>\n");
        return Ok(out);
    }

    // rule 5: the palette is fitted to the selected pixels only, in raster order.
    let rgb = img.to_rgb8();
    let raw = rgb.as_raw();
    let bits = keep.bits();
    let mut chosen: Vec<u8> = Vec::new();
    for (i, &sel) in bits.iter().enumerate() {
        if sel {
            chosen.push(raw[i * 3]);
            chosen.push(raw[i * 3 + 1]);
            chosen.push(raw[i * 3 + 2]);
        }
    }
    let palette = quantize(&chosen, opts.colors as usize, opts.max_iterations as usize);

    // rule 6: label the whole image; an unselected pixel takes the outside label.
    let outside = palette.colors.len() as u8;
    let mut labels = vec![outside; w * h];
    let mut next = 0usize;
    for (i, &sel) in bits.iter().enumerate() {
        if sel {
            labels[i] = palette.labels[next];
            next += 1;
        }
    }

    // rule 7: despeckle, then put every unselected pixel back outside so that
    // despeckling cannot fill holes in the selection.
    let mut labels = despeckle(&labels, w, h, opts.speckle as usize);
    for (i, &sel) in bits.iter().enumerate() {
        if !sel {
            labels[i] = outside;
        }
    }

    // rule 8: one path per palette label, in label order; the outside label is
    // never drawn.
    let loops = trace(&labels, w, h);
    for (i, color) in palette.colors.iter().enumerate() {
        let mut curves: Vec<Vec<[f64; 8]>> = Vec::new();
        for lp in loops.iter().filter(|l| l.label as usize == i) {
            let cubics = smooth_loop(&lp.points, &lp.junction, opts.smoothness as f64);
            let rows: Vec<[f64; 8]> = cubics
                .iter()
                .map(|c| {
                    [
                        c.from.0, c.from.1, c.c1.0, c.c1.1, c.c2.0, c.c2.1, c.to.0, c.to.1,
                    ]
                })
                .collect();
            curves.push(rows);
        }
        let d = path_data(&curves);
        if !d.is_empty() {
            out.push_str(&format!(
                "<path fill=\"{}\" fill-rule=\"evenodd\" d=\"{}\"/>\n",
                hex(*color),
                d
            ));
        }
    }

    // rule 9: close the document.
    out.push_str("</svg>\n");
    Ok(out)
}
