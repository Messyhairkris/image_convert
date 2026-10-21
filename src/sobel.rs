//! Sobel gradient magnitude and edge mask.

/// round(sqrt(gx^2 + gy^2)) per pixel; samples outside the image use the nearest
/// edge pixel (coordinates clamped).
pub fn magnitude(gray: &[u8], width: usize, height: usize) -> Vec<u16> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    const KX: [[i32; 3]; 3] = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    const KY: [[i32; 3]; 3] = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
    let mut res = Vec::with_capacity(gray.len());
    for y in 0..height {
        for x in 0..width {
            let mut gx = 0i32;
            let mut gy = 0i32;
            for j in 0..3 {
                for i in 0..3 {
                    let x_offset = i as isize - 1;
                    let y_offset = j as isize - 1;
                    let sx = (x as isize + x_offset).clamp(0, width as isize - 1) as usize;
                    let sy = (y as isize + y_offset).clamp(0, height as isize - 1) as usize;
                    let pixel = gray[sy * width + sx] as i32;
                    gx += KX[j][i] * pixel;
                    gy += KY[j][i] * pixel;
                }
            }
            let mag = ((gx * gx + gy * gy) as f64).sqrt().round() as u16;
            res.push(mag);
        }
    }
    res
}

/// 0 (black) where magnitude >= threshold, else 255 (white).
pub fn edge_mask(gray: &[u8], width: usize, height: usize, threshold: u16) -> Vec<u8> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let mag = magnitude(gray, width, height);
    mag.into_iter()
        .map(|m| if m >= threshold { 0 } else { 255 })
        .collect()
}
