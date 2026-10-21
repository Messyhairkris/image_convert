//! Selections as text, one shape per line.

use crate::options::Error;
use crate::select::{Op, Shape};

/// A saved selection: the selection operations in order, and the eraser strokes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Selection {
    pub ops: Vec<(Op, Shape)>,
    pub erased: Vec<Shape>,
}

/// Parse the text. Blank lines and everything after `#` are ignored.
pub fn parse(text: &str) -> Result<Selection, Error> {
    let mut sel = Selection::default();

    for (idx, raw_line) in text.split('\n').enumerate() {
        let line_no = idx + 1;

        // Drop comment.
        let line = if let Some(pos) = raw_line.find('#') {
            &raw_line[..pos]
        } else {
            raw_line
        };

        let mut parts = line.split_whitespace();

        // No tokens -> skip.
        let action = match parts.next() {
            Some(a) => a,
            None => continue,
        };

        // Action validation.
        let (op_opt, is_erase) = match action {
            "replace" => (Some(Op::Replace), false),
            "add" => (Some(Op::Add), false),
            "subtract" => (Some(Op::Subtract), false),
            "erase" => (None, true),
            _ => {
                return Err(Error::InvalidOption {
                    name: "selection",
                    reason: format!(
                        "line {}: unknown action \"{}\" (use replace, add, subtract or erase)",
                        line_no, action
                    ),
                })
            }
        };

        // Shape kind.
        let shape_kind = match parts.next() {
            Some(s) => s,
            None => {
                return Err(Error::InvalidOption {
                    name: "selection",
                    reason: format!(
                        "line {}: unknown shape \"\" (use polygon or stroke)",
                        line_no
                    ),
                })
            }
        };

        // Validate shape kind vs action.
        if is_erase && shape_kind != "stroke" {
            return Err(Error::InvalidOption {
                name: "selection",
                reason: format!("line {}: erase takes a stroke", line_no),
            });
        }
        if !is_erase && shape_kind != "polygon" && shape_kind != "stroke" {
            return Err(Error::InvalidOption {
                name: "selection",
                reason: format!(
                    "line {}: unknown shape \"{}\" (use polygon or stroke)",
                    line_no, shape_kind
                ),
            });
        }

        // Parse according to kind.
        let shape = if shape_kind == "polygon" {
            // All remaining tokens are points.
            let point_strs: Vec<&str> = parts.collect();

            // Parse points first to catch bad point before count error.
            let mut points = Vec::with_capacity(point_strs.len());
            for token in &point_strs {
                match parse_point(token) {
                    Ok(p) => points.push(p),
                    Err(_) => {
                        return Err(Error::InvalidOption {
                            name: "selection",
                            reason: format!(
                                "line {}: bad point \"{}\" (write x,y)",
                                line_no, token
                            ),
                        })
                    }
                }
            }

            // After successful point parsing, enforce minimum count.
            if points.len() < 3 {
                return Err(Error::InvalidOption {
                    name: "selection",
                    reason: format!("line {}: a polygon needs at least 3 points", line_no),
                });
            }

            Shape::Polygon(points)
        } else {
            // stroke: first token is radius.
            let radius_token = match parts.next() {
                Some(r) => r,
                None => {
                    return Err(Error::InvalidOption {
                        name: "selection",
                        reason: format!("line {}: bad radius \"\"", line_no),
                    })
                }
            };
            let radius: f32 = match radius_token.parse::<f32>() {
                Ok(v) if v.is_finite() && v > 0.0 => v,
                _ => {
                    return Err(Error::InvalidOption {
                        name: "selection",
                        reason: format!("line {}: bad radius \"{}\"", line_no, radius_token),
                    })
                }
            };
            let point_strs: Vec<&str> = parts.collect();
            if point_strs.is_empty() {
                return Err(Error::InvalidOption {
                    name: "selection",
                    reason: format!("line {}: a stroke needs at least 1 point", line_no),
                });
            }
            let mut points = Vec::with_capacity(point_strs.len());
            for token in point_strs {
                match parse_point(token) {
                    Ok(p) => points.push(p),
                    Err(_) => {
                        return Err(Error::InvalidOption {
                            name: "selection",
                            reason: format!(
                                "line {}: bad point \"{}\" (write x,y)",
                                line_no, token
                            ),
                        })
                    }
                }
            }
            Shape::Stroke { points, radius }
        };

        if is_erase {
            sel.erased.push(shape);
        } else {
            sel.ops.push((op_opt.unwrap(), shape));
        }
    }

    Ok(sel)
}

// Helper: parse a "x,y" token into [f32;2].
fn parse_point(token: &str) -> Result<[f32; 2], ()> {
    let mut split = token.splitn(2, ',');
    let x_str = split.next().ok_or(())?;
    let y_str = split.next().ok_or(())?;
    let x: f32 = x_str.parse().map_err(|_| ())?;
    let y: f32 = y_str.parse().map_err(|_| ())?;
    if x.is_finite() && y.is_finite() {
        Ok([x, y])
    } else {
        Err(())
    }
}

/// The text form: a header comment, the operations in order, then the eraser strokes.
pub fn write(sel: &Selection) -> String {
    let mut out = String::new();
    out.push_str("# photo_tools selection\n");

    for (op, shape) in &sel.ops {
        let action = match op {
            Op::Replace => "replace",
            Op::Add => "add",
            Op::Subtract => "subtract",
        };
        out.push_str(action);
        out.push(' ');
        out.push_str(&shape_to_string(shape));
        out.push('\n');
    }

    for shape in &sel.erased {
        out.push_str("erase ");
        out.push_str(&shape_to_string(shape));
        out.push('\n');
    }

    out
}

// Helper: convert a Shape to its textual representation (without leading action).
fn shape_to_string(shape: &Shape) -> String {
    match shape {
        Shape::Polygon(points) => {
            let mut s = String::from("polygon");
            for p in points {
                s.push(' ');
                s.push_str(&format_point(p));
            }
            s
        }
        Shape::Stroke { points, radius } => {
            let mut s = String::from("stroke ");
            s.push_str(&format!("{}", radius));
            for p in points {
                s.push(' ');
                s.push_str(&format_point(p));
            }
            s
        }
    }
}

// Helper: format a point as "x,y" using f32 Display.
fn format_point(p: &[f32; 2]) -> String {
    format!("{},{}", p[0], p[1])
}
