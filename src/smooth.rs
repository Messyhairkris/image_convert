//! Outline simplification and cubic Bezier fitting that keeps shared borders
//! identical on both sides.

/// A point in the plane.
pub type Pt = (f64, f64);

/// One cubic Bezier segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic {
    pub from: Pt,
    pub c1: Pt,
    pub c2: Pt,
    pub to: Pt,
}

/// Ramer-Douglas-Peucker on an open chain; the endpoints are always kept.
pub fn simplify(points: &[Pt], epsilon: f64) -> Vec<Pt> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let first = points[0];
    let last = points[n - 1];
    // rule 2: largest distance wins, ties go to the smallest index.
    let mut best = 0usize;
    let mut best_d = f64::NEG_INFINITY;
    for i in 1..n - 1 {
        let d = point_distance(points[i], first, last);
        if d > best_d {
            best_d = d;
            best = i;
        }
    }
    if best_d > epsilon {
        // rule 3: keep points[best]; the two halves share it, so drop the repeat.
        let mut out = simplify(&points[..=best], epsilon);
        let tail = simplify(&points[best..], epsilon);
        out.extend_from_slice(&tail[1..]);
        out
    } else {
        vec![first, last]
    }
}

/// simplify() run in the chain's canonical direction, returned in the given one.
pub fn simplify_canonical(chain: &[Pt], epsilon: f64) -> Vec<Pt> {
    let n = chain.len();
    if n < 3 {
        return chain.to_vec();
    }
    // rule 5: pick the direction whose start is smaller by (y, x).
    let reverse = if chain[0] != chain[n - 1] {
        smaller_by_yx(chain[n - 1], chain[0])
    } else {
        smaller_by_yx(chain[n - 2], chain[1])
    };
    if reverse {
        let mut rev: Vec<Pt> = chain.iter().rev().copied().collect();
        rev = simplify(&rev, epsilon);
        rev.reverse();
        rev
    } else {
        simplify(chain, epsilon)
    }
}

/// The chains of a loop: junction to junction, or the whole loop closed on itself.
pub fn split_chains(points: &[(u32, u32)], junction: &[bool]) -> Vec<Vec<Pt>> {
    let n = points.len();
    if n == 0 {
        return vec![Vec::new()];
    }
    let pts: Vec<Pt> = points.iter().map(|&(x, y)| (x as f64, y as f64)).collect();
    let mut junctions: Vec<usize> = Vec::new();
    for i in 0..n {
        if junction.get(i).copied().unwrap_or(false) {
            junctions.push(i);
        }
    }
    if junctions.is_empty() {
        // rule 7: no junction, so the whole loop closed back on its start.
        let mut chain = pts.clone();
        chain.push(pts[0]);
        return vec![chain];
    }
    // rule 8: junction to the next junction inclusive, the last one wrapping round.
    let mut chains = Vec::with_capacity(junctions.len());
    for k in 0..junctions.len() {
        let start = junctions[k];
        let end = junctions[(k + 1) % junctions.len()];
        let mut chain: Vec<Pt> = Vec::new();
        if end > start {
            chain.extend_from_slice(&pts[start..=end]);
        } else {
            chain.extend_from_slice(&pts[start..]);
            chain.extend_from_slice(&pts[..=end]);
        }
        chains.push(chain);
    }
    chains
}

/// The whole loop as cubic segments, in loop order.
pub fn smooth_loop(points: &[(u32, u32)], junction: &[bool], epsilon: f64) -> Vec<Cubic> {
    if points.len() < 2 {
        return Vec::new();
    }
    // rule 10: a closed loop is one with no junction at all (rule 7).
    let closed = !junction.iter().take(points.len()).any(|&j| j);
    let chains = split_chains(points, junction);
    let mut out: Vec<Cubic> = Vec::new();
    for chain in &chains {
        let q = simplify_canonical(chain, epsilon);
        if q.len() < 2 {
            continue;
        }
        let m = q.len() - 1;
        for i in 0..m {
            let a = q[i];
            let b = q[i + 1];
            let (c1, c2) = if epsilon == 0.0 {
                // straight lines: a + (b - a) / 3 and b - (b - a) / 3.
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                (
                    (a.0 + dx / 3.0, a.1 + dy / 3.0),
                    (b.0 - dx / 3.0, b.1 - dy / 3.0),
                )
            } else {
                let p0 = if i > 0 {
                    q[i - 1]
                } else if closed && q.len() > 2 {
                    q[m - 1]
                } else {
                    a
                };
                let p3 = if i + 1 < m {
                    q[i + 2]
                } else if closed && q.len() > 2 {
                    q[1]
                } else {
                    b
                };
                (
                    (a.0 + (b.0 - p0.0) / 6.0, a.1 + (b.1 - p0.1) / 6.0),
                    (b.0 - (p3.0 - a.0) / 6.0, b.1 - (p3.1 - a.1) / 6.0),
                )
            };
            out.push(Cubic {
                from: a,
                c1,
                c2,
                to: b,
            });
        }
    }
    out
}

/// Rule 2's distance from `p` to the segment `first`-`last` (or to `first` when
/// the two endpoints coincide).
fn point_distance(p: Pt, first: Pt, last: Pt) -> f64 {
    let (dx, dy) = (p.0 - first.0, p.1 - first.1);
    if first == last {
        (dx * dx + dy * dy).sqrt()
    } else {
        let (lx, ly) = (last.0 - first.0, last.1 - first.1);
        (lx * dy - ly * dx).abs() / (lx * lx + ly * ly).sqrt()
    }
}

/// "Smaller by (y, x)": compare y first, then x.
fn smaller_by_yx(a: Pt, b: Pt) -> bool {
    if a.1 != b.1 {
        a.1 < b.1
    } else {
        a.0 < b.0
    }
}
