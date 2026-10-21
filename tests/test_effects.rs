//! Integration tests for `photo_tools::effects` (contract: effects.rs, `--mode fx`).
//!
//! Expected values are built from the accepted modules (luma, dither, halftone, hatch,
//! dodge, ascii) exactly as the contract's rules and worked-example table describe.

use image::{DynamicImage, GrayImage, Rgb, RgbImage};
use imageproc::filter::gaussian_blur_f32;
use photo_tools::ascii;
use photo_tools::dither::floyd_steinberg;
use photo_tools::dodge::dodge_layers;
use photo_tools::effects::{render, EffectOutput};
use photo_tools::halftone::halftone;
use photo_tools::hatch::hatch;
use photo_tools::luma::{invert, rgb_to_gray};
use photo_tools::options::{EffectOptions, Error, Style};

/// A deterministic RGB pattern, so every expected value is reproducible.
fn pattern_rgb(w: u32, h: u32) -> DynamicImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = (x * 13 + y * 7) % 256;
            let g = (x * 5 + y * 29 + 11) % 256;
            let b = (x * 3 + y * 17 + 40) % 256;
            img.put_pixel(x, y, Rgb([r as u8, g as u8, b as u8]));
        }
    }
    DynamicImage::ImageRgb8(img)
}

/// Rule 3: the gray bytes effects must derive from an image (Rec. 601 via luma).
fn gray_of(img: &DynamicImage) -> Vec<u8> {
    rgb_to_gray(img.to_rgb8().as_raw())
}

/// Default options for one style.
fn opts(style: Style) -> EffectOptions {
    EffectOptions {
        style,
        ..EffectOptions::default()
    }
}

/// The Image output as a whole value, built from raw bytes at a given size.
fn image_out(w: u32, h: u32, bytes: Vec<u8>) -> EffectOutput {
    EffectOutput::Image(GrayImage::from_raw(w, h, bytes).unwrap())
}

#[test]
fn rule_1_validation_runs_before_the_empty_image_check() {
    // Worked example row 3: a 0x3 image would give EmptyImage (rule 2), but rule 1 says
    // validate FIRST, so the invalid cell (1 is below 2) must win.
    let img = DynamicImage::ImageRgb8(RgbImage::new(0, 3));
    let o = EffectOptions {
        cell: 1,
        ..opts(Style::Dither)
    };
    assert_eq!(
        render(&img, &o),
        Err(Error::InvalidOption {
            name: "cell",
            reason: "must be from 2 to 64".to_string()
        })
    );
}

#[test]
fn rule_2_zero_width_or_height_is_empty_image() {
    // Worked example row 4: 0x3 with otherwise valid Dither defaults.
    let no_width = DynamicImage::ImageRgb8(RgbImage::new(0, 3));
    assert_eq!(
        render(&no_width, &opts(Style::Dither)),
        Err(Error::EmptyImage)
    );
    let no_height = DynamicImage::ImageRgb8(RgbImage::new(3, 0));
    assert_eq!(
        render(&no_height, &opts(Style::Dither)),
        Err(Error::EmptyImage)
    );
}

#[test]
fn rule_3_dither_uses_rec601_gray_dark_green_goes_black() {
    // (0, 200, 0): Rec. 601 gray = 0.587 * 200 = 117.4 -> 117 < 128 -> dithered to 0.
    // to_luma8 (Rec. 709) would give 143 -> 255, so this row catches the wrong luma.
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([0, 200, 0])));
    assert_eq!(
        render(&img, &opts(Style::Dither)),
        Ok(image_out(1, 1, vec![0]))
    );
}

#[test]
fn rule_3_dither_uses_rec601_gray_pure_green_goes_white() {
    // (0, 255, 0): Rec. 601 gray = 0.587 * 255 = 149.685 -> 150 >= 128 -> 255.
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([0, 255, 0])));
    assert_eq!(
        render(&img, &opts(Style::Dither)),
        Ok(image_out(1, 1, vec![255]))
    );
}

#[test]
fn rule_4_dither_matches_the_floyd_steinberg_module() {
    let img = pattern_rgb(16, 12);
    let gray = gray_of(&img);
    let expected = image_out(16, 12, floyd_steinberg(&gray, 16, 12));
    assert_eq!(render(&img, &opts(Style::Dither)), Ok(expected));
}

#[test]
fn rule_4_halftone_matches_the_halftone_module() {
    let img = pattern_rgb(9, 4);
    let gray = gray_of(&img);
    let o = EffectOptions {
        cell: 2,
        ..opts(Style::Halftone)
    };
    let expected = image_out(9, 4, halftone(&gray, 9, 4, 2));
    assert_eq!(render(&img, &o), Ok(expected));
}

#[test]
fn rule_4_hatch_matches_the_hatch_module() {
    // Worked example row 5: 20x12, Hatch, cell 5 (the cell option is the hatch spacing).
    let img = pattern_rgb(20, 12);
    let gray = gray_of(&img);
    let o = EffectOptions {
        cell: 5,
        ..opts(Style::Hatch)
    };
    let expected = image_out(20, 12, hatch(&gray, 20, 12, 5));
    assert_eq!(render(&img, &o), Ok(expected));
}

#[test]
fn rule_4_sketch_is_dodge_of_the_blurred_inverted_gray() {
    // Worked example row 8: 20x12, Sketch, blur 3.0.
    let img = pattern_rgb(20, 12);
    let gray = gray_of(&img);
    let inverted = GrayImage::from_raw(20, 12, invert(&gray)).unwrap();
    let blurred = gaussian_blur_f32(&inverted, 3.0);
    let expected = image_out(20, 12, dodge_layers(&gray, blurred.as_raw()));
    let o = EffectOptions {
        blur: 3.0,
        ..opts(Style::Sketch)
    };
    assert_eq!(render(&img, &o), Ok(expected));
}

#[test]
fn rule_4_ascii_plain_matches_the_ascii_module() {
    // Worked example row 6: 20x12, Ascii, columns 7, no colour, no invert.
    let img = pattern_rgb(20, 12);
    let gray = gray_of(&img);
    let o = EffectOptions {
        columns: 7,
        ..opts(Style::Ascii)
    };
    let expected = ascii::render(&gray, None, 20, 12, 7, false);
    assert_eq!(render(&img, &o), Ok(EffectOutput::Text(expected)));
}

#[test]
fn rule_4_ascii_ansi_and_invert_match_the_ascii_module() {
    // Worked example row 7: 20x12, Ascii, columns 5, ansi, invert. The RGB bytes (not
    // the gray) must be passed as the colour source.
    let img = pattern_rgb(20, 12);
    let gray = gray_of(&img);
    let rgb = img.to_rgb8();
    let o = EffectOptions {
        columns: 5,
        ansi: true,
        invert: true,
        ..opts(Style::Ascii)
    };
    let expected = ascii::render(&gray, Some(rgb.as_raw()), 20, 12, 5, true);
    assert_eq!(render(&img, &o), Ok(EffectOutput::Text(expected)));
}

#[test]
fn rule_5_image_output_has_the_input_width_and_height() {
    // Non-square input: comparing against from_raw(7, 5, ..) fails if the module swaps
    // or shrinks the output dimensions (ImageBuffer equality includes dimensions).
    let img = pattern_rgb(7, 5);
    let gray = gray_of(&img);
    let expected = image_out(7, 5, floyd_steinberg(&gray, 7, 5));
    assert_eq!(render(&img, &opts(Style::Dither)), Ok(expected));
}
