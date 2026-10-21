//! Integration tests for `photo_tools::ascii` module.

use photo_tools::ascii::{grid, render};

#[test]
fn rule_1_grid_zero_dimensions() {
    // Rule 1: zero width or height returns all zeros.
    assert_eq!(grid(0, 5, 3), (0, 0, 0, 0));
    assert_eq!(grid(5, 0, 3), (0, 0, 0, 0));
}

#[test]
fn rule_2_grid_columns_clamp_and_cell_sizes() {
    // Worked examples for rule 2.
    assert_eq!(grid(4, 2, 2), (2, 2, 2, 1));
    assert_eq!(grid(10, 10, 3), (3, 6, 3, 1));
    assert_eq!(grid(4, 8, 0), (4, 8, 1, 1));
    assert_eq!(grid(3, 6, 10), (1, 2, 3, 3));
    assert_eq!(grid(100, 30, 25), (4, 8, 25, 3));
}

#[test]
fn rule_3_render_basic_black() {
    // All pixels black, no invert -> darkest character '@'.
    let gray = [0u8; 8];
    assert_eq!(render(&gray, None, 4, 2, 2, false), "@@\n");
}

#[test]
fn rule_3_render_basic_white() {
    // All pixels white, no invert -> lightest character ' ' (space).
    let gray = [255u8; 8];
    assert_eq!(render(&gray, None, 4, 2, 2, false), "  \n");
}

#[test]
fn rule_4_render_invert_switches_palette() {
    // Same black input, invert flag flips to light space.
    let gray = [0u8; 8];
    assert_eq!(render(&gray, None, 4, 2, 2, true), "  \n");
}

#[test]
fn rule_3_and_4_render_mixed_values() {
    // Mixed gray values, checks mean truncation and ramp index.
    let gray = [0, 255, 0, 255, 128, 200, 128, 200];
    assert_eq!(render(&gray, None, 2, 4, 2, false), "@ \n=.\n");
}

#[test]
fn rule_3_ignored_pixels_past_last_cell() {
    // 5x4 image, column 4 white but cells cover only first 4 columns.
    // Expected output ignores the white column.
    let gray: Vec<u8> = (0..20).map(|i| if i % 5 == 4 { 255 } else { 0 }).collect();
    assert_eq!(render(&gray, None, 5, 4, 2, false), "@@\n");
}

#[test]
fn rule_5_ansi_layout_two_coloured_cells() {
    // Two coloured cells, 2x2 image, red and blue, columns = 2.
    let gray = [76u8, 29, 76, 29];
    let rgb = [
        255, 0, 0, // red
        0, 0, 255, // blue
        255, 0, 0, // red
        0, 0, 255, // blue
    ];
    let expected = "\x1b[38;2;255;0;0m*\x1b[38;2;0;0;255m#\x1b[0m\n";
    assert_eq!(render(&gray, Some(&rgb), 2, 2, 2, false), expected);
}

#[test]
fn rule_6_render_empty_image_returns_empty_string() {
    // No pixels -> empty output.
    assert_eq!(render(&[], None, 0, 0, 10, false), "");
}
