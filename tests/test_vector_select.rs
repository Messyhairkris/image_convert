//! Integration tests for `vector_select::render_selected`, written from its contract.

use image::{DynamicImage, RgbImage};
use photo_tools::options::{Error, VectorOptions};
use photo_tools::select::Mask;
use photo_tools::vector::render;
use photo_tools::vector_select::render_selected;

const HEAD: &str =
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"1\" viewBox=\"0 0 2 1\">\n";

const RED_PATH: &str = "<path fill=\"#ff0000\" fill-rule=\"evenodd\" d=\"M 1 0 C 1 0.33 1 0.67 1 1 C 0.67 1 0.33 1 0 1 C 0 0.67 0 0.33 0 0 C 0.33 0 0.67 0 1 0 Z\"/>\n";

const BLUE_PATH: &str = "<path fill=\"#0000ff\" fill-rule=\"evenodd\" d=\"M 1 0 C 1.33 0 1.67 0 2 0 C 2 0.33 2 0.67 2 1 C 1.67 1 1.33 1 1 1 C 1 0.67 1 0.33 1 0 Z\"/>\n";

/// The 2 x 1 image red | blue.
fn rb() -> DynamicImage {
    let img = RgbImage::from_raw(2, 1, vec![255, 0, 0, 0, 0, 255]).unwrap();
    DynamicImage::ImageRgb8(img)
}

/// VO from the worked-example table.
fn vo() -> VectorOptions {
    VectorOptions {
        colors: 2,
        smoothness: 0.0,
        speckle: 0,
        max_iterations: 5,
    }
}

/// Rule 1: an invalid option fails before anything else.
#[test]
fn rule_1_invalid_colors_returns_validate_error() {
    let opts = VectorOptions { colors: 1, ..vo() };
    let keep = Mask::new(2, 1, true);
    let got = render_selected(&rb(), &opts, &keep);
    assert_eq!(
        got,
        Err(Error::InvalidOption {
            name: "colors",
            reason: "must be from 2 to 64".to_string(),
        })
    );
}

/// Rule 2: an image with zero width or height is EmptyImage.
#[test]
fn rule_2_empty_image_returns_empty_image_error() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    let keep = Mask::new(0, 0, false);
    assert_eq!(render_selected(&img, &vo(), &keep), Err(Error::EmptyImage));
}

/// Rule 3: a mask whose size differs from the image's is refused.
#[test]
fn rule_3_mismatched_mask_returns_selection_error() {
    let keep = Mask::new(1, 1, true);
    let got = render_selected(&rb(), &vo(), &keep);
    assert_eq!(
        got,
        Err(Error::InvalidOption {
            name: "selection",
            reason: "is 1x1 but the image is 2x1".to_string(),
        })
    );
}

/// Rule 4: no selected pixels means the header and the closing tag, no shapes.
#[test]
fn rule_4_keep_none_has_no_shapes() {
    let keep = Mask::new(2, 1, false);
    let got = render_selected(&rb(), &vo(), &keep).unwrap();
    let want = format!("{HEAD}</svg>\n");
    assert_eq!(got, want);
}

/// Rules 5-8 with only the left pixel kept: the palette fits the red pixel
/// and only the red region is drawn.
#[test]
fn rule_5_to_8_keep_left_pixel_only() {
    let keep = Mask::from_bits(2, 1, vec![true, false]).unwrap();
    let got = render_selected(&rb(), &vo(), &keep).unwrap();
    let want = format!("{HEAD}{RED_PATH}</svg>\n");
    assert_eq!(got, want);
}

/// Rule 8 with both pixels kept: the exact worked-example row, blue path
/// before red, both inside the selection.
#[test]
fn rule_8_keep_both_exact_document() {
    let keep = Mask::new(2, 1, true);
    let got = render_selected(&rb(), &vo(), &keep).unwrap();
    let want = format!("{HEAD}{BLUE_PATH}{RED_PATH}</svg>\n");
    assert_eq!(got, want);
}

/// Final contract paragraph: with every pixel selected the output is
/// `vector::render`'s document without its `<rect .../>` line.
#[test]
fn keep_all_equals_vector_render_without_rect_line() {
    let keep = Mask::new(2, 1, true);
    let got = render_selected(&rb(), &vo(), &keep).unwrap();
    let full = render(&rb(), &vo()).unwrap();
    // vector::render draws one rect line; the background is the smaller label
    // on a tie, and label 0 is blue here.
    let want = full.replacen("<rect width=\"2\" height=\"1\" fill=\"#0000ff\"/>\n", "", 1);
    assert_eq!(got, want);
}
