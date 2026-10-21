//! Integration tests for `photo_tools::shapes_file`: selection text parse and write.

use photo_tools::options::Error;
use photo_tools::select::{Op, Shape};
use photo_tools::shapes_file::{parse, write, Selection};

/// The one error shape the contract allows: name "selection", reason `line N: <text>`.
fn sel_err(line: usize, reason: &str) -> Error {
    Error::InvalidOption {
        name: "selection",
        reason: format!("line {}: {}", line, reason),
    }
}

// ---- parse: the worked-example rows ----

#[test]
fn rule_7_replace_polygon_line_parses_to_ops() {
    // Row 1: a plain replace polygon line.
    let got = parse("replace polygon 1,1 3,1 3,3").unwrap();
    assert_eq!(
        got,
        Selection {
            ops: vec![(
                Op::Replace,
                Shape::Polygon(vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0]])
            )],
            erased: vec![],
        }
    );
}

#[test]
fn rule_1_blank_and_comment_lines_are_skipped() {
    // Row 2: a comment line and a blank line between two good lines; erase -> erased.
    let got = parse("add stroke 4 10,10 20,10\n  # note\n\nerase stroke 2.5 5,5").unwrap();
    assert_eq!(
        got,
        Selection {
            ops: vec![(
                Op::Add,
                Shape::Stroke {
                    points: vec![[10.0, 10.0], [20.0, 10.0]],
                    radius: 4.0,
                }
            )],
            erased: vec![Shape::Stroke {
                points: vec![[5.0, 5.0]],
                radius: 2.5,
            }],
        }
    );
}

#[test]
fn rule_1_text_after_hash_on_a_line_is_dropped() {
    // Row 3: a tail comment must not reach the point list.
    let got = parse("subtract polygon 0,0 4,0 4,4 # tail comment").unwrap();
    assert_eq!(
        got,
        Selection {
            ops: vec![(
                Op::Subtract,
                Shape::Polygon(vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]])
            )],
            erased: vec![],
        }
    );
}

#[test]
fn rule_1_tabs_double_spaces_and_crlf_are_whitespace() {
    // Row 4: \t, double spaces and a trailing \r all split as whitespace.
    let got = parse("replace polygon 1,1\t3,1  3,3\r\n").unwrap();
    assert_eq!(
        got,
        Selection {
            ops: vec![(
                Op::Replace,
                Shape::Polygon(vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0]])
            )],
            erased: vec![],
        }
    );
}

#[test]
fn rule_1_empty_text_and_comment_only_give_empty_selection() {
    // Row 5: no tokens anywhere means an empty selection, not an error.
    assert_eq!(parse(""), Ok(Selection::default()));
    assert_eq!(parse("# only a comment"), Ok(Selection::default()));
}

// ---- parse: errors ----

#[test]
fn rule_1_error_line_numbers_count_blank_and_comment_lines() {
    // Row 6: the blank line is line 1, so the bad line is line 2.
    assert_eq!(
        parse("\nmove polygon 1,1 2,2 3,3"),
        Err(sel_err(
            2,
            "unknown action \"move\" (use replace, add, subtract or erase)"
        ))
    );
    // Comments and blank lines are counted too: the bad line here is line 4.
    assert_eq!(
        parse("# one\n\n# three\nnope polygon 1,1 2,2 3,3"),
        Err(sel_err(
            4,
            "unknown action \"nope\" (use replace, add, subtract or erase)"
        ))
    );
}

#[test]
fn rule_3_missing_shape_reports_an_empty_name() {
    // Row 7: "add" with no shape word.
    assert_eq!(
        parse("add"),
        Err(sel_err(1, "unknown shape \"\" (use polygon or stroke)"))
    );
}

#[test]
fn rule_3_unknown_shape_is_an_error() {
    // Row 8.
    assert_eq!(
        parse("add circle 1,1"),
        Err(sel_err(
            1,
            "unknown shape \"circle\" (use polygon or stroke)"
        ))
    );
}

#[test]
fn rule_3_erase_with_polygon_is_an_error() {
    // Row 9: erase only takes strokes.
    assert_eq!(
        parse("erase polygon 1,1 2,2 3,3"),
        Err(sel_err(1, "erase takes a stroke"))
    );
}

#[test]
fn rule_5_zero_radius_is_a_bad_radius() {
    // Row 10: the radius must be > 0.
    assert_eq!(
        parse("add stroke 0 1,1"),
        Err(sel_err(1, "bad radius \"0\""))
    );
}

#[test]
fn rule_5_missing_radius_is_the_empty_token() {
    // Row 11: an absent radius is the empty string.
    assert_eq!(parse("add stroke"), Err(sel_err(1, "bad radius \"\"")));
}

#[test]
fn rule_5_stroke_needs_at_least_one_point() {
    // Row 12: a good radius with no points.
    assert_eq!(
        parse("add stroke 3"),
        Err(sel_err(1, "a stroke needs at least 1 point"))
    );
}

#[test]
fn rule_4_polygon_needs_at_least_three_points() {
    // Row 13.
    assert_eq!(
        parse("add polygon 1,1 2,2"),
        Err(sel_err(1, "a polygon needs at least 3 points"))
    );
}

#[test]
fn rule_6_point_without_a_comma_is_a_bad_point() {
    // Row 14.
    assert_eq!(
        parse("add polygon 1,1 2;2 3,3"),
        Err(sel_err(1, "bad point \"2;2\" (write x,y)"))
    );
}

#[test]
fn rule_6_non_finite_point_is_a_bad_point() {
    // Row 15: "inf" parses as f32 but is not finite.
    assert_eq!(
        parse("add polygon 1,1 2,inf 3,3"),
        Err(sel_err(1, "bad point \"2,inf\" (write x,y)"))
    );
}

#[test]
fn rule_6_point_splits_at_the_first_comma() {
    // Row 16: "1,1,1" splits into "1" and "1,1"; the half "1,1" is not an f32.
    assert_eq!(
        parse("add polygon 1,1,1 2,2 3,3"),
        Err(sel_err(1, "bad point \"1,1,1\" (write x,y)"))
    );
}

#[test]
fn rule_5_radius_must_be_finite_and_positive() {
    // Rule 5 beyond the worked rows: negative, nan and inf radii are all bad.
    assert_eq!(
        parse("add stroke -2 1,1"),
        Err(sel_err(1, "bad radius \"-2\""))
    );
    assert_eq!(
        parse("add stroke nan 1,1"),
        Err(sel_err(1, "bad radius \"nan\""))
    );
    assert_eq!(
        parse("add stroke inf 1,1"),
        Err(sel_err(1, "bad radius \"inf\""))
    );
}

#[test]
fn parse_stops_at_the_first_error() {
    // Both lines are bad; only line 1 is reported.
    assert_eq!(
        parse("move polygon 1,1 2,2 3,3\nadd circle 1,1"),
        Err(sel_err(
            1,
            "unknown action \"move\" (use replace, add, subtract or erase)"
        ))
    );
}

// ---- write ----

#[test]
fn write_header_then_ops_then_erased_lines() {
    // The write worked example: header, op lines in order, then the erase line.
    let sel = Selection {
        ops: vec![
            (
                Op::Replace,
                Shape::Polygon(vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0]]),
            ),
            (
                Op::Subtract,
                Shape::Stroke {
                    points: vec![[2.0, 2.0]],
                    radius: 1.5,
                },
            ),
        ],
        erased: vec![Shape::Stroke {
            points: vec![[0.25, 7.0], [8.0, 9.0]],
            radius: 3.0,
        }],
    };
    assert_eq!(
        write(&sel),
        "# photo_tools selection\n\
         replace polygon 1,1 3,1 3,3\n\
         subtract stroke 1.5 2,2\n\
         erase stroke 3 0.25,7 8,9\n"
    );
}

#[test]
fn write_of_an_empty_selection_is_just_the_header() {
    // Write rules 1 and 5: the header line, and the last line also ends with \n.
    assert_eq!(write(&Selection::default()), "# photo_tools selection\n");
}

#[test]
fn rule_4_write_numbers_use_f32_display() {
    // Write rule 4: 10.0 -> "10", 2.5 -> "2.5", 0.1 -> "0.1".
    let sel = Selection {
        ops: vec![(
            Op::Add,
            Shape::Stroke {
                points: vec![[10.0, 0.1]],
                radius: 2.5,
            },
        )],
        erased: vec![],
    };
    assert_eq!(
        write(&sel),
        "# photo_tools selection\nadd stroke 2.5 10,0.1\n"
    );
}

#[test]
fn rule_4_write_then_parse_round_trips() {
    // Write rule 4: parse(&write(&s)) == s, including 0.1 and negative coordinates.
    let sel = Selection {
        ops: vec![
            (
                Op::Replace,
                Shape::Polygon(vec![[0.1, 2.5], [10.0, 0.0], [-3.25, 4.0]]),
            ),
            (
                Op::Add,
                Shape::Stroke {
                    points: vec![[-1.5, 2.0], [7.0, 8.0]],
                    radius: 0.1,
                },
            ),
            (
                Op::Subtract,
                Shape::Polygon(vec![[100.0, 50.0], [150.0, 50.0], [150.0, 100.0]]),
            ),
        ],
        erased: vec![Shape::Stroke {
            points: vec![[0.0, -3.0], [12.5, 9.75]],
            radius: 0.2,
        }],
    };
    assert_eq!(parse(&write(&sel)), Ok(sel));
}
