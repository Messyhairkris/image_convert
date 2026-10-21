//! Where region borders meet.

/// True where three or more values meet (outside counts as one value), or at a
/// checkerboard saddle of two labels with all four cells inside the image.
pub fn is_junction(labels: &[u8], width: usize, height: usize, x: usize, y: usize) -> bool {
    // Cell value at (px, py); OUTSIDE is -1, distinct from every u8 label.
    // i64 so x - 1 at x == 0 (and y - 1 at y == 0) cannot underflow.
    let cell = |px: i64, py: i64| -> i32 {
        if px < 0 || py < 0 || px >= width as i64 || py >= height as i64 {
            -1
        } else {
            labels[(py as usize) * width + (px as usize)] as i32
        }
    };

    let nw = cell(x as i64 - 1, y as i64 - 1);
    let ne = cell(x as i64, y as i64 - 1);
    let sw = cell(x as i64 - 1, y as i64);
    let se = cell(x as i64, y as i64);

    let values = [nw, ne, sw, se];
    // Count distinct values: position i is new if it differs from every earlier one.
    let distinct = (0..4)
        .filter(|&i| (0..i).all(|j| values[j] != values[i]))
        .count();

    // Rule 2: three or more distinct values (OUTSIDE counts as one value).
    if distinct >= 3 {
        return true;
    }

    // Rule 3: checkerboard saddle of two labels, all four cells inside.
    let all_inside = x > 0 && y > 0 && x < width && y < height;
    all_inside && nw == se && ne == sw && nw != ne
}
