//! Text art from cell brightness.

/// Lightest to darkest.
pub const RAMP: &[u8] = b" .:-=+*#%@";

/// (cell_w, cell_h, cols, rows) for an image and a requested column count.
pub fn grid(width: usize, height: usize, columns: usize) -> (usize, usize, usize, usize) {
    // Rule 1: an empty image has no cells.
    if width == 0 || height == 0 {
        return (0, 0, 0, 0);
    }
    // Rule 2: the requested column count is clamped to 1..=width.
    let columns = columns.clamp(1, width);
    let cell_w = width / columns;
    // Cells are twice as tall as wide; a shorter image caps the cell height.
    let cell_h = (cell_w * 2).min(height);
    let cols = width / cell_w;
    let rows = height / cell_h;
    (cell_w, cell_h, cols, rows)
}

/// One line per row of cells, each ending in "\n". With `rgb`, every character is
/// preceded by a 24-bit colour escape and every line ends with a reset.
pub fn render(
    gray: &[u8],
    rgb: Option<&[u8]>,
    width: usize,
    height: usize,
    columns: usize,
    invert: bool,
) -> String {
    let (cell_w, cell_h, cols, rows) = grid(width, height, columns);
    if cols == 0 || rows == 0 {
        return String::new();
    }
    let count = (cell_w * cell_h) as u64;
    let mut out = String::new();
    for r in 0..rows {
        for c in 0..cols {
            let mut sum: u64 = 0;
            let (mut sr, mut sg, mut sb) = (0u64, 0u64, 0u64);
            for dy in 0..cell_h {
                let y = r * cell_h + dy;
                for dx in 0..cell_w {
                    let x = c * cell_w + dx;
                    let i = y * width + x;
                    sum += u64::from(gray[i]);
                    if let Some(rgb) = rgb {
                        // Rule 5: channel means cover the same pixels as the gray mean.
                        let j = i * 3;
                        sr += u64::from(rgb[j]);
                        sg += u64::from(rgb[j + 1]);
                        sb += u64::from(rgb[j + 2]);
                    }
                }
            }
            // Rule 3: truncated mean; rule 4: ramp index, flipped when inverted.
            let mean = (sum / count) as u32;
            let idx = if invert {
                (mean * 9 / 255) as usize
            } else {
                ((255 - mean) * 9 / 255) as usize
            };
            if let Some(_rgb) = rgb {
                let rmean = sr / count;
                let gmean = sg / count;
                let bmean = sb / count;
                out.push_str("\x1b[38;2;");
                out.push_str(&rmean.to_string());
                out.push(';');
                out.push_str(&gmean.to_string());
                out.push(';');
                out.push_str(&bmean.to_string());
                out.push('m');
            }
            out.push(char::from(RAMP[idx]));
        }
        if rgb.is_some() {
            out.push_str("\x1b[0m");
        }
        out.push('\n');
    }
    out
}
