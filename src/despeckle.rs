//! Merge small regions of a label map into a neighbour.

/// Components are found once, on the input; small ones are merged in raster
/// order of their first pixel, each into the most common neighbouring label.
pub fn despeckle(labels: &[u8], width: usize, height: usize, min_area: usize) -> Vec<u8> {
    // Rule 1: min_area <= 1 leaves the map unchanged.
    if min_area <= 1 {
        return labels.to_vec();
    }
    let len = labels.len();
    let mut current = labels.to_vec();

    // Rule 2: components on the input, 4-connected, numbered in raster order
    // of their first pixel; a raster scan meets that pixel first.
    let unassigned = usize::MAX;
    let mut comp_of = vec![unassigned; len];
    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut buf = [0usize; 4];
    for start in 0..len {
        if comp_of[start] != unassigned {
            continue;
        }
        let id = comps.len();
        let label = labels[start];
        let mut pixels = Vec::new();
        let mut stack = vec![start];
        comp_of[start] = id;
        while let Some(p) = stack.pop() {
            pixels.push(p);
            let n = neighbours(p, width, height, &mut buf);
            for &q in &buf[..n] {
                if comp_of[q] == unassigned && labels[q] == label {
                    comp_of[q] = id;
                    stack.push(q);
                }
            }
        }
        comps.push(pixels);
    }

    // Rule 4 counts one slot per label value; `touched` lists the labels that
    // have a nonzero count so the array resets without a full sweep.
    let mut counts = [0u64; 256];
    let mut touched: Vec<u8> = Vec::new();
    for (c, pixels) in comps.iter().enumerate() {
        // Rule 3: components at or above min_area stay as they are.
        if pixels.len() >= min_area {
            continue;
        }
        // Rule 4: count the CURRENT label of each outside neighbour; a
        // neighbour adjacent to two component pixels counts twice.
        for &p in pixels {
            let n = neighbours(p, width, height, &mut buf);
            for &q in &buf[..n] {
                if comp_of[q] == c {
                    continue;
                }
                let l = current[q] as usize;
                if counts[l] == 0 {
                    touched.push(l as u8);
                }
                counts[l] += 1;
            }
        }
        // Rule 5: no outside neighbour leaves the component unchanged.
        if touched.is_empty() {
            continue;
        }
        // Rule 5: largest count wins; scanning upward makes the smaller label
        // value take ties.
        let mut best = 0usize;
        let mut best_count = 0u64;
        for (l, &count) in counts.iter().enumerate() {
            if count > best_count {
                best_count = count;
                best = l;
            }
        }
        for &p in pixels {
            current[p] = best as u8;
        }
        for &l in &touched {
            counts[l as usize] = 0;
        }
        touched.clear();
    }
    current
}

/// The 4-neighbours of `p` that are inside the image, written into `out`;
/// returns how many there are.
fn neighbours(p: usize, width: usize, height: usize, out: &mut [usize; 4]) -> usize {
    let x = p % width;
    let y = p / width;
    let mut n = 0;
    if x > 0 {
        out[n] = p - 1;
        n += 1;
    }
    if x + 1 < width {
        out[n] = p + 1;
        n += 1;
    }
    if y > 0 {
        out[n] = p - width;
        n += 1;
    }
    if y + 1 < height {
        out[n] = p + width;
        n += 1;
    }
    n
}
