//! Region outlines along pixel edges.

use crate::edges::raw_loops;
pub use crate::junction::is_junction;

/// One closed outline of one label. `points[i]` is a grid vertex (x, y) with
/// 0 <= x <= width and 0 <= y <= height; the last point joins back to the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    pub label: u8,
    pub points: Vec<(u32, u32)>,
    pub junction: Vec<bool>,
}

/// Every outline of every label, sorted by label, then by first point (y, x).
pub fn trace(labels: &[u8], width: usize, height: usize) -> Vec<Loop> {
    let raw = raw_loops(labels, width, height);
    let mut loops: Vec<Loop> = Vec::with_capacity(raw.len());
    for rl in raw {
        loops.push(simplify(
            labels, width, height, rl.label, &rl.verts, &rl.dirs,
        ));
    }
    // Rule 4: label, then first point (y, x), then second point (y, x).
    loops.sort_by_key(|l| (l.label, yx(&l.points[0]), yx(&l.points[1])));
    loops
}

/// Rule 2: keep corners and junctions, drop straight-through points; then
/// rule 3: rotate to the fixed start.
fn simplify(
    labels: &[u8],
    width: usize,
    height: usize,
    label: u8,
    verts: &[(u32, u32)],
    dirs: &[u8],
) -> Loop {
    let n = verts.len();
    let mut points: Vec<(u32, u32)> = Vec::with_capacity(n);
    let mut junction: Vec<bool> = Vec::with_capacity(n);
    for i in 0..n {
        let (x, y) = verts[i];
        let arriving = dirs[(i + n - 1) % n];
        let leaving = dirs[i];
        let is_j = is_junction(labels, width, height, x as usize, y as usize);
        // Rule 2: keep a corner or a junction, drop a straight-through point.
        if arriving != leaving || is_j {
            points.push((x, y));
            junction.push(is_j);
        }
    }
    // Rule 3: rotate both vectors to the fixed start.
    if let Some(k) = start_index(&points) {
        points.rotate_left(k);
        junction.rotate_left(k);
    }
    Loop {
        label,
        points,
        junction,
    }
}

/// Rule 3: index of the smallest point by (y, x); if that point occurs more
/// than once, the occurrence whose following point is smaller by (y, x).
fn start_index(points: &[(u32, u32)]) -> Option<usize> {
    let n = points.len();
    if n == 0 {
        return None;
    }
    let mut best = 0;
    for i in 1..n {
        let a = yx(&points[i]);
        let b = yx(&points[best]);
        if a < b || (a == b && follow(points, i) < follow(points, best)) {
            best = i;
        }
    }
    Some(best)
}

/// The point after index `i`, wrapping round, as a (y, x) key for the rule 3
/// tie-break.
fn follow(points: &[(u32, u32)], i: usize) -> (u32, u32) {
    yx(&points[(i + 1) % points.len()])
}

/// A vertex (x, y) as a (y, x) comparison key.
fn yx(p: &(u32, u32)) -> (u32, u32) {
    (p.1, p.0)
}
