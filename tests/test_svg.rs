// 3.14159 is a rounding input for fmt_num, not an approximation of PI.
#![allow(clippy::approx_constant)]

//! Integration tests for `photo_tools::svg` (contract: SVG text output, byte-exact).

use photo_tools::svg::{document, fmt_num, hex, path_data};

// rule 1: r = (v * 100.0).round() as i64 (half away from zero); sign "-" only
// if r < 0; frac printed with no digits, one digit, or two digits.

#[test]
fn rule_1_fmt_num_worked_example_rows() {
    let rows: &[(f64, &str)] = &[
        (0.0, "0"),
        (0.07, "0.07"),
        (1.5, "1.5"),
        (-0.5, "-0.5"),
        (2.0, "2"),
        (1234.5678, "1234.57"),
        (-0.001, "0"),
        (0.005, "0.01"),
        (-0.0, "0"),
        (0.004999, "0"),
        (3.14159, "3.14"),
        (2.0 / 3.0, "0.67"),
        (-1.25, "-1.25"),
        (100.0, "100"),
        (10.1, "10.1"),
        (-7.999, "-8"),
    ];
    for &(v, want) in rows {
        assert_eq!(fmt_num(v), want, "fmt_num({v})");
    }
}

#[test]
fn rule_1_fmt_num_rounds_half_away_from_zero() {
    // 0.125 * 100 = 12.5 exactly; half away from zero -> 13 -> "0.13".
    assert_eq!(fmt_num(0.125), "0.13");
    // -12.5 -> -13, and r < 0 keeps the sign: "-0.13", not "-0" or "0".
    assert_eq!(fmt_num(-0.125), "-0.13");
    // -0.5 -> -1 -> "-0.01": a negative tie must not collapse to "0" or "-0".
    assert_eq!(fmt_num(-0.005), "-0.01");
}

// rule 2: "#" then each channel as two lowercase hex digits.

#[test]
fn rule_2_hex_lowercase_two_hex_digits_per_channel() {
    assert_eq!(hex([255, 0, 16]), "#ff0010");
    assert_eq!(hex([171, 205, 239]), "#abcdef");
    assert_eq!(hex([0, 0, 0]), "#000000");
    // Values under 16 must be zero-padded, not one digit.
    assert_eq!(hex([10, 1, 254]), "#0a01fe");
}

// rule 3: "M x0 y0" from the FIRST segment's start, one " C ..." per segment,
// " Z", loops joined with one space, empty loops skipped.

#[test]
fn rule_3_path_data_single_loop_move_then_cubics_then_close() {
    let loops = [vec![[0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0]]];
    assert_eq!(path_data(&loops), "M 0 0 C 1 0 2 0 3 0 Z");
}

#[test]
fn rule_3_path_data_two_segments_format_each_number() {
    // 3.333 -> "3.33" and 2.666 -> "2.67" must go through fmt_num, and the
    // move must use the first segment's start (1, 2), not the last endpoint.
    let loops = [vec![
        [1.0, 2.0, 1.5, 2.0, 2.5, 2.5, 3.0, 3.0],
        [3.0, 3.0, 2.0, 3.333, 1.0, 2.666, 1.0, 2.0],
    ]];
    assert_eq!(
        path_data(&loops),
        "M 1 2 C 1.5 2 2.5 2.5 3 3 C 2 3.33 1 2.67 1 2 Z"
    );
}

#[test]
fn rule_3_path_data_skips_empty_loop_and_joins_with_one_space() {
    let a = vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0]];
    let b = vec![[5.0, 5.0, 6.0, 5.0, 6.0, 6.0, 5.0, 6.0]];
    let loops = [a, vec![], b];
    assert_eq!(
        path_data(&loops),
        "M 0 0 C 0 1 1 1 1 0 Z M 5 5 C 6 5 6 6 5 6 Z"
    );
}

#[test]
fn rule_3_path_data_no_loops_or_only_empty_loops_is_empty_string() {
    assert_eq!(path_data(&[]), "");
    assert_eq!(path_data(&[vec![], vec![]]), "");
}

// rule 4: header, background rect, one path line per shape in order (a shape
// with empty d gets no line), "</svg>", one final newline, nothing between.

#[test]
fn rule_4_document_one_line_per_shape_in_order_skips_empty_d() {
    let shapes = [
        ([255, 0, 0], String::from("M 0 0 C 1 0 1 0 1 1 Z")),
        ([0, 0, 255], String::new()),
        ([0, 128, 0], String::from("M 1 1 C 2 1 2 1 2 2 Z")),
    ];
    let want = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"3\" height=\"2\" viewBox=\"0 0 3 2\">\n",
        "<rect width=\"3\" height=\"2\" fill=\"#ffffff\"/>\n",
        "<path fill=\"#ff0000\" fill-rule=\"evenodd\" d=\"M 0 0 C 1 0 1 0 1 1 Z\"/>\n",
        "<path fill=\"#008000\" fill-rule=\"evenodd\" d=\"M 1 1 C 2 1 2 1 2 2 Z\"/>\n",
        "</svg>\n"
    );
    assert_eq!(document(3, 2, [255, 255, 255], &shapes), want);
}

#[test]
fn rule_4_document_all_shapes_empty_d_means_no_path_lines() {
    let shapes = [([9, 8, 7], String::new())];
    let want = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"4\" viewBox=\"0 0 2 4\">\n",
        "<rect width=\"2\" height=\"4\" fill=\"#090807\"/>\n",
        "</svg>\n"
    );
    assert_eq!(document(2, 4, [9, 8, 7], &shapes), want);
}

#[test]
fn rule_4_document_no_shapes_just_header_rect_and_close() {
    let want = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">\n",
        "<rect width=\"1\" height=\"1\" fill=\"#010203\"/>\n",
        "</svg>\n"
    );
    assert_eq!(document(1, 1, [1, 2, 3], &[]), want);
}
