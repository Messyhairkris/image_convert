//! One-pixel lines to pixel chains (centerline tracing).

/// Direction steps in rule-1 order: E, SE, S, SW, W, NW, N, NE.
const STEPS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// True when `(x, y)` is inside the image and foreground (rule 1: outside is background).
fn at(fg: &[bool], width: usize, height: usize, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return false;
    }
    let (x, y) = (x as usize, y as usize);
    x < width && y < height && fg[y * width + x]
}

/// The direction indexes (into E, SE, S, SW, W, NW, N, NE) of the m-neighbours of
/// foreground pixel (x, y).
pub fn neighbours(fg: &[bool], width: usize, height: usize, x: usize, y: usize) -> Vec<usize> {
    let mut kept = Vec::new();
    if x >= width || y >= height {
        return kept;
    }
    let (xi, yi) = (x as i32, y as i32);
    for (k, &(dx, dy)) in STEPS.iter().enumerate() {
        if !at(fg, width, height, xi + dx, yi + dy) {
            continue;
        }
        // rule 2: a diagonal joined through a side pixel is not an m-neighbour.
        if dx != 0 && dy != 0 {
            let side_a = at(fg, width, height, xi + dx, yi);
            let side_b = at(fg, width, height, xi, yi + dy);
            if side_a || side_b {
                continue;
            }
        }
        kept.push(k);
    }
    kept
}

/// The image with the per-pixel m-neighbour and used-link state of rules 3 and 4.
struct Tracer<'a> {
    fg: &'a [bool],
    width: usize,
    height: usize,
    /// rule 3: the number of m-neighbours of each pixel.
    cnt: Vec<u8>,
    /// rule 2: the m-neighbour directions of each pixel, in rule-1 order.
    dirs: Vec<[u8; 8]>,
    /// rule 4: bit `k` is set when the link in direction `k` is used.
    used: Vec<u8>,
}

impl<'a> Tracer<'a> {
    /// Build the m-neighbour table for every foreground pixel.
    fn new(fg: &'a [bool], width: usize, height: usize) -> Self {
        let n = width * height;
        let mut cnt = vec![0u8; n];
        let mut dirs = vec![[0u8; 8]; n];
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                if !fg[i] {
                    continue;
                }
                let ns = neighbours(fg, width, height, x, y);
                cnt[i] = ns.len() as u8;
                for (j, &k) in ns.iter().enumerate() {
                    dirs[i][j] = k as u8;
                }
            }
        }
        Tracer {
            fg,
            width,
            height,
            cnt,
            dirs,
            used: vec![0u8; n],
        }
    }

    /// rule 3: a node is a foreground pixel whose degree is not 2.
    fn is_node(&self, i: usize) -> bool {
        self.fg[i] && self.cnt[i] != 2
    }

    /// The first m-neighbour direction of pixel `i` whose link is unused, if any.
    fn first_unused(&self, i: usize) -> Option<usize> {
        let ds = self.dirs[i];
        for &kd in ds.iter().take(self.cnt[i] as usize) {
            let k = kd as usize;
            if self.used[i] & (1u8 << k) == 0 {
                return Some(k);
            }
        }
        None
    }

    /// rule 5: walk from pixel `start` in direction `k0`, marking every link used.
    /// `loop_mode` selects the pass-2 STOP test (back at `start`) over the first node.
    fn walk(&mut self, start: usize, k0: usize, loop_mode: bool) -> Vec<(u32, u32)> {
        let mut chain = vec![self.point(start)];
        let mut cur = start;
        let mut k = k0;
        loop {
            let (dx, dy) = STEPS[k];
            let nx = (cur % self.width) as i32 + dx;
            let ny = (cur / self.width) as i32 + dy;
            let ni = ny as usize * self.width + nx as usize;
            self.used[cur] |= 1u8 << k;
            self.used[ni] |= 1u8 << ((k + 4) % 8);
            chain.push((nx as u32, ny as u32));
            let stop = if loop_mode {
                ni == start
            } else {
                self.cnt[ni] != 2
            };
            if stop {
                break;
            }
            match self.first_unused(ni) {
                Some(kk) => {
                    cur = ni;
                    k = kk;
                }
                None => break,
            }
        }
        chain
    }

    /// The `(x, y)` coordinates of pixel index `i`.
    fn point(&self, i: usize) -> (u32, u32) {
        ((i % self.width) as u32, (i / self.width) as u32)
    }
}

/// Every foreground pixel in chains of (x, y).
pub fn trace(fg: &[bool], width: usize, height: usize) -> Vec<Vec<(u32, u32)>> {
    let mut chains = Vec::new();
    if width == 0 || height == 0 || fg.len() < width * height {
        return chains;
    }
    let mut t = Tracer::new(fg, width, height);
    // rule 6: pass 1, the nodes in raster order.
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            if !t.is_node(i) {
                continue;
            }
            if t.cnt[i] == 0 {
                chains.push(vec![(x as u32, y as u32)]);
                continue;
            }
            let ds = t.dirs[i];
            for &kd in ds.iter().take(t.cnt[i] as usize) {
                let k = kd as usize;
                if t.used[i] & (1u8 << k) != 0 {
                    continue;
                }
                let chain = t.walk(i, k, false);
                chains.push(chain);
            }
        }
    }
    // rule 7: pass 2, the closed loops that contain no node.
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            if !t.fg[i] || t.is_node(i) {
                continue;
            }
            if let Some(k) = t.first_unused(i) {
                let chain = t.walk(i, k, true);
                chains.push(chain);
            }
        }
    }
    chains
}
