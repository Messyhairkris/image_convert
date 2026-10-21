//! Integration tests for `src/lineart.rs` (the `--mode lineart` engine).

use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;
use photo_tools::lineart::render;
use photo_tools::luma::rgb_to_gray;
use photo_tools::options::{EdgeFilter, Error, LineArtOptions};

fn sobel_opts(blur: f32, low: f32, high: f32) -> LineArtOptions {
    LineArtOptions {
        filter: EdgeFilter::Sobel,
        blur,
        low,
        high,
    }
}

/// RGB bytes of a `width`x`height` image: left half white, right half black.
fn half_white_raw(width: u32, height: u32) -> Vec<u8> {
    let mut raw = Vec::new();
    for _y in 0..height {
        for x in 0..width {
            let v = if x * 2 < width { 255 } else { 0 };
            raw.extend_from_slice(&[v, v, v]);
        }
    }
    raw
}

/// 3x3 RGB image, black except a white centre pixel. Its Sobel magnitudes are
/// [361, 510, 361, 510, 0, 510, 361, 510, 361] (rule 5 worked examples).
fn white_centre_3x3() -> DynamicImage {
    let mut raw = vec![0u8; 27];
    // centre pixel (1, 1) occupies bytes 12..15
    raw[12] = 255;
    raw[13] = 255;
    raw[14] = 255;
    DynamicImage::ImageRgb8(RgbImage::from_raw(3, 3, raw).unwrap())
}

/// Rule 1: `validate` runs first, so an invalid option is reported even for an
/// empty image.
#[test]
fn rule_1_invalid_option_is_reported_even_for_an_empty_image() {
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    let opts = LineArtOptions {
        filter: EdgeFilter::Sobel,
        blur: 60.0,
        low: 50.0,
        high: 100.0,
    };
    let want = Error::InvalidOption {
        name: "blur",
        reason: "must be a number from 0 to 50".to_string(),
    };
    assert_eq!(render(&img, &opts), Err(want));
}

/// Rule 2: a zero in either dimension means `Error::EmptyImage` (both table rows).
#[test]
fn rule_2_zero_sized_image_is_empty_image() {
    let opts = LineArtOptions::default();
    let zero_zero = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
    assert_eq!(render(&zero_zero, &opts), Err(Error::EmptyImage));
    let five_zero = DynamicImage::ImageRgb8(RgbImage::new(5, 0));
    assert_eq!(render(&five_zero, &opts), Err(Error::EmptyImage));
}

/// Rule 3: graying uses Rec. 601 (`rgb_to_gray` of `to_rgb8`), never `to_luma8`.
#[test]
fn rule_3_red_pixels_use_rec601_not_to_luma8() {
    // Rec. 601 gray is [76, 76, 0], magnitudes [0, 304, 304]: threshold 300
    // blacks only the last two pixels. With `to_luma8` (gray 54, magnitude
    // 216 < 300) all three would stay white.
    let raw = vec![255, 0, 0, 255, 0, 0, 0, 0, 0];
    let img = DynamicImage::ImageRgb8(RgbImage::from_raw(3, 1, raw).unwrap());
    // high is 900, not 600: the Sobel path must not use `opts.high`
    let out = render(&img, &sobel_opts(0.0, 300.0, 900.0)).unwrap();
    assert_eq!(out.into_raw(), vec![255, 0, 0]);
}

/// Rule 3: Luma8 and RGBA variants go through `to_rgb8` the same way RGB does.
#[test]
fn rule_3_luma8_and_rgba_inputs_behave_like_rgb() {
    let luma8 = DynamicImage::ImageLuma8(GrayImage::from_raw(3, 1, vec![76, 76, 0]).unwrap());
    let rgba_raw = vec![255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 255];
    let rgba = DynamicImage::ImageRgba8(RgbaImage::from_raw(3, 1, rgba_raw).unwrap());
    let opts = sobel_opts(0.0, 300.0, 900.0);
    // both decode to gray [76, 76, 0], so both give the RGB row's output
    assert_eq!(render(&luma8, &opts).unwrap().into_raw(), vec![255, 0, 0]);
    assert_eq!(render(&rgba, &opts).unwrap().into_raw(), vec![255, 0, 0]);
}

/// Rule 4: sigma 0 means no blur at all (`gaussian_blur_f32` would panic on 0).
#[test]
fn rule_4_blur_zero_never_calls_gaussian_blur_f32() {
    let raw = half_white_raw(8, 8);
    let img = DynamicImage::ImageRgb8(RgbImage::from_raw(8, 8, raw.clone()).unwrap());
    let opts = LineArtOptions {
        filter: EdgeFilter::Canny,
        blur: 0.0,
        low: 45.0,
        high: 90.0,
    };
    let gray = GrayImage::from_raw(8, 8, rgb_to_gray(&raw)).unwrap();
    let want: Vec<u8> = canny(&gray, 45.0, 90.0)
        .into_raw()
        .iter()
        .map(|v| 255 - v)
        .collect();
    let out = render(&img, &opts).unwrap();
    assert_eq!(out.into_raw(), want);
}

/// Rule 4: a positive sigma blurs with `gaussian_blur_f32` before Canny (table row).
#[test]
fn rule_4_positive_blur_blurs_before_canny() {
    let raw = half_white_raw(8, 8);
    let img = DynamicImage::ImageRgb8(RgbImage::from_raw(8, 8, raw.clone()).unwrap());
    let opts = LineArtOptions {
        filter: EdgeFilter::Canny,
        blur: 1.2,
        low: 45.0,
        high: 90.0,
    };
    let gray = GrayImage::from_raw(8, 8, rgb_to_gray(&raw)).unwrap();
    let want: Vec<u8> = canny(&gaussian_blur_f32(&gray, 1.2), 45.0, 90.0)
        .into_raw()
        .iter()
        .map(|v| 255 - v)
        .collect();
    let out = render(&img, &opts).unwrap();
    assert_eq!(out.into_raw(), want);
}

/// Rule 5 (Sobel): threshold 400 keeps only the 510-magnitude pixels black.
#[test]
fn rule_5_sobel_low_400_edges_only() {
    let out = render(&white_centre_3x3(), &sobel_opts(0.0, 400.0, 900.0)).unwrap();
    assert_eq!(out.into_raw(), vec![255, 0, 255, 0, 255, 0, 255, 0, 255]);
}

/// Rule 5 (Sobel): 361.4 rounds to 361, and 361 >= 361 blacks the corners too.
#[test]
fn rule_5_sobel_low_361_4_rounds_down() {
    let out = render(&white_centre_3x3(), &sobel_opts(0.0, 361.4, 900.0)).unwrap();
    assert_eq!(out.into_raw(), vec![0, 0, 0, 0, 255, 0, 0, 0, 0]);
}

/// Rule 5 (Sobel): 361.6 rounds to 362, so the 361-magnitude corners stay white.
#[test]
fn rule_5_sobel_low_361_6_rounds_up() {
    let out = render(&white_centre_3x3(), &sobel_opts(0.0, 361.6, 900.0)).unwrap();
    assert_eq!(out.into_raw(), vec![255, 0, 255, 0, 255, 0, 255, 0, 255]);
}

/// Rule 6: the output keeps the input's width and height.
#[test]
fn rule_6_output_matches_input_dimensions() {
    let raw: Vec<u8> = (0..45u32).map(|i| (i * 5 % 256) as u8).collect();
    let img = DynamicImage::ImageRgb8(RgbImage::from_raw(5, 3, raw).unwrap());
    let out = render(&img, &sobel_opts(0.0, 100.0, 900.0)).unwrap();
    assert_eq!(out.dimensions(), (5, 3));
    assert_eq!(out.as_raw().len(), 15);
}
