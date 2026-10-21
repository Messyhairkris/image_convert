//! Integration tests for `photo_tools::hatch` (cross-hatching).

use photo_tools::hatch::hatch;

/// A `width * height` image where every pixel has value `v`.
fn uniform(v: u8, width: usize, height: usize) -> Vec<u8> {
    vec![v; width * height]
}

#[test]
fn rule_1_spacing_below_two_becomes_two() {
    // v = 100 turns on layers 1 and 2 only; s = 2 gives a checkerboard.
    let gray = uniform(100, 4, 4);
    let want = vec![
        0, 255, 0, 255, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0,
    ];
    assert_eq!(hatch(&gray, 4, 4, 2), want);
    assert_eq!(hatch(&gray, 4, 4, 0), want);
    assert_eq!(hatch(&gray, 4, 4, 1), want);
}

#[test]
fn rule_2_layer_1_threshold_is_strictly_below_192() {
    // 191 < 192 keeps layer 1; 192 does not, so the image is all white.
    let want_191 = vec![0, 255, 255, 0, 255, 255, 0, 255, 255, 0, 255, 255];
    assert_eq!(hatch(&uniform(191, 4, 3), 4, 3, 3), want_191);
    assert_eq!(hatch(&uniform(192, 4, 3), 4, 3, 3), vec![255; 12]);
}

#[test]
fn rule_2_layer_2_threshold_is_strictly_below_128() {
    // 127 adds layer 2 on top of layer 1; 128 keeps layer 1 only.
    let want_127 = vec![0, 255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255];
    let want_128 = vec![0, 255, 255, 0, 255, 255, 0, 255, 255, 0, 255, 255];
    assert_eq!(hatch(&uniform(127, 4, 3), 4, 3, 3), want_127);
    assert_eq!(hatch(&uniform(128, 4, 3), 4, 3, 3), want_128);
}

#[test]
fn rule_2_layer_3_threshold_is_strictly_below_64() {
    // 63 adds layer 3 (y % s == 0), filling row 0; 64 keeps layers 1 and 2.
    let want_63 = vec![0, 0, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255];
    let want_64 = vec![0, 255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255];
    assert_eq!(hatch(&uniform(63, 4, 3), 4, 3, 3), want_63);
    assert_eq!(hatch(&uniform(64, 4, 3), 4, 3, 3), want_64);
}

#[test]
fn rule_2_layer_4_threshold_is_strictly_below_32() {
    // 31 adds layer 4 (x % s == 0), filling every row; 32 keeps layers 1-3.
    let want_31 = vec![0; 12];
    let want_32 = vec![0, 0, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255];
    assert_eq!(hatch(&uniform(31, 4, 3), 4, 3, 3), want_31);
    assert_eq!(hatch(&uniform(32, 4, 3), 4, 3, 3), want_32);
}

#[test]
fn worked_example_4x3_spacing_3_value_255_is_all_white() {
    assert_eq!(hatch(&uniform(255, 4, 3), 4, 3, 3), vec![255; 12]);
}

#[test]
fn worked_example_5x4_spacing_4_value_0() {
    let want = vec![
        0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 0, 0, 0, 255, 0, 0,
    ];
    assert_eq!(hatch(&uniform(0, 5, 4), 5, 4, 4), want);
}

#[test]
fn worked_example_mixed_dark_left_column() {
    let gray = vec![0, 255, 255, 255, 0, 255, 255, 255];
    let want = vec![0, 255, 255, 255, 0, 255, 255, 255];
    assert_eq!(hatch(&gray, 4, 2, 3), want);
}

#[test]
fn rule_3_each_pixel_decided_by_its_own_value() {
    // Three dark pixels of different values, each isolated by white neighbours.
    let gray = vec![191, 255, 255, 255, 255, 127, 255, 255, 255, 255, 63, 255];
    let want = vec![0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 0, 255];
    assert_eq!(hatch(&gray, 4, 3, 3), want);
}
