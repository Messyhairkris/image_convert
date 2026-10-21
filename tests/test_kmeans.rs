//! Integration tests for the `kmeans` module of `photo_tools`.

use photo_tools::kmeans::{quantize, Palette};

/// Pack (r, g, b) tuples into the flat byte slice `quantize` expects.
fn pack_rgb(pixels: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut v = Vec::with_capacity(pixels.len() * 3);
    for (r, g, b) in pixels {
        v.push(*r);
        v.push(*g);
        v.push(*b);
    }
    v
}

#[test]
fn rule_2_no_pixels_returns_empty() {
    // Rule 2: no pixels -> empty palette and labels.
    let out = quantize(&[], 4, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![],
            labels: vec![]
        }
    );
}

#[test]
fn rule_5_two_clusters_stable() {
    // Worked row: (10,10,10) x3, (200,50,50) x2, order a a b a b, k=2.
    let pixels = [
        (10, 10, 10),
        (10, 10, 10),
        (200, 50, 50),
        (10, 10, 10),
        (200, 50, 50),
    ];
    let out = quantize(&pack_rgb(&pixels), 2, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[10, 10, 10], [200, 50, 50]],
            labels: vec![0, 0, 1, 0, 1],
        }
    );
}

#[test]
fn rule_3_k_clamped_to_bin_count() {
    // Worked row: (9,9,9) x6, k=8. One bin, so k clamps to 1 (rule 3).
    let pixels = [(9, 9, 9); 6];
    let out = quantize(&pack_rgb(&pixels), 8, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[9, 9, 9]],
            labels: vec![0; 6],
        }
    );
}

#[test]
fn rule_3_k_zero_clamped_to_one() {
    // Worked row: (10,10,10) x3, (200,50,50) x2, k=0 -> clamps to 1 (rule 3).
    // Single centre = pixel-weighted mean = (430/5, 130/5, 130/5) = (86, 26, 26).
    let pixels = [
        (10, 10, 10),
        (10, 10, 10),
        (10, 10, 10),
        (200, 50, 50),
        (200, 50, 50),
    ];
    let out = quantize(&pack_rgb(&pixels), 0, 5);
    assert_eq!(
        out,
        Palette {
            colors: vec![[86, 26, 26]],
            labels: vec![0; 5],
        }
    );
}

#[test]
fn rule_6_max_iter_zero_keeps_initial_centres() {
    // Worked row: (8,8,8), (15,15,15), k=1, max_iter=0.
    // Same bin, position 11.5; no iterations (rule 6); 11.5 rounds to 12 (rule 7).
    let pixels = [(8, 8, 8), (15, 15, 15)];
    let out = quantize(&pack_rgb(&pixels), 1, 0);
    assert_eq!(
        out,
        Palette {
            colors: vec![[12, 12, 12]],
            labels: vec![0, 0],
        }
    );
}

#[test]
fn rule_6_assignment_and_update() {
    // Worked row: (0,0,0), (40,0,0), (48,0,0), (200,0,0), k=2.
    // Centre 0 absorbs 0/40/48 -> 88/3 = 29.33 -> 29 (rule 6c, rule 7).
    let pixels = [(0, 0, 0), (40, 0, 0), (48, 0, 0), (200, 0, 0)];
    let out = quantize(&pack_rgb(&pixels), 2, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[29, 0, 0], [200, 0, 0]],
            labels: vec![0, 0, 0, 1],
        }
    );
}

#[test]
fn rule_5_third_centre_farthest() {
    // Worked row: same four pixels, k=3.
    // Centre 2 = 48 (farthest from nearest centre); 40 and 48 join it -> 44.
    let pixels = [(0, 0, 0), (40, 0, 0), (48, 0, 0), (200, 0, 0)];
    let out = quantize(&pack_rgb(&pixels), 3, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[0, 0, 0], [200, 0, 0], [44, 0, 0]],
            labels: vec![0, 2, 2, 1],
        }
    );
}

#[test]
fn rule_5_tie_smaller_key() {
    // Worked row: g g g (64,128,128) (192,128,128), g=(128,128,128), k=2.
    // (64,..) and (192,..) both 4096 from centre 0, count 1 each -> smaller key
    // (64,..) is centre 1 (rule 5). Centre 0 becomes (3*128+192)/4 = 144.
    let pixels = [
        (128, 128, 128),
        (128, 128, 128),
        (128, 128, 128),
        (64, 128, 128),
        (192, 128, 128),
    ];
    let out = quantize(&pack_rgb(&pixels), 2, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[144, 128, 128], [64, 128, 128]],
            labels: vec![0, 0, 0, 1, 0],
        }
    );
}

#[test]
fn rule_6_tie_lower_index() {
    // Worked row: (0,0,0) x2, (100,0,0), (200,0,0) x2, k=2.
    // (100,..) is 10000 from both centres -> lower index, centre 0 (rule 6a).
    // Centre 0 becomes (2*0+100)/3 = 33.33 -> 33.
    let pixels = [(0, 0, 0), (0, 0, 0), (100, 0, 0), (200, 0, 0), (200, 0, 0)];
    let out = quantize(&pack_rgb(&pixels), 2, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[33, 0, 0], [200, 0, 0]],
            labels: vec![0, 0, 0, 1, 1],
        }
    );
}

#[test]
fn rule_5_tie_larger_count() {
    // Walk-through: g g g (64,128,128) (192,128,128) (192,128,128), k=2.
    // (64,..) and (192,..) both 4096 from centre 0; (192,..) has count 2 -> wins
    // the tie by count (rule 5). Centre 0 becomes (3*128+64)/4 = 112.
    let pixels = [
        (128, 128, 128),
        (128, 128, 128),
        (128, 128, 128),
        (64, 128, 128),
        (192, 128, 128),
        (192, 128, 128),
    ];
    let out = quantize(&pack_rgb(&pixels), 2, 10);
    assert_eq!(
        out,
        Palette {
            colors: vec![[112, 128, 128], [192, 128, 128]],
            labels: vec![0, 0, 0, 0, 1, 1],
        }
    );
}

#[test]
fn rule_8_label_tie_lower_index() {
    // (0,0,0) x2, (16,0,0) x2, (8,0,0) x1, k=2, max_iter=0.
    // Palette is [[0,0,0], [16,0,0]]; (8,0,0) is 64 from both -> lower index 0
    // (rule 8).
    let pixels = [(0, 0, 0), (0, 0, 0), (16, 0, 0), (16, 0, 0), (8, 0, 0)];
    let out = quantize(&pack_rgb(&pixels), 2, 0);
    assert_eq!(
        out,
        Palette {
            colors: vec![[0, 0, 0], [16, 0, 0]],
            labels: vec![0, 0, 1, 1, 0],
        }
    );
}
