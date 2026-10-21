//! Integration tests for `photo_tools::contour`, written from the contract alone.

use photo_tools::contour::{is_junction, trace, Loop};

/// Builds the expected `Loop` for one worked-example row.
fn lp(label: u8, points: &[(u32, u32)], junction: &[bool]) -> Loop {
    Loop {
        label,
        points: points.to_vec(),
        junction: junction.to_vec(),
    }
}

#[test]
fn rule_1_label_is_carried_through() {
    // trace(&[3], 1, 1): one pixel, one loop, label 3.
    assert_eq!(
        trace(&[3], 1, 1),
        vec![lp(
            3,
            &[(0, 0), (1, 0), (1, 1), (0, 1)],
            &[false, false, false, false]
        )]
    );
}

#[test]
fn rule_2_straight_through_points_are_dropped() {
    // trace(&[0, 0], 2, 1): (1,0) and (1,1) are straight and not junctions.
    assert_eq!(
        trace(&[0, 0], 2, 1),
        vec![lp(
            0,
            &[(0, 0), (2, 0), (2, 1), (0, 1)],
            &[false, false, false, false]
        )]
    );
}

#[test]
fn rule_2_junction_keeps_a_straight_point() {
    // trace(&[0, 0, 1, 2], 2, 2): (1,1) is straight for label 0, but three labels meet.
    assert_eq!(
        trace(&[0, 0, 1, 2], 2, 2),
        vec![
            lp(
                0,
                &[(0, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
                &[false, false, true, true, true]
            ),
            lp(
                1,
                &[(0, 1), (1, 1), (1, 2), (0, 2)],
                &[true, true, true, false]
            ),
            lp(
                2,
                &[(1, 1), (2, 1), (2, 2), (1, 2)],
                &[true, true, false, true]
            ),
        ]
    );
}

#[test]
fn rule_3_loop_starts_at_its_smallest_point() {
    // trace(&[0, 0, 0, 0, 1, 0, 0, 0, 0], 3, 3): the outer loop starts at (0,0) and the
    // hole loop at (1,1), each its own smallest point by (y, x).
    assert_eq!(
        trace(&[0, 0, 0, 0, 1, 0, 0, 0, 0], 3, 3),
        vec![
            lp(
                0,
                &[(0, 0), (3, 0), (3, 3), (0, 3)],
                &[false, false, false, false]
            ),
            lp(
                0,
                &[(1, 1), (1, 2), (2, 2), (2, 1)],
                &[false, false, false, false]
            ),
            lp(
                1,
                &[(1, 1), (2, 1), (2, 2), (1, 2)],
                &[false, false, false, false]
            ),
        ]
    );
}

#[test]
fn rule_4_loops_sorted_by_label_then_first_point() {
    // trace(&[0, 1, 1, 0], 2, 2): label 0 first points (0,0) then (1,1); label 1 first
    // points (1,0) then (0,1) - y before x.
    assert_eq!(
        trace(&[0, 1, 1, 0], 2, 2),
        vec![
            lp(
                0,
                &[(0, 0), (1, 0), (1, 1), (0, 1)],
                &[false, true, true, true]
            ),
            lp(
                0,
                &[(1, 1), (2, 1), (2, 2), (1, 2)],
                &[true, true, false, true]
            ),
            lp(
                1,
                &[(1, 0), (2, 0), (2, 1), (1, 1)],
                &[true, false, true, true]
            ),
            lp(
                1,
                &[(0, 1), (1, 1), (1, 2), (0, 2)],
                &[true, true, true, false]
            ),
        ]
    );
}

#[test]
fn rule_5_empty_image_gives_no_loops() {
    assert_eq!(trace(&[], 0, 0), Vec::<Loop>::new());
}

#[test]
fn worked_example_two_labels_side_by_side() {
    // trace(&[0, 1], 2, 1): (1,0) is a junction (OUTSIDE, OUTSIDE, 0, 1).
    assert_eq!(
        trace(&[0, 1], 2, 1),
        vec![
            lp(
                0,
                &[(0, 0), (1, 0), (1, 1), (0, 1)],
                &[false, true, true, false]
            ),
            lp(
                1,
                &[(1, 0), (2, 0), (2, 1), (1, 1)],
                &[true, false, false, true]
            ),
        ]
    );
}

#[test]
fn worked_example_l_shape() {
    // trace(&[1, 1, 1, 0], 2, 2): (1,1) is a corner but not a junction.
    assert_eq!(
        trace(&[1, 1, 1, 0], 2, 2),
        vec![
            lp(
                0,
                &[(1, 1), (2, 1), (2, 2), (1, 2)],
                &[false, true, false, true]
            ),
            lp(
                1,
                &[(0, 0), (2, 0), (2, 1), (1, 1), (1, 2), (0, 2)],
                &[false, false, true, false, true, false],
            ),
        ]
    );
}

#[test]
fn reexport_is_junction_on_a_checkerboard() {
    // [0, 1, 1, 0] 2x2: (0,0) sees OUTSIDE and 0; (1,0) and (1,1) see three values.
    let labels = [0u8, 1, 1, 0];
    assert!(!is_junction(&labels, 2, 2, 0, 0));
    assert!(is_junction(&labels, 2, 2, 1, 0));
    assert!(is_junction(&labels, 2, 2, 1, 1));
}

#[test]
fn reexport_is_junction_two_labels_are_not_a_checkerboard() {
    // [0, 1, 0, 1] 2x2: (1,1) sees only 0 and 1.
    let labels = [0u8, 1, 0, 1];
    assert!(!is_junction(&labels, 2, 2, 1, 1));
}

#[test]
fn reexport_is_junction_single_pixel() {
    // [5] 1x1: (1,1) sees OUTSIDE and 5.
    assert!(!is_junction(&[5], 1, 1, 1, 1));
}
