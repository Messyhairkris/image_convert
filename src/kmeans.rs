//! Deterministic K-means colour quantization over 5-bit colour bins.

use std::collections::BTreeMap;

/// The palette (centre order) and one label per pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub colors: Vec<[u8; 3]>,
    pub labels: Vec<u8>,
}

/// One 5-bit colour bin: its pixel count and per-channel sums (rule 1).
#[derive(Debug, Clone, Copy)]
struct Bin {
    count: u64,
    sum_r: u64,
    sum_g: u64,
    sum_b: u64,
}

impl Bin {
    /// The bin's position: the mean of its pixels, in `f64` (rule 1).
    fn position(&self) -> [f64; 3] {
        let n = self.count as f64;
        [
            self.sum_r as f64 / n,
            self.sum_g as f64 / n,
            self.sum_b as f64 / n,
        ]
    }
}

/// The 5-bit bin key of one pixel (rule 1).
fn bin_key(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) >> 3) << 10 | ((g as u32) >> 3) << 5 | ((b as u32) >> 3)
}

/// Squared Euclidean distance between two `f64` colours (rule 4).
fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    d0 * d0 + d1 * d1 + d2 * d2
}

/// Quantize packed RGB bytes (3 per pixel) to at most `k` colours.
pub fn quantize(rgb: &[u8], k: usize, max_iter: usize) -> Palette {
    // rule 1: bin the pixels; the BTreeMap keeps the bins in ascending key order.
    let mut map: BTreeMap<u32, Bin> = BTreeMap::new();
    for px in rgb.as_chunks::<3>().0 {
        let bin = map.entry(bin_key(px[0], px[1], px[2])).or_insert(Bin {
            count: 0,
            sum_r: 0,
            sum_g: 0,
            sum_b: 0,
        });
        bin.count += 1;
        bin.sum_r += px[0] as u64;
        bin.sum_g += px[1] as u64;
        bin.sum_b += px[2] as u64;
    }

    // rule 2: no pixels at all.
    if map.is_empty() {
        return Palette {
            colors: Vec::new(),
            labels: Vec::new(),
        };
    }

    let bins: Vec<Bin> = map.values().copied().collect();
    let positions: Vec<[f64; 3]> = bins.iter().map(Bin::position).collect();
    let n = bins.len();

    // rule 3: clamp k to 1..=64, then to at most the number of bins.
    let k = k.clamp(1, 64).min(n);

    // rule 5: farthest-first initial centres.
    let mut centres: Vec<[f64; 3]> = Vec::with_capacity(k);
    let mut first = 0;
    for i in 1..n {
        if bins[i].count > bins[first].count {
            first = i;
        }
    }
    centres.push(positions[first]);
    while centres.len() < k {
        let mut best = 0;
        let mut best_dist = -1.0_f64;
        for (i, pos) in positions.iter().enumerate() {
            let mut nearest = f64::INFINITY;
            for c in &centres {
                let d = dist2(*pos, *c);
                if d < nearest {
                    nearest = d;
                }
            }
            // ties: the larger count, then the smaller key (i ascends by key).
            let wins =
                nearest > best_dist || (nearest == best_dist && bins[i].count > bins[best].count);
            if wins {
                best = i;
                best_dist = nearest;
            }
        }
        centres.push(positions[best]);
    }

    // rule 6: iterate at most max_iter times.
    let mut assignment = vec![0usize; n];
    let mut previous: Option<Vec<usize>> = None;
    for _ in 0..max_iter {
        // 6a: nearest centre; ties -> the lower centre index.
        for (i, pos) in positions.iter().enumerate() {
            let mut best = 0;
            let mut best_dist = f64::INFINITY;
            for (ci, c) in centres.iter().enumerate() {
                let d = dist2(*pos, *c);
                if d < best_dist {
                    best_dist = d;
                    best = ci;
                }
            }
            assignment[i] = best;
        }
        // 6b: stop when the assignment repeats; the first iteration always continues.
        if previous.as_ref() == Some(&assignment) {
            break;
        }
        previous = Some(assignment.clone());
        // 6c: move each centre to the pixel-weighted mean of its bins.
        let mut sums = vec![[0u64; 4]; centres.len()];
        for (i, bin) in bins.iter().enumerate() {
            let ci = assignment[i];
            sums[ci][0] += bin.sum_r;
            sums[ci][1] += bin.sum_g;
            sums[ci][2] += bin.sum_b;
            sums[ci][3] += bin.count;
        }
        for ci in 0..centres.len() {
            if sums[ci][3] > 0 {
                let c = sums[ci][3] as f64;
                centres[ci] = [
                    sums[ci][0] as f64 / c,
                    sums[ci][1] as f64 / c,
                    sums[ci][2] as f64 / c,
                ];
            }
        }
    }

    // rule 7: round each centre channel, half away from zero, clamped to 0..=255.
    let colors: Vec<[u8; 3]> = centres
        .iter()
        .map(|c| {
            [
                c[0].round().clamp(0.0, 255.0) as u8,
                c[1].round().clamp(0.0, 255.0) as u8,
                c[2].round().clamp(0.0, 255.0) as u8,
            ]
        })
        .collect();

    // rule 8: label every pixel by the nearest rounded palette colour, in integers.
    let mut labels = Vec::with_capacity(rgb.len() / 3);
    for px in rgb.as_chunks::<3>().0 {
        let p = [px[0] as i32, px[1] as i32, px[2] as i32];
        let mut best = 0;
        let mut best_dist = i32::MAX;
        for (ci, c) in colors.iter().enumerate() {
            let d0 = p[0] - c[0] as i32;
            let d1 = p[1] - c[1] as i32;
            let d2 = p[2] - c[2] as i32;
            let d = d0 * d0 + d1 * d1 + d2 * d2;
            if d < best_dist {
                best_dist = d;
                best = ci;
            }
        }
        labels.push(best as u8);
    }

    Palette { colors, labels }
}
