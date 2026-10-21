//! Gray levels from RGB, and inversion.

/// Rec. 601 luma of one pixel, rounded: (299 r + 587 g + 114 b + 500) / 1000.
pub fn luma(r: u8, g: u8, b: u8) -> u8 {
    let r = r as u32;
    let g = g as u32;
    let b = b as u32;
    ((299 * r + 587 * g + 114 * b + 500) / 1000) as u8
}

/// One gray byte per complete RGB triple; a trailing partial triple is ignored.
pub fn rgb_to_gray(rgb: &[u8]) -> Vec<u8> {
    rgb.as_chunks::<3>().0.iter()
        .map(|chunk| luma(chunk[0], chunk[1], chunk[2]))
        .collect()
}

/// 255 - v for every byte.
pub fn invert(gray: &[u8]) -> Vec<u8> {
    gray.iter().map(|&v| 255 - v).collect()
}
