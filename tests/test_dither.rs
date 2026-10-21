//! Integration tests for `photo_tools::dither::floyd_steinberg`, written from the
//! contract (Floyd-Steinberg error diffusion to one bit).

use photo_tools::dither::floyd_steinberg;

/// Rule 3: a pixel below the 128 threshold becomes black (0).
#[test]
fn rule_3_below_128_is_black() {
    let gray = [100u8];
    assert_eq!(floyd_steinberg(&gray, 1, 1), vec![0]);
}

/// Rule 3: a pixel exactly at the 128 threshold becomes white (255), since new = 255
/// when old >= 128.
#[test]
fn rule_3_at_128_is_white() {
    let gray = [128u8];
    assert_eq!(floyd_steinberg(&gray, 1, 1), vec![255]);
}

/// Rule 4: the right neighbour receives err * 7 / 16. 100 -> 0 leaves err 100, so the
/// second pixel becomes 100 + 700/16 = 143, which is >= 128 -> 255.
#[test]
fn rule_4_right_share_spreads_error() {
    let gray = [100u8, 100];
    assert_eq!(floyd_steinberg(&gray, 2, 1), vec![0, 255]);
}

/// Rule 4: a negative share truncates toward zero, not toward negative infinity.
/// 200 -> 255 leaves err -55; the right neighbour gets -385/16 = -24 (not -25), so
/// 152 - 24 = 128, which is >= 128 -> 255.
#[test]
fn rule_4_negative_share_truncates_toward_zero() {
    let gray = [200u8, 152];
    assert_eq!(floyd_steinberg(&gray, 2, 1), vec![255, 255]);
}

/// Rule 5: the buffer is never clamped. 100 -> 0 leaves err 100; the middle pixel
/// becomes 255 + 43 = 298 (kept, not clamped to 255), so its err is 43 and the last
/// pixel becomes 115 + 18 = 133 -> 255.
#[test]
fn rule_5_buffer_is_never_clamped() {
    let gray = [100u8, 255, 115];
    assert_eq!(floyd_steinberg(&gray, 3, 1), vec![0, 255, 255]);
}

/// Rule 4: error diffuses to right, down and down-right in a 2x2 block of 64s.
/// (0,0) 64 -> 0, err 64: right 28 -> 92, down 20 -> 84, down-right 4 -> 68.
/// (1,0) 92 -> 0, err 92: down-left 17 -> 101, down 28 -> 96.
/// (0,1) 101 -> 0, err 101: right 44 -> 140. (1,1) 140 -> 255.
#[test]
fn rule_4_2x2_error_diffusion() {
    let gray = [64u8, 64, 64, 64];
    assert_eq!(floyd_steinberg(&gray, 2, 2), vec![0, 0, 0, 255]);
}

/// Rule 2: every row runs left to right (no serpentine). Row 1 must be visited
/// (0,1) before (1,1); a serpentine order would leave (1,1) at 115 -> 0.
#[test]
fn rule_2_rows_run_left_to_right() {
    let gray = [128u8, 128, 100, 100];
    assert_eq!(floyd_steinberg(&gray, 2, 2), vec![255, 0, 0, 255]);
}

/// Rule 4: full diffusion across a 3x3 block of 100s.
#[test]
fn rule_4_3x3_full_diffusion() {
    let gray = [100u8; 9];
    assert_eq!(
        floyd_steinberg(&gray, 3, 3),
        vec![0, 255, 0, 0, 255, 0, 0, 0, 255]
    );
}
