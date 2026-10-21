//! Integration tests for `photo_tools::halftone`.

use photo_tools::halftone::halftone;

#[test]
fn rule_6_white_cell_has_no_black_pixel_even_at_centre() {
    // 3x3, cell 3: one cell, mean 255, so r = 0. The centre pixel sits at distance
    // 0 from the centre, and 0 < 0 is false, so nothing is black.
    let gray = vec![255u8; 9];
    assert_eq!(halftone(&gray, 3, 3, 3), vec![255u8; 9]);

    // 4x4, cell 4: same rule, r = 0.
    let gray = vec![255u8; 16];
    assert_eq!(halftone(&gray, 4, 4, 4), vec![255u8; 16]);
}

#[test]
fn rule_4_full_darkness_reaches_cell_corners() {
    // 4x4, cell 4: mean 0, darkness 1, r^2 = 8. The farthest pixel centre is
    // 1.5^2 + 1.5^2 = 4.5 < 8, so every pixel is black.
    let gray = vec![0u8; 16];
    assert_eq!(halftone(&gray, 4, 4, 4), vec![0u8; 16]);
}

#[test]
fn rule_6_mid_gray_leaves_corners_white() {
    // 4x4, cell 4, mean 128: r^2 = 3.984. Corner centres are 4.5 away (white);
    // edge and inner centres are 2.5 or 0.5 away (black).
    let gray = vec![128u8; 16];
    let want = vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 255];
    assert_eq!(halftone(&gray, 4, 4, 4), want);
}

#[test]
fn rule_5_clipped_cell_keeps_full_cell_centre() {
    // 3x3, cell 4: one clipped cell, but the centre is still (2, 2). Only (0, 0)
    // is 4.5 away (white); every other pixel is within 2.5 (black).
    let gray = vec![128u8; 9];
    let want = vec![255, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(halftone(&gray, 3, 3, 4), want);
}

#[test]
fn rule_1_cell_below_2_is_treated_as_2() {
    // 2x2, all 200: with c = 2 the mean is 200, r^2 = 0.431 < 0.5, so all white.
    // A naive c = 1 would give r^2 = 0.108 and paint every pixel black.
    let gray = vec![200u8; 4];
    let want = vec![255u8; 4];
    assert_eq!(halftone(&gray, 2, 2, 2), want);
    assert_eq!(halftone(&gray, 2, 2, 1), want);
    assert_eq!(halftone(&gray, 2, 2, 0), want);
}

#[test]
fn rule_3_mean_truncates_toward_zero() {
    // 2x2, cell 2: sum 765, count 4. Integer division gives mean 191, so
    // r^2 = 2 * 64/255 = 0.50196 > 0.5 and every pixel is black. A float mean of
    // 191.25 would give r^2 = 0.5 exactly and leave the cell white.
    let gray = vec![191u8, 191, 191, 192];
    assert_eq!(halftone(&gray, 2, 2, 2), vec![0u8; 4]);
}

#[test]
fn rule_2_right_edge_cell_is_clipped() {
    // 5x1, cell 4: cell 0 is columns 0..4 (mean 0, all black); cell 1 is column 4
    // alone (mean 255, r = 0, white).
    let gray = vec![0u8, 0, 0, 0, 255];
    assert_eq!(halftone(&gray, 5, 1, 4), vec![0u8, 0, 0, 0, 255]);
}

#[test]
fn rule_2_cells_are_independent() {
    // 4x2, cell 2: the left 2x2 block is all 0 (mean 0, r^2 = 2, black); the right
    // block is all 255 (r = 0, white).
    let gray = vec![0u8, 0, 255, 255, 0, 0, 255, 255];
    let want = vec![0, 0, 255, 255, 0, 0, 255, 255];
    assert_eq!(halftone(&gray, 4, 2, 2), want);
}
