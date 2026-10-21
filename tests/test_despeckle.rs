//! Integration tests for the despeckle module.

use photo_tools::despeckle::despeckle;

/// Rule 1: min_area of 0 or 1 returns a copy unchanged (first table row).
#[test]
fn rule_1_min_area_at_most_1_returns_copy() {
    let labels = vec![0, 1, 0, 1, 0, 1]; // 0 1 0 / 1 0 1
    assert_eq!(despeckle(&labels, 3, 2, 0), labels);
    assert_eq!(despeckle(&labels, 3, 2, 1), labels);
}

/// Rule 2: a single-pixel island sees its 4-connected neighbour four times.
#[test]
fn rule_2_speck_sees_surrounding_label() {
    // 0 0 0 / 0 1 0 / 0 0 0 -> all 0
    let labels = vec![0, 0, 0, 0, 1, 0, 0, 0, 0];
    assert_eq!(despeckle(&labels, 3, 3, 2), vec![0; 9]);
}

/// Rule 5: tie on counts resolves to the smaller label value.
#[test]
fn rule_5_tie_goes_to_smaller_label() {
    // centre 0 sees 1 twice and 2 twice -> becomes 1
    let labels = vec![1, 1, 1, 2, 0, 1, 2, 2, 2];
    let want = vec![1, 1, 1, 2, 1, 1, 2, 2, 2];
    assert_eq!(despeckle(&labels, 3, 3, 2), want);
}

/// Rules 3-5: one pass over components; rule 4 counts CURRENT labels after
/// earlier merges (worked example `0 5 2`, min_area 2).
#[test]
fn rule_4_one_pass_walk_0_5_2() {
    // [0] -> 5 (now 5 5 2); [5] sees 5 and 2, tie -> 2; [2] stays 2.
    let labels = vec![0, 5, 2];
    assert_eq!(despeckle(&labels, 3, 1, 2), vec![5, 2, 2]);
}

/// Rule 6: areas are never recomputed; a 4-pixel region under min_area 5 is
/// still judged by its rule-2 area after both regions merge to 0.
#[test]
fn rule_6_areas_never_recomputed() {
    // A (four 1s) -> 0, then B (four 2s) -> 0, then centre 0 stays 0.
    let labels = vec![1, 1, 1, 2, 0, 1, 2, 2, 2];
    assert_eq!(despeckle(&labels, 3, 3, 5), vec![0; 9]);
}

/// Rule 2: diagonal pixels do not join; all four pixels are separate
/// 1-pixel components and merge to 3 (worked example `7 3 / 3 7`).
#[test]
fn rule_2_diagonals_do_not_join() {
    // Raster order: (0,0)=7 sees 3 twice -> 3; (1,0)=3 sees 3 twice -> 3;
    // (0,1)=3 sees 3 twice -> 3; (1,1)=7 sees 3 twice -> 3. Result: all 3.
    let labels = vec![7, 3, 3, 7];
    assert_eq!(despeckle(&labels, 2, 2, 2), vec![3, 3, 3, 3]);
}

/// Rule 5: a component touching no other pixel is left unchanged (1x1 case).
#[test]
fn rule_5_isolated_pixel_unchanged() {
    assert_eq!(despeckle(&[7], 1, 1, 5), vec![7]);
}

/// Rule 5: a single component filling the image touches nothing, unchanged.
#[test]
fn rule_5_single_component_unchanged() {
    let labels = vec![3, 3, 3, 3];
    assert_eq!(despeckle(&labels, 2, 2, 10), vec![3, 3, 3, 3]);
}

/// Rule 3: components at or above min_area are skipped entirely.
#[test]
fn rule_3_components_at_min_area_are_skipped() {
    // Top row is one component of area 3 (== min_area), bottom row area 3 too.
    let labels = vec![1, 1, 1, 2, 2, 2];
    assert_eq!(despeckle(&labels, 3, 2, 3), vec![1, 1, 1, 2, 2, 2]);
}
