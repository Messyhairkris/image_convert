//! Guo-Hall thinning to one-pixel lines.

/// Thin `fg` (true = line) to lines one pixel wide, keeping every line connected.
pub fn thin(fg: &[bool], width: usize, height: usize) -> Vec<bool> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let mut current = fg.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for sub in 0..2 {
            let src = current.clone();
            let mut marked = vec![false; width * height];
            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;
                    if !src[idx] {
                        continue;
                    }
                    // Neighbour coordinates
                    let x_is = x as isize;
                    let y_is = y as isize;
                    let w_is = width as isize;
                    let h_is = height as isize;
                    // p2 = (x, y-1)
                    let p2 = if y_is > 0 {
                        src[((y_is - 1) * w_is + x_is) as usize]
                    } else {
                        false
                    };
                    // p3 = (x+1, y-1)
                    let p3 = if y_is > 0 && x_is + 1 < w_is {
                        src[((y_is - 1) * w_is + (x_is + 1)) as usize]
                    } else {
                        false
                    };
                    // p4 = (x+1, y)
                    let p4 = if x_is + 1 < w_is {
                        src[(y_is * w_is + (x_is + 1)) as usize]
                    } else {
                        false
                    };
                    // p5 = (x+1, y+1)
                    let p5 = if x_is + 1 < w_is && y_is + 1 < h_is {
                        src[((y_is + 1) * w_is + (x_is + 1)) as usize]
                    } else {
                        false
                    };
                    // p6 = (x, y+1)
                    let p6 = if y_is + 1 < h_is {
                        src[((y_is + 1) * w_is + x_is) as usize]
                    } else {
                        false
                    };
                    // p7 = (x-1, y+1)
                    let p7 = if x_is > 0 && y_is + 1 < h_is {
                        src[((y_is + 1) * w_is + (x_is - 1)) as usize]
                    } else {
                        false
                    };
                    // p8 = (x-1, y)
                    let p8 = if x_is > 0 {
                        src[(y_is * w_is + (x_is - 1)) as usize]
                    } else {
                        false
                    };
                    // p9 = (x-1, y-1)
                    let p9 = if x_is > 0 && y_is > 0 {
                        src[((y_is - 1) * w_is + (x_is - 1)) as usize]
                    } else {
                        false
                    };
                    // Compute C
                    let c = ((!p2 as u8) & ((p3 | p4) as u8))
                        + ((!p4 as u8) & ((p5 | p6) as u8))
                        + ((!p6 as u8) & ((p7 | p8) as u8))
                        + ((!p8 as u8) & ((p9 | p2) as u8));
                    // Compute N1 and N2
                    let n1 = ((p9 | p2) as u8)
                        + ((p3 | p4) as u8)
                        + ((p5 | p6) as u8)
                        + ((p7 | p8) as u8);
                    let n2 = ((p2 | p3) as u8)
                        + ((p4 | p5) as u8)
                        + ((p6 | p7) as u8)
                        + ((p8 | p9) as u8);
                    let n = n1.min(n2);
                    // Compute m
                    let m = if sub == 0 {
                        (p6 | p7 | !p9) & p8
                    } else {
                        (p2 | p3 | !p5) & p4
                    };
                    if c == 1 && (2..=3).contains(&n) && !m {
                        marked[idx] = true;
                    }
                }
            }
            // Remove marked pixels
            let mut removed_in_sub = false;
            for i in 0..current.len() {
                if marked[i] {
                    current[i] = false;
                    removed_in_sub = true;
                }
            }
            if removed_in_sub {
                changed = true;
            }
        }
    }
    current
}
