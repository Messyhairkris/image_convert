//! Integration tests for `photo_tools::select`: the contract's rules and worked examples.

use image::GrayImage;
use photo_tools::select::{build, rasterize, Mask, Op, Shape};

/// `["##..", "...."]` -> the row-major bits the contract's tables are written in.
fn bits(rows: &[&str]) -> Vec<bool> {
    let mut out = Vec::new();
    for row in rows {
        for c in row.chars() {
            out.push(c == '#');
        }
    }
    out
}

fn poly(points: &[[f32; 2]]) -> Shape {
    Shape::Polygon(points.to_vec())
}

fn stroke(points: &[[f32; 2]], radius: f32) -> Shape {
    Shape::Stroke {
        points: points.to_vec(),
        radius,
    }
}

/// Rule 2, one pixel at a time: the slow reference the row sweep must reproduce.
fn reference_polygon(points: &[[f32; 2]], width: u32, height: u32) -> Vec<bool> {
    let mut out = vec![false; (width as usize) * (height as usize)];
    if points.len() < 3
        || points
            .iter()
            .any(|p| !p[0].is_finite() || !p[1].is_finite())
    {
        return out;
    }
    let n = points.len();
    for y in 0..height {
        let py = y as f64 + 0.5;
        for x in 0..width {
            let px = x as f64 + 0.5;
            let mut sum = 0i32;
            for i in 0..n {
                let a = points[i];
                let b = points[(i + 1) % n];
                let (ax, ay) = (a[0] as f64, a[1] as f64);
                let (bx, by) = (b[0] as f64, b[1] as f64);
                let up = ay <= py && by > py;
                let down = by <= py && ay > py;
                if !up && !down {
                    continue;
                }
                let xi = ax + (py - ay) * (bx - ax) / (by - ay);
                if px < xi {
                    sum += if up { 1 } else { -1 };
                }
            }
            out[(y as usize) * (width as usize) + (x as usize)] = sum != 0;
        }
    }
    out
}

#[test]
fn rule_1_polygon_with_fewer_than_three_points_selects_nothing() {
    assert_eq!(rasterize(&poly(&[]), 4, 1), bits(&["...."]));
    assert_eq!(rasterize(&poly(&[[0.0, 0.0]]), 4, 1), bits(&["...."]));
    assert_eq!(
        rasterize(&poly(&[[0.0, 0.0], [4.0, 0.0]]), 4, 1),
        bits(&["...."])
    );
}

#[test]
fn rule_1_non_finite_coordinate_selects_nothing() {
    let nan = poly(&[[0.0, 0.0], [9.0, 0.0], [f32::NAN, 9.0]]);
    assert_eq!(
        rasterize(&nan, 4, 4),
        bits(&["....", "....", "....", "...."])
    );
    let inf = poly(&[[0.0, 0.0], [9.0, 0.0], [f32::INFINITY, 9.0]]);
    assert_eq!(
        rasterize(&inf, 4, 4),
        bits(&["....", "....", "....", "...."])
    );
    let bad = stroke(&[[1.0, 1.0], [f32::NAN, 2.0]], 1.0);
    assert_eq!(
        rasterize(&bad, 4, 4),
        bits(&["....", "....", "....", "...."])
    );
}

#[test]
fn rule_1_stroke_with_no_points_or_bad_radius_selects_nothing() {
    let none = bits(&["...", "...", "..."]);
    assert_eq!(rasterize(&stroke(&[], 1.0), 3, 3), none);
    assert_eq!(rasterize(&stroke(&[[1.0, 1.0]], 0.0), 3, 3), none);
    assert_eq!(rasterize(&stroke(&[[1.0, 1.0]], -2.0), 3, 3), none);
    assert_eq!(rasterize(&stroke(&[[1.0, 1.0]], f32::NAN), 3, 3), none);
    assert_eq!(rasterize(&stroke(&[[1.0, 1.0]], f32::INFINITY), 3, 3), none);
}

#[test]
fn rule_2_polygon_uses_nonzero_winding() {
    let square = poly(&[[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]]);
    assert_eq!(
        rasterize(&square, 4, 4),
        bits(&["....", ".##.", ".##.", "...."])
    );
}

#[test]
fn rule_2_centre_on_left_or_top_edge_is_inside() {
    let square = poly(&[[0.5, 0.5], [2.5, 0.5], [2.5, 2.5], [0.5, 2.5]]);
    assert_eq!(rasterize(&square, 3, 3), bits(&["##.", "##.", "..."]));
}

#[test]
fn rule_2_region_circled_twice_still_counts() {
    let twice = poly(&[
        [0.0, 0.0],
        [4.0, 0.0],
        [4.0, 4.0],
        [0.0, 4.0],
        [0.0, 0.0],
        [4.0, 0.0],
        [4.0, 4.0],
        [0.0, 4.0],
    ]);
    assert_eq!(
        rasterize(&twice, 4, 4),
        bits(&["####", "####", "####", "####"])
    );
}

#[test]
fn rule_2_points_outside_the_image_are_fine() {
    let big = poly(&[[-2.0, -2.0], [6.0, -2.0], [6.0, 6.0], [-2.0, 6.0]]);
    assert_eq!(
        rasterize(&big, 4, 4),
        bits(&["####", "####", "####", "####"])
    );
}

#[test]
fn rule_2_irregular_polygon_matches_the_per_pixel_reference() {
    let star: Vec<[f32; 2]> = vec![
        [2.0, 0.0],
        [3.0, 2.0],
        [5.0, 2.0],
        [3.5, 3.5],
        [4.0, 5.0],
        [2.0, 4.0],
        [0.0, 5.0],
        [0.5, 3.5],
        [-1.0, 2.0],
        [1.0, 2.0],
    ];
    assert_eq!(
        rasterize(&Shape::Polygon(star.clone()), 6, 6),
        reference_polygon(&star, 6, 6)
    );

    // Concave L: the notch from (2,2) to (5,5) must stay unselected.
    let ell: Vec<[f32; 2]> = vec![
        [1.0, 1.0],
        [5.0, 1.0],
        [5.0, 2.0],
        [2.0, 2.0],
        [2.0, 5.0],
        [1.0, 5.0],
    ];
    assert_eq!(
        rasterize(&Shape::Polygon(ell.clone()), 6, 6),
        reference_polygon(&ell, 6, 6)
    );

    // Vertices outside the image are fine.
    let outside: Vec<[f32; 2]> = vec![[-1.0, -1.0], [3.0, 2.0], [5.0, -1.0]];
    assert_eq!(
        rasterize(&Shape::Polygon(outside.clone()), 4, 4),
        reference_polygon(&outside, 4, 4)
    );
}

#[test]
fn rule_2_self_crossing_polygon_matches_the_per_pixel_reference() {
    // Bowtie: the two triangles meet at (2, 2); centres on the left edge are in.
    let bowtie: Vec<[f32; 2]> = vec![[0.5, 0.5], [3.5, 3.5], [3.5, 0.5], [0.5, 3.5]];
    assert_eq!(
        rasterize(&Shape::Polygon(bowtie.clone()), 4, 4),
        bits(&["....", "#.#.", "#.#.", "...."])
    );
    assert_eq!(
        rasterize(&Shape::Polygon(bowtie.clone()), 4, 4),
        reference_polygon(&bowtie, 4, 4)
    );

    let crossed: Vec<[f32; 2]> = vec![
        [0.5, 2.5],
        [4.5, 2.5],
        [2.5, 0.5],
        [2.5, 4.5],
        [0.5, 0.5],
        [4.5, 4.5],
    ];
    assert_eq!(
        rasterize(&Shape::Polygon(crossed.clone()), 5, 5),
        reference_polygon(&crossed, 5, 5)
    );
}

#[test]
fn rule_3_row_sweep_matches_the_per_pixel_reference_on_a_larger_image() {
    // Rule 3 must give exactly rule 2's result, only faster.
    let n = 24usize;
    let mut ring: Vec<[f32; 2]> = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32) * 2.0 * std::f32::consts::PI / (n as f32);
        ring.push([64.0 + 50.0 * t.cos(), 64.0 + 50.0 * t.sin()]);
    }
    assert_eq!(
        rasterize(&Shape::Polygon(ring.clone()), 128, 128),
        reference_polygon(&ring, 128, 128)
    );
}

#[test]
fn rule_4_stroke_single_point_is_a_round_brush() {
    assert_eq!(
        rasterize(&stroke(&[[2.0, 2.0]], 1.0), 4, 4),
        bits(&["....", ".##.", ".##.", "...."])
    );
    assert_eq!(
        rasterize(&stroke(&[[2.0, 2.0]], 1.6), 4, 4),
        bits(&[".##.", "####", "####", ".##."])
    );
}

#[test]
fn rule_4_stroke_distance_exactly_equal_to_radius_counts() {
    assert_eq!(
        rasterize(&stroke(&[[0.5, 0.5]], 1.0), 3, 3),
        bits(&["##.", "#..", "..."])
    );
}

#[test]
fn rule_4_stroke_segment_covers_its_whole_length() {
    assert_eq!(
        rasterize(&stroke(&[[0.0, 0.5], [4.0, 0.5]], 0.5), 4, 2),
        bits(&["####", "...."])
    );
}

#[test]
fn rule_4_stroke_uses_any_segment_of_the_polyline() {
    // Bent stroke: the corner at (0.5, 2.5) joins the two segments.
    assert_eq!(
        rasterize(&stroke(&[[0.5, 0.5], [0.5, 2.5], [2.5, 2.5]], 0.5), 3, 3),
        bits(&["#..", "#..", "###"])
    );
}

#[test]
fn mask_new_sets_every_bit_and_get_is_false_outside_the_image() {
    let full = Mask::new(2, 2, true);
    assert_eq!(full.width(), 2);
    assert_eq!(full.height(), 2);
    assert_eq!(full.bits(), &[true, true, true, true]);
    assert_eq!(full.count(), 4);
    assert!(full.get(1, 1));
    assert!(!full.get(2, 0));
    assert!(!full.get(0, 2));
    assert!(!full.get(9, 9));

    let empty = Mask::new(3, 1, false);
    assert_eq!(empty.bits(), &[false, false, false]);
    assert_eq!(empty.count(), 0);
    assert!(!empty.get(0, 0));
}

#[test]
fn mask_from_bits_rejects_a_wrong_length() {
    assert_eq!(Mask::from_bits(2, 2, vec![true; 3]), None);
    assert_eq!(Mask::from_bits(2, 2, vec![true; 5]), None);
    assert_eq!(Mask::from_bits(0, 0, vec![]), Some(Mask::new(0, 0, false)));

    let m = Mask::from_bits(2, 2, vec![true, false, false, true]).unwrap();
    assert_eq!(m.bits(), &[true, false, false, true]);
    assert_eq!(m.count(), 2);
}

#[test]
fn mask_from_luma_selects_gray_at_least_128() {
    let row = GrayImage::from_raw(3, 1, vec![127u8, 128, 255]).unwrap();
    let mask = Mask::from_luma(&row);
    assert_eq!(mask.width(), 3);
    assert_eq!(mask.height(), 1);
    assert_eq!(mask.bits(), &[false, true, true]);
    assert_eq!(mask.count(), 2);

    // Row-major: the 2 x 2 image's second row follows the first.
    let square = GrayImage::from_raw(2, 2, vec![0u8, 255, 128, 127]).unwrap();
    assert_eq!(Mask::from_luma(&square).bits(), &[false, true, true, false]);
}

#[test]
fn mask_apply_follows_replace_add_subtract() {
    let a = poly(&[[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]]);
    let b = poly(&[[3.0, 0.0], [4.0, 0.0], [4.0, 1.0], [3.0, 1.0]]);
    let c = poly(&[[1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0]]);

    let mut m = Mask::new(4, 1, false);
    m.apply(Op::Add, &a);
    assert_eq!(m.bits(), &[true, true, false, false]);
    m.apply(Op::Subtract, &c);
    assert_eq!(m.bits(), &[true, false, false, false]);
    m.apply(Op::Add, &b);
    assert_eq!(m.bits(), &[true, false, false, true]);
    m.apply(Op::Replace, &c);
    assert_eq!(m.bits(), &[false, true, false, false]);
}

#[test]
fn build_rule_1_no_operations_is_none() {
    assert_eq!(build(4, 1, &[]), None);
    assert_eq!(build(0, 0, &[]), None);
}

#[test]
fn build_rule_2_start_value_depends_on_the_first_operation() {
    let a = poly(&[[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]]);
    // Add with no selection starts empty.
    let add = build(4, 1, &[(Op::Add, a.clone())]).unwrap();
    assert_eq!(add.bits(), &[true, true, false, false]);
    // Ctrl with no selection removes from the whole image.
    let sub = build(4, 1, &[(Op::Subtract, a.clone())]).unwrap();
    assert_eq!(sub.bits(), &[false, false, true, true]);
    // Replace starts empty too.
    let rep = build(4, 1, &[(Op::Replace, a)]).unwrap();
    assert_eq!(rep.bits(), &[true, true, false, false]);
}

#[test]
fn build_rule_2_applies_operations_in_order() {
    let a = poly(&[[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]]);
    let b = poly(&[[3.0, 0.0], [4.0, 0.0], [4.0, 1.0], [3.0, 1.0]]);
    let c = poly(&[[1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0]]);

    let add_b = build(4, 1, &[(Op::Replace, a.clone()), (Op::Add, b.clone())]).unwrap();
    assert_eq!(add_b.bits(), &[true, true, false, true]);

    let sub_c = build(4, 1, &[(Op::Replace, a.clone()), (Op::Subtract, c.clone())]).unwrap();
    assert_eq!(sub_c.bits(), &[true, false, false, false]);

    // Replace drops what came before.
    let replace_c = build(4, 1, &[(Op::Add, b), (Op::Replace, c)]).unwrap();
    assert_eq!(replace_c.bits(), &[false, true, false, false]);
}
