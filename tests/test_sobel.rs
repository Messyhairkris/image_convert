//! Integration tests for the `sobel` module: gradient magnitude and edge mask.

use photo_tools::sobel::{edge_mask, magnitude};

#[test]
fn rule_1_clamped_border_vertical_step() {
    // Clamping replicates the edge column, so the step 0->255 gives a full
    // 4*255 = 1020 gradient on both columns next to the edge; corners get 0.
    let gray = [0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255];
    let _want = vec![1020, 1020, 0, 0, 1020, 1020, 0, 0, 1020, 1020, 0, 0];
    let mut got = magnitude(&gray, 4, 3);
    // contract row: every row is [0, 1020, 1020, 0]
    let want = [vec![0u16, 1020, 1020, 0],
        vec![0u16, 1020, 1020, 0],
        vec![0u16, 1020, 1020, 0]]
    .concat();
    got = magnitude(&gray, 4, 3);
    assert_eq!(got, want);
}

#[test]
fn rule_2_horizontal_step_uses_ky_kernel() {
    // gx = 0 everywhere, gy = 255 * 2 + 255 * 2 = 1020 even at the borders
    // because clamping keeps the 255 rows in the kernel.
    let gray = [0, 0, 0, 255, 255, 255];
    let got = magnitude(&gray, 3, 2);
    assert_eq!(got, vec![1020; 6]);
}

#[test]
fn rule_3_round_half_away_from_zero_2x2() {
    // Each corner of [0,1;1,2] has gx = 4, gy = 4 (clamped reads), so
    // sqrt(32) = 5.656... rounds to 6.
    let gray = [0, 1, 1, 2];
    let got = magnitude(&gray, 2, 2);
    assert_eq!(got, vec![6, 6, 6, 6]);
}

#[test]
fn rule_3_single_bright_pixel_magnitude() {
    // At (0,0) clamping puts 255 at kernel [2][2]: gx = gy = 255,
    // sqrt(130050) = 360.62... -> 361. At (1,0) it sits at [2][1]:
    // gx = 0, gy = 2 * 255 = 510. Corners behave like (0,0), edges
    // midpoints like (1,0), center sees no slope -> 0.
    let gray = [0, 0, 0, 0, 255, 0, 0, 0, 0];
    let got = magnitude(&gray, 3, 3);
    assert_eq!(got, vec![361, 510, 361, 510, 0, 510, 361, 510, 361]);
}

#[test]
fn rule_3_uniform_image_and_single_pixel_have_zero_magnitude() {
    let flat = [90u8; 12];
    assert_eq!(magnitude(&flat, 4, 3), vec![0u16; 12]);
    let one = [200u8];
    assert_eq!(magnitude(&one, 1, 1), vec![0u16]);
}

#[test]
fn rule_4_threshold_boundary_361_362() {
    // Corner magnitudes are exactly 361: >= 361 makes them black, 362 does not.
    let gray = [0, 0, 0, 0, 255, 0, 0, 0, 0];
    let at_361 = edge_mask(&gray, 3, 3, 361);
    assert_eq!(at_361, vec![0, 0, 0, 0, 255, 0, 0, 0, 0]);
    let at_362 = edge_mask(&gray, 3, 3, 362);
    assert_eq!(at_362, vec![255, 0, 255, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn rule_4_threshold_400_and_511_select_by_magnitude() {
    // Magnitudes 361 < 400 <= 510: only the 510s are black.
    // 511 > 510: nothing is black.
    let gray = [0, 0, 0, 0, 255, 0, 0, 0, 0];
    let at_400 = edge_mask(&gray, 3, 3, 400);
    assert_eq!(at_400, vec![255, 0, 255, 0, 255, 0, 255, 0, 255]);
    let at_511 = edge_mask(&gray, 3, 3, 511);
    assert_eq!(at_511, vec![255u8; 9]);
}

#[test]
fn rule_4_threshold_zero_makes_everything_black() {
    let gray = [0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255];
    let got = edge_mask(&gray, 4, 3, 0);
    assert_eq!(got, vec![0u8; 12]);
}
