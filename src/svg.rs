//! SVG text output.

/// `v` rounded to 2 decimals, without trailing zeros: 1.5 -> "1.5", 2.0 -> "2",
/// -0.001 -> "0", 3.14159 -> "3.14".
pub fn fmt_num(v: f64) -> String {
    let r = (v * 100.0).round() as i64;
    let sign = if r < 0 { "-" } else { "" };
    let abs_r = r.abs();
    let whole = abs_r / 100;
    let frac = abs_r % 100;
    if frac == 0 {
        format!("{}{}", sign, whole)
    } else if frac % 10 == 0 {
        format!("{}{}.{}", sign, whole, frac / 10)
    } else {
        format!("{}{}.{:02}", sign, whole, frac)
    }
}

/// "#rrggbb", lowercase.
pub fn hex(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

/// Path data for closed loops of cubic segments [x0, y0, c1x, c1y, c2x, c2y, x1, y1].
pub fn path_data(loops: &[Vec<[f64; 8]>]) -> String {
    let mut parts = Vec::new();
    for loop_vec in loops {
        if loop_vec.is_empty() {
            continue;
        }
        let first = loop_vec[0];
        let mut s = format!("M {} {}", fmt_num(first[0]), fmt_num(first[1]));
        for seg in loop_vec.iter() {
            s.push_str(&format!(
                " C {} {} {} {} {} {}",
                fmt_num(seg[2]),
                fmt_num(seg[3]),
                fmt_num(seg[4]),
                fmt_num(seg[5]),
                fmt_num(seg[6]),
                fmt_num(seg[7])
            ));
        }
        s.push_str(" Z");
        parts.push(s);
    }
    parts.join(" ")
}

/// A whole SVG document: a background rectangle, then one path per shape.
pub fn document(
    width: u32,
    height: u32,
    background: [u8; 3],
    shapes: &[([u8; 3], String)],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
        width,
        height,
        hex(background)
    ));
    for (color, d) in shapes {
        if d.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "<path fill=\"{}\" fill-rule=\"evenodd\" d=\"{}\"/>\n",
            hex(*color),
            d
        ));
    }
    out.push_str("</svg>\n");
    out
}
