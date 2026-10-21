//! Integration tests for `photo_tools::junction::is_junction`.
//!
//! Every expected value is worked out by hand from the contract's rules and its
//! worked-example table. In the comments `O` is OUTSIDE, which counts as one value.

use photo_tools::junction::is_junction;

/// Rule 2: three or more distinct values around the vertex.
#[test]
fn rule_2_three_distinct_labels_is_a_junction() {
    // 0 0 / 1 2 at (1,1): NW=0 NE=0 SW=1 SE=2 -> {0,1,2} -> true.
    assert!(is_junction(&[0, 0, 1, 2], 2, 2, 1, 1));
    // 0 1 / 2 3 at (1,1): NW=0 NE=1 SW=2 SE=3 -> four values -> true.
    assert!(is_junction(&[0, 1, 2, 3], 2, 2, 1, 1));
}

/// Rule 2: OUTSIDE counts as one value, so a border vertex can still be a junction.
#[test]
fn rule_2_outside_counts_as_one_value() {
    // 0 1 (2x1) at (1,0): NW=O NE=O SW=0 SE=1 -> {O,0,1} -> true.
    assert!(is_junction(&[0, 1], 2, 1, 1, 0));
    // 0 1 (2x1) at (1,1): NW=0 NE=1 SW=O SE=O -> {0,1,O} -> true.
    assert!(is_junction(&[0, 1], 2, 1, 1, 1));
    // 0 0 1 / 0 0 1 (3x2) at (2,0): NW=O NE=O SW=0 SE=1 -> {O,0,1} -> true.
    assert!(is_junction(&[0, 0, 1, 0, 0, 1], 3, 2, 2, 0));
    // 0 0 1 / 0 0 1 (3x2) at (2,2): NW=0 NE=1 SW=O SE=O -> {0,1,O} -> true.
    assert!(is_junction(&[0, 0, 1, 0, 0, 1], 3, 2, 2, 2));
}

/// Rule 2 at x == 0: `x - 1` must not underflow, and the left column is OUTSIDE.
#[test]
fn rule_2_left_border_vertex_does_not_underflow() {
    // 0 1 / 2 3 at (0,1): NW=O NE=0 SW=O SE=2 -> {O,0,2} -> true.
    assert!(is_junction(&[0, 1, 2, 3], 2, 2, 0, 1));
    // 0 1 / 2 3 at (0,0): NW=O NE=O SW=O SE=0 -> {O,0} -> false.
    assert!(!is_junction(&[0, 1, 2, 3], 2, 2, 0, 0));
}

/// Rule 2 at y == 0: `y - 1` must not underflow, and the top row is OUTSIDE.
#[test]
fn rule_2_top_border_vertex_does_not_underflow() {
    // 0 1 / 2 3 at (1,0): NW=O NE=O SW=0 SE=1 -> {O,0,1} -> true.
    assert!(is_junction(&[0, 1, 2, 3], 2, 2, 1, 0));
    // 0 0 / 1 2 at (1,0): NW=O NE=O SW=0 SE=0 -> {O,0} -> false.
    assert!(!is_junction(&[0, 0, 1, 2], 2, 2, 1, 0));
}

/// Rule 3: a checkerboard saddle of two labels with all four cells inside.
#[test]
fn rule_3_checkerboard_saddle_is_a_junction() {
    // 0 1 / 1 0 at (1,1): NW=0 NE=1 SW=1 SE=0 -> NW==SE, NE==SW, NW!=NE -> true.
    assert!(is_junction(&[0, 1, 1, 0], 2, 2, 1, 1));
    // 7 3 / 3 7 at (1,1): NW=7 NE=3 SW=3 SE=7 -> NW==SE, NE==SW, NW!=NE -> true.
    assert!(is_junction(&[7, 3, 3, 7], 2, 2, 1, 1));
}

/// Rule 3 needs NW != NE: a two-label pattern that is not a saddle is not a junction.
#[test]
fn rule_3_saddle_needs_two_distinct_labels() {
    // 0 1 / 0 1 at (1,1): NW=0 NE=1 SW=0 SE=1 -> NW != SE -> false.
    assert!(!is_junction(&[0, 1, 0, 1], 2, 2, 1, 1));
    // 4 4 / 4 4 at (1,1): NW=NE=SW=SE=4 -> one value -> false.
    assert!(!is_junction(&[4, 4, 4, 4], 2, 2, 1, 1));
}

/// Rule 4: exactly two distinct values, not a saddle, is not a junction.
#[test]
fn rule_4_two_values_is_not_a_junction() {
    // 1 1 / 1 0 at (1,1): NW=1 NE=1 SW=1 SE=0 -> {1,0}, NW != SE -> false.
    assert!(!is_junction(&[1, 1, 1, 0], 2, 2, 1, 1));
    // 3 3 (2x1) at (1,1): NW=3 NE=3 SW=O SE=O -> {3,O} -> false.
    assert!(!is_junction(&[3, 3], 2, 1, 1, 1));
    // 0 0 1 / 0 0 1 (3x2) at (2,1): NW=0 NE=1 SW=0 SE=1 -> NW != SE -> false.
    assert!(!is_junction(&[0, 0, 1, 0, 0, 1], 3, 2, 2, 1));
}

/// Rule 4: each corner vertex of a single pixel sees one label and three OUTSIDE cells.
#[test]
fn rule_4_single_pixel_corners_are_not_junctions() {
    // 5 (1x1): every vertex holds {5, O} -> two values -> false.
    assert!(!is_junction(&[5], 1, 1, 0, 0));
    assert!(!is_junction(&[5], 1, 1, 1, 0));
    assert!(!is_junction(&[5], 1, 1, 0, 1));
    assert!(!is_junction(&[5], 1, 1, 1, 1));
}

/// Rule 4: the outer corners of a two-label strip see only two values.
#[test]
fn rule_4_outer_corners_of_a_strip_are_not_junctions() {
    // 0 1 (2x1) at (0,0): NW=O NE=O SW=O SE=0 -> {O,0} -> false.
    assert!(!is_junction(&[0, 1], 2, 1, 0, 0));
    // 0 1 (2x1) at (2,1): NW=1 NE=O SW=O SE=O -> {1,O} -> false.
    assert!(!is_junction(&[0, 1], 2, 1, 2, 1));
}
