//! Integration tests for `photo_tools::luma` module.

use photo_tools::luma;

#[test]
fn rule_1_luma_extremes() {
    // rule 1: basic extremes
    assert_eq!(luma::luma(0, 0, 0), 0); // 0 -> 0
    assert_eq!(luma::luma(255, 255, 255), 255); // max -> 255
}

#[test]
fn rule_1_luma_primary_colors() {
    // rule 1: primary colors from worked examples
    assert_eq!(luma::luma(255, 0, 0), 76); // red
    assert_eq!(luma::luma(0, 255, 0), 150); // green
    assert_eq!(luma::luma(0, 0, 255), 29); // blue
}

#[test]
fn rule_1_luma_rounding_boundaries() {
    // rule 1: rounding up vs down
    assert_eq!(luma::luma(0, 0, 5), 1); // 570+500=1070 -> 1
    assert_eq!(luma::luma(0, 0, 4), 0); // 456+500=956 -> 0
}

#[test]
fn rule_1_luma_misc() {
    // rule 1: mixed values from worked examples
    assert_eq!(luma::luma(10, 20, 30), 18); // (2990+11740+3420+500)/1000 = 18
}

#[test]
fn rule_2_rgb_to_gray_full_and_partial() {
    // rule 2: full triples and trailing partial ignored
    let input = &[255, 0, 0, 0, 255, 0, 7];
    let expected = vec![76, 150];
    assert_eq!(luma::rgb_to_gray(input), expected);
}

#[test]
fn rule_2_rgb_to_gray_edge_cases() {
    // rule 2: empty input and incomplete triple
    assert_eq!(luma::rgb_to_gray(&[]), Vec::new());
    assert_eq!(luma::rgb_to_gray(&[1, 2]), Vec::new());
}

#[test]
fn rule_3_invert_examples() {
    // rule 3: normal inversion
    let input = &[0, 255, 100, 1];
    let expected = vec![255, 0, 155, 254];
    assert_eq!(luma::invert(input), expected);
}

#[test]
fn rule_3_invert_empty() {
    // rule 3: empty slice yields empty vector
    assert_eq!(luma::invert(&[]), Vec::new());
}
