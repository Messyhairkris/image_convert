//! Integration tests for `photo_tools::composite` module.

use image::{GrayAlphaImage, GrayImage};
use photo_tools::composite::{blank_outside, erase, flatten, keep, opaque};
use photo_tools::options::Error;
use photo_tools::select::Mask;

#[test]
fn rule_1_keep_size_mismatch() {
    // Image 3x1, mask 2x1 -> error, image unchanged (immutable).
    let img = GrayImage::from_vec(3, 1, vec![10, 20, 30]).unwrap();
    let mask = Mask::new(2, 1, true);
    let err = keep(&img, &mask).unwrap_err();
    match err {
        Error::InvalidOption { name, reason } => {
            assert_eq!(name, "selection");
            assert_eq!(reason, "is 2x1 but the image is 3x1");
        }
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn rule_1_erase_size_mismatch() {
    // 2x2 image, 2x1 mask -> error, image unchanged.
    let mut img = GrayAlphaImage::from_vec(
        2,
        2,
        vec![
            10, 255, 20, 255, // row 0
            30, 255, 40, 255, // row 1
        ],
    )
    .unwrap();
    let before = img.clone();
    let mask = Mask::new(2, 1, true);
    let err = erase(&mut img, &mask).unwrap_err();
    match err {
        Error::InvalidOption { name, reason } => {
            assert_eq!(name, "selection");
            assert_eq!(reason, "is 2x1 but the image is 2x2");
        }
        _ => panic!("unexpected error variant"),
    }
    assert_eq!(img.as_raw(), before.as_raw());
}

#[test]
fn rule_2_blank_outside_length_mismatch() {
    // 3 bytes buffer, mask 4 pixels -> error, buffer unchanged.
    let mut gray = vec![5u8, 6, 7];
    let mask = Mask::new(2, 2, true); // 4 pixels
    let err = blank_outside(&mut gray, &mask, false).unwrap_err();
    match err {
        Error::InvalidOption { name, reason } => {
            assert_eq!(name, "selection");
            assert_eq!(reason, "has 4 pixels but the image has 3");
        }
        _ => panic!("unexpected error variant"),
    }
    assert_eq!(gray, vec![5, 6, 7]);
}

#[test]
fn rule_3_opaque() {
    // 2x1 image -> all opaque.
    let img = GrayImage::from_vec(2, 1, vec![10, 200]).unwrap();
    let out = opaque(&img);
    let expected = vec![
        10, 255, // pixel 0
        200, 255, // pixel 1
    ];
    assert_eq!(out.as_raw(), &expected);
}

#[test]
fn rule_4_keep_success() {
    // 3x1 image, mask #.# -> keep selected, others transparent white.
    let img = GrayImage::from_vec(3, 1, vec![10, 200, 0]).unwrap();
    let mask = Mask::from_bits(3, 1, vec![true, false, true]).unwrap();
    let out = keep(&img, &mask).unwrap();
    let expected = vec![
        10, 255, // kept
        255, 0, // transparent white
        0, 255, // kept
    ];
    assert_eq!(out.as_raw(), &expected);
}

#[test]
fn rule_5_erase_success() {
    // Start with three pixels, mask .## -> erase last two.
    let mut img = GrayAlphaImage::from_vec(
        3,
        1,
        vec![
            10, 255, // pixel 0 (kept)
            20, 255, // pixel 1 (to erase)
            255, 0, // pixel 2 (already transparent)
        ],
    )
    .unwrap();
    let mask = Mask::from_bits(3, 1, vec![false, true, true]).unwrap();
    erase(&mut img, &mask).unwrap();
    let expected = vec![
        10, 255, // unchanged
        255, 0, // erased
        255, 0, // unchanged (already transparent)
    ];
    assert_eq!(img.as_raw(), &expected);
}

#[test]
fn rule_6_flatten() {
    // Pixels with various alphas -> transparent become white.
    let img = GrayAlphaImage::from_vec(
        3,
        1,
        vec![
            10, 255, // opaque
            37, 0, // transparent -> white
            90, 128, // opaque (alpha ignored)
        ],
    )
    .unwrap();
    let out = flatten(&img);
    let expected = vec![10, 255, 90];
    assert_eq!(out.as_raw(), &expected);
}

#[test]
fn rule_7_blank_outside_no_invert() {
    // 3 bytes, mask #.#, invert false -> non-selected become 255.
    let mut gray = vec![5u8, 6, 7];
    let mask = Mask::from_bits(3, 1, vec![true, false, true]).unwrap();
    blank_outside(&mut gray, &mask, false).unwrap();
    let expected = vec![5, 255, 7];
    assert_eq!(gray, expected);
}

#[test]
fn rule_7_blank_outside_invert() {
    // 3 bytes, mask #.#, invert true -> non-selected become 0.
    let mut gray = vec![5u8, 6, 7];
    let mask = Mask::from_bits(3, 1, vec![true, false, true]).unwrap();
    blank_outside(&mut gray, &mask, true).unwrap();
    let expected = vec![5, 0, 7];
    assert_eq!(gray, expected);
}
