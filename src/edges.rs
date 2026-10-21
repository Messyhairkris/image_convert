//! Directed pixel-edge boundaries of each label, linked into closed paths.

/// One closed boundary path before simplification: every grid vertex it passes, in
/// travel order, and the direction of the edge leaving each vertex
/// (0 east, 1 south, 2 west, 3 north).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLoop {
    pub label: u8,
    pub verts: Vec<(u32, u32)>,
    pub dirs: Vec<u8>,
}

/// Every boundary edge as (label, start vertex, direction), ordered by start vertex
/// index y * (width + 1) + x, then by direction.
pub fn boundary_edges(labels: &[u8], width: usize, height: usize) -> Vec<(u8, (u32, u32), u8)> {
    let slots = edge_slots(labels, width, height);
    let cols = width + 1;
    let mut out = Vec::new();
    for (slot, owner) in slots.iter().enumerate() {
        if let Some(label) = *owner {
            let vertex_index = slot / 4;
            let direction = (slot % 4) as u8;
            let x = (vertex_index % cols) as u32;
            let y = (vertex_index / cols) as u32;
            out.push((label, (x, y), direction));
        }
    }
    out
}

/// The edges linked into closed loops, in the order their first edge appears in
/// boundary_edges; at a vertex with a choice, turn right, else straight, else left.
pub fn raw_loops(labels: &[u8], width: usize, height: usize) -> Vec<RawLoop> {
    let slots = edge_slots(labels, width, height);
    let cols = width + 1;
    let mut used = vec![false; slots.len()];
    let mut loops = Vec::new();
    for start in 0..slots.len() {
        if used[start] {
            continue;
        }
        let Some(label) = slots[start] else {
            continue;
        };
        let mut verts = Vec::new();
        let mut dirs = Vec::new();
        let mut cur = start;
        loop {
            used[cur] = true;
            let vertex_index = cur / 4;
            let d = (cur % 4) as u8;
            let x = vertex_index % cols;
            let y = vertex_index / cols;
            verts.push((x as u32, y as u32));
            dirs.push(d);
            let (ex, ey) = match d {
                0 => (x + 1, y),
                1 => (x, y + 1),
                2 => (x - 1, y),
                _ => (x, y - 1),
            };
            let end_vertex = ey * cols + ex;
            // Rule 3: right, else straight, else left; never back.
            let mut next = None;
            for cand in [d + 1, d, d + 3] {
                let idx = end_vertex * 4 + (cand % 4) as usize;
                if !used[idx] && slots[idx] == Some(label) {
                    next = Some(idx);
                    break;
                }
            }
            match next {
                Some(idx) => cur = idx,
                None => break,
            }
        }
        loops.push(RawLoop { label, verts, dirs });
    }
    loops
}

/// One slot per possible edge, index `vertex_index * 4 + direction` with
/// `vertex_index = y * (width + 1) + x`, holding the owning label.
fn edge_slots(labels: &[u8], width: usize, height: usize) -> Vec<Option<u8>> {
    let cols = width + 1;
    let mut slots = vec![None; (cols * (height + 1)) * 4];
    for py in 0..height {
        for px in 0..width {
            let label = labels[py * width + px];
            // Rule 1: a side is an edge when the neighbouring cell is not the
            // pixel's label (another label, or outside the image).
            if py == 0 || labels[(py - 1) * width + px] != label {
                slots[vertex_slot(px, py, cols, 0)] = Some(label);
            }
            if px + 1 == width || labels[py * width + px + 1] != label {
                slots[vertex_slot(px + 1, py, cols, 1)] = Some(label);
            }
            if py + 1 == height || labels[(py + 1) * width + px] != label {
                slots[vertex_slot(px + 1, py + 1, cols, 2)] = Some(label);
            }
            if px == 0 || labels[py * width + px - 1] != label {
                slots[vertex_slot(px, py + 1, cols, 3)] = Some(label);
            }
        }
    }
    slots
}

fn vertex_slot(x: usize, y: usize, cols: usize, direction: usize) -> usize {
    (y * cols + x) * 4 + direction
}
