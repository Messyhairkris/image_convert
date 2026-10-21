//! Integration tests for `lineart_svg`: line art as SVG centerline strokes.

use image::{DynamicImage, Rgb, RgbImage};

use photo_tools::lineart_svg::{chain_path, render_svg, StrokeOptions};
use photo_tools::options::{EdgeFilter, Error, LineArtOptions};
use photo_tools::select::Mask;

/// The contract's `RING`: a 3 x 3 RGB image, black except a white centre.
fn ring() -> DynamicImage {
    let mut img = RgbImage::from_pixel(3, 3, Rgb([0, 0, 0]));
    img.put_pixel(1, 1, Rgb([255, 255, 255]));
    DynamicImage::ImageRgb8(img)
}

/// The contract's `SOBEL`: line art is every pixel black except the centre.
fn sobel() -> LineArtOptions {
    LineArtOptions {
        filter: EdgeFilter::Sobel,
        blur: 0.0,
        low: 361.4,
        high: 361.4,
    }
}

/// The contract's `T0`: straight lines through every corner.
fn t0() -> StrokeOptions {
    StrokeOptions {
        tolerance: 0.0,
        ..Default::default()
    }
}

/// The document head for a `w` x `h` image.
fn head(w: u32, h: u32) -> String {
    format!(
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" ",
            "width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n"
        ),
        w, h, w, h
    )
}

/// The `<path ...>` opening tag with the given stroke width.
fn path_open(width: &str) -> String {
    format!(
        concat!(
            "<path fill=\"none\" stroke=\"#000000\" stroke-width=\"{}\" ",
            "stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\""
        ),
        width
    )
}

#[test]
fn stroke_options_default_is_tolerance_1_width_1_min_pixels_3() {
    let s = StrokeOptions::default();
    assert_eq!(s.tolerance, 1.0);
    assert_eq!(s.width, 1.0);
    assert_eq!(s.min_pixels, 3);
}

#[test]
fn validate_rule_1_tolerance_outside_0_to_10() {
    let low = StrokeOptions {
        tolerance: -0.1,
        ..Default::default()
    };
    assert_eq!(
        low.validate(),
        Err(Error::InvalidOption {
            name: "tolerance",
            reason: "must be a number from 0 to 10".to_string(),
        })
    );
    let high = StrokeOptions {
        tolerance: 10.5,
        ..Default::default()
    };
    assert_eq!(
        high.validate(),
        Err(Error::InvalidOption {
            name: "tolerance",
            reason: "must be a number from 0 to 10".to_string(),
        })
    );
    // 0.0 and 10.0 are inside the inclusive range.
    assert_eq!(
        StrokeOptions {
            tolerance: 0.0,
            ..Default::default()
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        StrokeOptions {
            tolerance: 10.0,
            ..Default::default()
        }
        .validate(),
        Ok(())
    );
}

#[test]
fn validate_rule_1_tolerance_not_finite() {
    let s = StrokeOptions {
        tolerance: f64::NAN,
        ..Default::default()
    };
    assert_eq!(
        s.validate(),
        Err(Error::InvalidOption {
            name: "tolerance",
            reason: "must be a number from 0 to 10".to_string(),
        })
    );
}

#[test]
fn validate_rule_2_width_zero_or_above_20() {
    let zero = StrokeOptions {
        width: 0.0,
        ..Default::default()
    };
    assert_eq!(
        zero.validate(),
        Err(Error::InvalidOption {
            name: "stroke-width",
            reason: "must be above 0 and at most 20".to_string(),
        })
    );
    let big = StrokeOptions {
        width: 20.5,
        ..Default::default()
    };
    assert_eq!(
        big.validate(),
        Err(Error::InvalidOption {
            name: "stroke-width",
            reason: "must be above 0 and at most 20".to_string(),
        })
    );
    // 20.0 is allowed: "at most 20".
    assert_eq!(
        StrokeOptions {
            width: 20.0,
            ..Default::default()
        }
        .validate(),
        Ok(())
    );
}

#[test]
fn validate_rule_3_min_pixels_above_10000() {
    let bad = StrokeOptions {
        min_pixels: 10001,
        ..Default::default()
    };
    assert_eq!(
        bad.validate(),
        Err(Error::InvalidOption {
            name: "min-pixels",
            reason: "must be at most 10000".to_string(),
        })
    );
    let ok = StrokeOptions {
        min_pixels: 10000,
        ..Default::default()
    };
    assert_eq!(ok.validate(), Ok(()));
}

#[test]
fn validate_checks_tolerance_before_width_and_min_pixels() {
    let s = StrokeOptions {
        tolerance: -1.0,
        width: 0.0,
        min_pixels: 20000,
    };
    assert_eq!(
        s.validate(),
        Err(Error::InvalidOption {
            name: "tolerance",
            reason: "must be a number from 0 to 10".to_string(),
        })
    );
}

#[test]
fn chain_path_rule_3_single_point_is_a_dot() {
    assert_eq!(chain_path(&[(1, 1)], 1.0), "M 1.5 1.5 L 1.5 1.5");
}

#[test]
fn chain_path_rule_5_tolerance_0_straight_run_is_one_line() {
    let chain = [(0, 0), (1, 0), (2, 0), (3, 0)];
    assert_eq!(chain_path(&chain, 0.0), "M 0.5 0.5 L 3.5 0.5");
}

#[test]
fn chain_path_rule_5_straight_run_with_tolerance_1_is_one_cubic() {
    let chain = [(0, 0), (1, 0), (2, 0), (3, 0)];
    assert_eq!(chain_path(&chain, 1.0), "M 0.5 0.5 C 1 0.5 3 0.5 3.5 0.5");
}

#[test]
fn chain_path_rule_5_l_shape_catmull_rom() {
    let chain = [(0, 0), (0, 1), (0, 2), (1, 2), (2, 2)];
    assert_eq!(
        chain_path(&chain, 1.0),
        "M 0.5 0.5 C 0.5 0.83 0.17 2.17 0.5 2.5 C 0.83 2.83 2.17 2.5 2.5 2.5"
    );
}

#[test]
fn chain_path_rule_6_closed_chain_tolerance_0_ends_with_z() {
    let chain = [
        (0, 0),
        (1, 0),
        (2, 0),
        (2, 1),
        (2, 2),
        (1, 2),
        (0, 2),
        (0, 1),
        (0, 0),
    ];
    assert_eq!(
        chain_path(&chain, 0.0),
        "M 0.5 0.5 L 2.5 0.5 L 2.5 2.5 L 0.5 2.5 L 0.5 0.5 Z"
    );
}

#[test]
fn chain_path_rule_5_closed_chain_catmull_rom() {
    let chain = [(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)];
    assert_eq!(
        chain_path(&chain, 0.5),
        "M 0.5 0.5 C 1.17 -0.17 3.83 -0.17 4.5 0.5 C 5.17 1.17 5.17 3.83 4.5 4.5 \
         C 3.83 5.17 1.17 5.17 0.5 4.5 C -0.17 3.83 -0.17 1.17 0.5 0.5 Z"
    );
}

#[test]
fn render_svg_ring_sobel_t0_no_selection() {
    let want = format!(
        "{}{}M 1.5 0.5 L 2.5 1.5 L 1.5 2.5 L 0.5 1.5 L 1.5 0.5 Z\"/>\n</svg>\n",
        head(3, 3),
        path_open("1")
    );
    assert_eq!(render_svg(&ring(), &sobel(), &t0(), None), Ok(want));
}

#[test]
fn render_svg_rule_4_keeps_only_selected_pixels() {
    // Every pixel with x >= 1: the left column of the diamond is cut off.
    let mask = Mask::from_bits(
        3,
        3,
        vec![false, true, true, false, true, true, false, true, true],
    )
    .unwrap();
    let want = format!(
        "{}{}M 1.5 0.5 L 2.5 1.5 L 1.5 2.5\"/>\n</svg>\n",
        head(3, 3),
        path_open("1")
    );
    assert_eq!(render_svg(&ring(), &sobel(), &t0(), Some(&mask)), Ok(want));
}

#[test]
fn render_svg_rule_6_drops_chains_below_min_pixels() {
    // The loop has 4 distinct pixels, below min_pixels 5.
    let stroke = StrokeOptions {
        min_pixels: 5,
        ..t0()
    };
    let want = head(3, 3) + "</svg>\n";
    assert_eq!(render_svg(&ring(), &sobel(), &stroke, None), Ok(want));
}

#[test]
fn render_svg_rule_7_stroke_width_goes_through_fmt_num() {
    let stroke = StrokeOptions { width: 2.5, ..t0() };
    let want = format!(
        "{}{}M 1.5 0.5 L 2.5 1.5 L 1.5 2.5 L 0.5 1.5 L 1.5 0.5 Z\"/>\n</svg>\n",
        head(3, 3),
        path_open("2.5")
    );
    assert_eq!(render_svg(&ring(), &sobel(), &stroke, None), Ok(want));
}

#[test]
fn render_svg_rule_7_no_path_element_when_no_lines() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([255, 255, 255])));
    let opts = LineArtOptions {
        filter: EdgeFilter::Sobel,
        blur: 0.0,
        low: 100.0,
        high: 100.0,
    };
    let want = head(4, 4) + "</svg>\n";
    assert_eq!(
        render_svg(&img, &opts, &StrokeOptions::default(), None),
        Ok(want)
    );
}

#[test]
fn render_svg_rule_1_stroke_tolerance_error() {
    let stroke = StrokeOptions {
        tolerance: 11.0,
        ..Default::default()
    };
    assert_eq!(
        render_svg(&ring(), &sobel(), &stroke, None),
        Err(Error::InvalidOption {
            name: "tolerance",
            reason: "must be a number from 0 to 10".to_string(),
        })
    );
}

#[test]
fn render_svg_rule_3_selection_size_mismatch() {
    let mask = Mask::new(2, 3, true);
    assert_eq!(
        render_svg(&ring(), &sobel(), &t0(), Some(&mask)),
        Err(Error::InvalidOption {
            name: "selection",
            reason: "is 2x3 but the image is 3x3".to_string(),
        })
    );
}

#[test]
fn render_svg_rule_2_empty_image() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    assert_eq!(
        render_svg(
            &img,
            &LineArtOptions::default(),
            &StrokeOptions::default(),
            None
        ),
        Err(Error::EmptyImage)
    );
}

#[test]
fn render_svg_rule_1_options_are_validated_before_strokes() {
    // Both option sets are bad; the line-art blur error must win.
    let opts = LineArtOptions {
        blur: 60.0,
        ..Default::default()
    };
    let stroke = StrokeOptions {
        tolerance: 11.0,
        ..Default::default()
    };
    assert_eq!(
        render_svg(&ring(), &opts, &stroke, None),
        Err(Error::InvalidOption {
            name: "blur",
            reason: "must be a number from 0 to 50".to_string(),
        })
    );
}
