//! Floyd-Steinberg error diffusion to one bit.

/// Every output byte is 0 or 255. Rows top to bottom, each left to right.
pub fn floyd_steinberg(gray: &[u8], width: usize, height: usize) -> Vec<u8> {
    // Rule 1: copy into a working i32 buffer.
    let mut buffer: Vec<i32> = gray.iter().map(|&b| b as i32).collect();

    // Result buffer, same layout as input.
    let mut out: Vec<u8> = Vec::with_capacity(width * height);

    // Visit each pixel row-major, left-to-right.
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let old = buffer[idx];
            // Rule 3: decide new pixel value.
            let new_val: u8 = if old >= 128 { 255 } else { 0 };
            out.push(new_val);
            let err = old - new_val as i32; // may be negative

            // Rule 4: distribute error to existing neighbours.
            // Right (x+1, y)
            if x + 1 < width {
                let share = err * 7 / 16;
                let n_idx = idx + 1;
                buffer[n_idx] += share;
            }
            // Down-left (x-1, y+1)
            if x > 0 && y + 1 < height {
                let share = err * 3 / 16;
                let n_idx = (y + 1) * width + (x - 1);
                buffer[n_idx] += share;
            }
            // Down (x, y+1)
            if y + 1 < height {
                let share = err * 5 / 16;
                let n_idx = (y + 1) * width + x;
                buffer[n_idx] += share;
            }
            // Down-right (x+1, y+1)
            if x + 1 < width && y + 1 < height {
                let share = err / 16;
                let n_idx = (y + 1) * width + (x + 1);
                buffer[n_idx] += share;
            }
        }
    }

    out
}
