//! Color Dodge blending.

/// Color Dodge of one channel (rules 1-3 of the contract).
pub fn color_dodge(base: u8, blend: u8) -> u8 {
    if base == 0 {
        0
    } else if blend == 255 {
        255
    } else {
        let d = (255 - blend) as u32;
        let result = (base as u32 * 255 + d / 2) / d;
        result.min(255) as u8
    }
}

/// color_dodge pairwise; the result is as long as the shorter input.
pub fn dodge_layers(base: &[u8], blend: &[u8]) -> Vec<u8> {
    base.iter()
        .zip(blend.iter())
        .map(|(&b, &bl)| color_dodge(b, bl))
        .collect()
}
