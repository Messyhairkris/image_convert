//! Cross-hatching.

/// Black (0) hatch lines on white (255); darker pixels collect more layers.
pub fn hatch(gray: &[u8], width: usize, height: usize, spacing: usize) -> Vec<u8> {
    // Rule 1: spacing below 2 counts as 2.
    let s = if spacing < 2 { 2 } else { spacing };
    let mut out = vec![255u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let v = gray[i];
            // Rule 2: black if any layer matches; comparisons are strict.
            let black = (v < 192 && (x + y) % s == 0)
                || (v < 128 && x % s == y % s)
                || (v < 64 && y % s == 0)
                || (v < 32 && x % s == 0);
            if black {
                out[i] = 0;
            }
        }
    }
    out
}
