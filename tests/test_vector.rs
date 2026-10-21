//! Integration tests for the vectorization engine (`--mode vector`).

use image::{DynamicImage, RgbImage};
use photo_tools::contour::trace;
use photo_tools::despeckle::despeckle;
use photo_tools::kmeans::quantize;
use photo_tools::options::{Error, VectorOptions};
use photo_tools::smooth::smooth_loop;
use photo_tools::svg::{document, path_data};
use photo_tools::vector::render;

fn opts(colors: u8, smoothness: f32, speckle: u32, max_iterations: u32) -> VectorOptions {
    VectorOptions {
        colors,
        smoothness,
        speckle,
        max_iterations,
    }
}

fn rgb_image(width: u32, height: u32, buf: Vec<u8>) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, buf).unwrap())
}

// Rules 3-8 applied with the accepted modules, independent of `render`'s wiring.
fn pipeline(img: &DynamicImage, o: &VectorOptions) -> String {
    let rgb = img.to_rgb8();
    let w = img.width() as usize;
    let h = img.height() as usize;
    let palette = quantize(rgb.as_raw(), o.colors as usize, o.max_iterations as usize);
    let labels = despeckle(&palette.labels, w, h, o.speckle as usize);
    let mut counts = vec![0usize; palette.colors.len()];
    for &l in &labels {
        counts[l as usize] += 1;
    }
    let mut bg = 0;
    for i in 1..counts.len() {
        if counts[i] > counts[bg] {
            bg = i;
        }
    }
    let background = palette.colors[bg];
    let loops = trace(&labels, w, h);
    let mut shapes: Vec<([u8; 3], String)> = Vec::new();
    for (i, color) in palette.colors.iter().enumerate() {
        let mut cur: Vec<Vec<[f64; 8]>> = Vec::new();
        for lp in &loops {
            if lp.label as usize != i {
                continue;
            }
            let cubics = smooth_loop(&lp.points, &lp.junction, o.smoothness as f64);
            let segs: Vec<[f64; 8]> = cubics
                .iter()
                .map(|c| {
                    [
                        c.from.0, c.from.1, c.c1.0, c.c1.1, c.c2.0, c.c2.1, c.to.0, c.to.1,
                    ]
                })
                .collect();
            cur.push(segs);
        }
        shapes.push((*color, path_data(&cur)));
    }
    document(img.width(), img.height(), background, &shapes)
}

// Rule 1: opts.validate() runs first, so invalid colors beat an empty image.
#[test]
fn rule_1_validate_runs_before_size_check() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    let got = render(&img, &opts(1, 0.0, 0, 20));
    let want = Err(Error::InvalidOption {
        name: "colors",
        reason: "must be from 2 to 64".to_string(),
    });
    assert_eq!(got, want);
}

// Rule 2: a valid options struct on a 0x0 image gives EmptyImage.
#[test]
fn rule_2_empty_image_error() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    assert_eq!(render(&img, &opts(8, 0.0, 0, 20)), Err(Error::EmptyImage));
}

// Rules 3-8, worked example: the exact 2x1 red/blue document.
#[test]
fn worked_example_2x1_exact_document() {
    let img = rgb_image(2, 1, vec![255, 0, 0, 0, 0, 255]);
    let got = render(&img, &opts(2, 0.0, 0, 20)).unwrap();
    let blue = concat!(
        "<path fill=\"#0000ff\" fill-rule=\"evenodd\" d=\"M 1 0",
        " C 1.33 0 1.67 0 2 0 C 2 0.33 2 0.67 2 1",
        " C 1.67 1 1.33 1 1 1 C 1 0.67 1 0.33 1 0 Z\"/>"
    );
    let red = concat!(
        "<path fill=\"#ff0000\" fill-rule=\"evenodd\" d=\"M 1 0",
        " C 1 0.33 1 0.67 1 1 C 0.67 1 0.33 1 0 1",
        " C 0 0.67 0 0.33 0 0 C 0.33 0 0.67 0 1 0 Z\"/>"
    );
    let want = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"1\" viewBox=\"0 0 2 1\">\n\
         <rect width=\"2\" height=\"1\" fill=\"#0000ff\"/>\n\
         {blue}\n{red}\n</svg>\n"
    );
    assert_eq!(got, want);
}

// Rule 5: the background is the label with the most pixels (3 black > 2 yellow).
#[test]
fn rule_5_background_is_the_most_common_label() {
    let mut buf = Vec::new();
    for _ in 0..2 {
        buf.extend_from_slice(&[255, 255, 0]);
    }
    for _ in 0..3 {
        buf.extend_from_slice(&[0, 0, 0]);
    }
    let img = rgb_image(5, 1, buf);
    let got = render(&img, &opts(2, 0.0, 0, 20)).unwrap();
    let rect = got.lines().nth(1).unwrap();
    assert_eq!(rect, "<rect width=\"5\" height=\"1\" fill=\"#000000\"/>");
}

// Rule 5: a 1-1 tie goes to the smaller label, which is 0 (blue, bin key 31).
#[test]
fn rule_5_background_tie_goes_to_smaller_label() {
    let img = rgb_image(2, 1, vec![255, 0, 0, 0, 0, 255]);
    let got = render(&img, &opts(2, 0.0, 0, 20)).unwrap();
    let rect = got.lines().nth(1).unwrap();
    assert_eq!(rect, "<rect width=\"2\" height=\"1\" fill=\"#0000ff\"/>");
}

// Rules 3-7: a single-colour 3x2 image quantizes to one colour and one path.
// Cubics from corners (0,0),(3,0),(3,2),(0,2) with c1 = a + (b-a)/3, c2 = b -
// (b-a)/3 (smoothness 0), e.g. (3,0)->(3,2) gives c1 = (3, 0.67), c2 = (3, 1.33).
#[test]
fn rules_3_to_7_single_colour_image_full_document() {
    let mut buf = Vec::new();
    for _ in 0..6 {
        buf.extend_from_slice(&[10, 200, 30]);
    }
    let img = rgb_image(3, 2, buf);
    let got = render(&img, &opts(8, 0.0, 0, 20)).unwrap();
    let path = concat!(
        "<path fill=\"#0ac81e\" fill-rule=\"evenodd\" d=\"M 0 0",
        " C 1 0 2 0 3 0 C 3 0.67 3 1.33 3 2",
        " C 2 2 1 2 0 2 C 0 1.33 0 0.67 0 0 Z\"/>"
    );
    let want = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"3\" height=\"2\" viewBox=\"0 0 3 2\">\n\
         <rect width=\"3\" height=\"2\" fill=\"#0ac81e\"/>\n\
         {path}\n</svg>\n"
    );
    assert_eq!(got, want);
}

// Rule 4: a 1-pixel speck (area 1 < speckle 2) merges into its neighbour, so the
// 6x6 image has 2 loops (2 "Z" markers); with speckle 0 it has 4 (each region's
// outline plus the hole and the speck).
#[test]
fn rule_4_despeckle_merges_the_speck_before_tracing() {
    let dark = [20u8, 20, 20];
    let light = [240u8, 240, 240];
    let mut buf = Vec::new();
    for y in 0..6u32 {
        for x in 0..6u32 {
            let px = if (x < 3) == !(x == 1 && y == 1) {
                &dark
            } else {
                &light
            };
            buf.extend_from_slice(px);
        }
    }
    let img = rgb_image(6, 6, buf);
    let merged = render(&img, &opts(2, 0.0, 2, 20)).unwrap();
    let kept = render(&img, &opts(2, 0.0, 0, 20)).unwrap();
    assert_eq!(merged.matches(" Z").count(), 2);
    assert_eq!(kept.matches(" Z").count(), 4);
    assert_eq!(merged, pipeline(&img, &opts(2, 0.0, 2, 20)));
}

// Rules 3-8: a larger 16x12 patterned image matches the document built from the
// accepted modules step by step (smoothness 1.0 exercises the curved branch).
#[test]
fn rules_3_to_8_larger_image_matches_pipeline_document() {
    let mut buf = Vec::new();
    for y in 0..12u32 {
        for x in 0..16u32 {
            buf.push((x.wrapping_mul(15)) as u8);
            buf.push((y.wrapping_mul(20)) as u8);
            buf.push(((x + y).wrapping_mul(11)) as u8);
        }
    }
    let img = rgb_image(16, 12, buf);
    let o = opts(8, 1.0, 4, 20);
    assert_eq!(render(&img, &o), Ok(pipeline(&img, &o)));
}
