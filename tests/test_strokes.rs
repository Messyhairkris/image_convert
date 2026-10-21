//! Integration tests for `photo_tools::strokes`.
//! Verifies the contract rules and all worked-example rows.

use photo_tools::strokes::{neighbours, trace};

/// Convert a textual grid (rows of `#` and `.`) into a row-major `Vec<bool>`.
fn parse_grid(grid: &[&str]) -> (Vec<bool>, usize, usize) {
    let height = grid.len();
    let width = if height > 0 { grid[0].len() } else { 0 };
    let mut fg = Vec::with_capacity(width * height);
    for row in grid {
        assert_eq!(row.len(), width, "all rows must have equal length");
        for ch in row.chars() {
            fg.push(ch == '#');
        }
    }
    (fg, width, height)
}

/// Helper to compare two `Vec<Vec<(u32,u32)>>` values.
fn assert_chains_eq(got: Vec<Vec<(u32, u32)>>, want: Vec<Vec<(u32, u32)>>) {
    assert_eq!(got, want);
}

/// Rule 1 - direction order is fixed. (Only a sanity check; the order is implicit in
/// `neighbours` and `trace` results, so we verify it via a known pattern.)
#[test]
fn rule_1_direction_order() {
    // A 3x3 block where the centre has all 8 neighbours.
    let (fg, w, h) = parse_grid(&["###", "###", "###"]);
    // Centre pixel (1,1) should report the four orthogonal directions (E,S,W,N)
    // because the diagonal steps are dropped by rule 2.
    let dirs = neighbours(&fg, w, h, 1, 1);
    assert_eq!(dirs, vec![0, 2, 4, 6]); // E,S,W,N in the fixed order
}

/// Rule 2 - diagonal steps are dropped when a side neighbour exists.
#[test]
fn rule_2_diagonal_drop_examples() {
    // Example 1 from the contract.
    let (fg1, w1, h1) = parse_grid(&["##.", ".##", "..#"]);
    // Pixel (1,0) -> S (2) and W (4)
    let dirs1 = neighbours(&fg1, w1, h1, 1, 0);
    assert_eq!(dirs1, vec![2, 4]);

    // Example 2 from the contract.
    let (fg2, w2, h2) = parse_grid(&["#..", ".#.", "..#"]);
    // Pixel (1,1) -> SE (1) and NW (5)
    let dirs2 = neighbours(&fg2, w2, h2, 1, 1);
    assert_eq!(dirs2, vec![1, 5]);

    // Example 3 - full 3x3 block.
    let (fg3, w3, h3) = parse_grid(&["###", "###", "###"]);
    // Pixel (1,1) -> orthogonal directions only.
    let dirs3 = neighbours(&fg3, w3, h3, 1, 1);
    assert_eq!(dirs3, vec![0, 2, 4, 6]);
}

/// Rule 3 - degree 2 pixels are not nodes; degree != 2 are nodes.
#[test]
fn rule_3_node_detection() {
    // The pattern `#.# / ... / #..` has three isolated foreground pixels.
    let (fg, w, h) = parse_grid(&["#.#", "...", "#.."]);
    // All three pixels have degree 0, therefore each forms its own chain.
    let chains = trace(&fg, w, h);
    let want = vec![vec![(0, 0)], vec![(2, 0)], vec![(0, 2)]];
    assert_chains_eq(chains, want);
}

/// Rule 4 - each link is used at most once (checked indirectly via the
/// staircase example where a naive implementation would create a fork).
#[test]
fn rule_4_link_usage_prevents_staircase_fork() {
    // Staircase pattern from the contract.
    let (fg, w, h) = parse_grid(&["##.", ".##", "..#"]);
    let chains = trace(&fg, w, h);
    let want = vec![vec![(0, 0), (1, 0), (1, 1), (2, 1), (2, 2)]];
    assert_chains_eq(chains, want);
}

/// Rule 5 - walking follows the first unused m-neighbour direction.
#[test]
fn rule_5_walk_follows_first_unused() {
    // Pattern `#.. / .#. / ..#` - a diagonal line.
    let (fg, w, h) = parse_grid(&["#..", ".#.", "..#"]);
    let chains = trace(&fg, w, h);
    let want = vec![vec![(0, 0), (1, 1), (2, 2)]];
    assert_chains_eq(chains, want);
}

/// Rule 6 - pass 1 creates chains from nodes in raster order.
#[test]
fn rule_6_pass1_node_order() {
    // Pattern `##### / ..#.. / ..#..` - a T-shape.
    let (fg, w, h) = parse_grid(&["#####", "..#..", "..#.."]);
    let chains = trace(&fg, w, h);
    let want = vec![
        vec![(0, 0), (1, 0), (2, 0)],
        vec![(2, 0), (3, 0), (4, 0)],
        vec![(2, 0), (2, 1), (2, 2)],
    ];
    assert_chains_eq(chains, want);
}

/// Rule 7 - pass 2 finds closed loops with no nodes.
#[test]
fn rule_7_pass2_closed_loop() {
    // Full 2x2 block - all pixels have degree 2, forming a loop.
    let (fg, w, h) = parse_grid(&["##", "##"]);
    let chains = trace(&fg, w, h);
    let want = vec![vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]];
    assert_chains_eq(chains, want);
}

/// Rule 8 - result order (pass 1 then pass 2) and every foreground pixel appears
/// in at least one chain.
#[test]
fn rule_8_result_order_and_coverage() {
    // Use a representative set of worked-example rows.
    let examples = [
        // 1. Simple horizontal line.
        (vec!["####"], vec![vec![(0, 0), (1, 0), (2, 0), (3, 0)]]),
        // 2. Single isolated pixel.
        (vec!["...", ".#.", "..."], vec![vec![(1, 1)]]),
        // 3. Three isolated pixels.
        (
            vec!["#.#", "...", "#.."],
            vec![vec![(0, 0)], vec![(2, 0)], vec![(0, 2)]],
        ),
        // 4. Staircase (already tested in rule 4, but we also check order).
        (
            vec!["##.", ".##", "..#"],
            vec![vec![(0, 0), (1, 0), (1, 1), (2, 1), (2, 2)]],
        ),
        // 5. Diagonal line.
        (
            vec!["#..", ".#.", "..#"],
            vec![vec![(0, 0), (1, 1), (2, 2)]],
        ),
        // 6. Full 3x3 ring (pass 2).
        (
            vec!["###", "#.#", "###"],
            vec![vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (2, 1),
                (2, 2),
                (1, 2),
                (0, 2),
                (0, 1),
                (0, 0),
            ]],
        ),
        // 7. 2x2 block (loop).
        (
            vec!["##", "##"],
            vec![vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]],
        ),
        // 8. Empty image.
        (vec![], vec![]),
    ];

    for (grid, expected) in examples.iter() {
        let (fg, w, h) = parse_grid(grid);
        let chains = trace(&fg, w, h);
        assert_chains_eq(chains.clone(), expected.clone());

        // Coverage check: every foreground pixel must appear in at least one chain.
        let mut covered = std::collections::HashSet::new();
        for chain in &chains {
            for &(x, y) in chain {
                covered.insert((x as usize, y as usize));
            }
        }
        for y in 0..h {
            for x in 0..w {
                if fg[y * w + x] {
                    assert!(
                        covered.contains(&(x, y)),
                        "pixel ({},{}) missing from all chains",
                        x,
                        y
                    );
                }
            }
        }
    }
}
