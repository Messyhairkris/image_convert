//! Integration tests for smooth.rs: Ramer-Douglas-Peucker simplification,
//! canonical direction, chain splitting and cubic Bezier fitting, written
//! from the module contract (rules 1-11) and its worked examples.

use photo_tools::smooth::{simplify, simplify_canonical, smooth_loop, split_chains, Cubic, Pt};

/// Contract rule 10 with epsilon == 0.0: c1 = a + (b - a) / 3, c2 = b - (b - a) / 3,
/// computed with the exact arithmetic forms the contract requires.
fn straight(a: Pt, b: Pt) -> Cubic {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    Cubic {
        from: a,
        c1: (a.0 + dx / 3.0, a.1 + dy / 3.0),
        c2: (b.0 - dx / 3.0, b.1 - dy / 3.0),
        to: b,
    }
}

/// Contract rule 10 Catmull-Rom: c1 = a + (b - p0) / 6, c2 = b - (p3 - a) / 6.
fn catmull_rom(a: Pt, b: Pt, p0: Pt, p3: Pt) -> Cubic {
    Cubic {
        from: a,
        c1: (a.0 + (b.0 - p0.0) / 6.0, a.1 + (b.1 - p0.1) / 6.0),
        c2: (b.0 - (p3.0 - a.0) / 6.0, b.1 - (p3.1 - a.1) / 6.0),
        to: b,
    }
}

#[test]
fn rule_1_fewer_than_3_points_returned_unchanged() {
    assert_eq!(simplify(&[], 1.0), Vec::<Pt>::new());
    assert_eq!(simplify(&[(1.0, 2.0)], 1.0), vec![(1.0, 2.0)]);
    assert_eq!(
        simplify(&[(0.0, 0.0), (1.0, 1.0)], 1.0),
        vec![(0.0, 0.0), (1.0, 1.0)]
    );
}

#[test]
fn rule_3_distance_must_be_strictly_greater_than_epsilon() {
    // Perpendicular distance of (1, 0.1) from the (0,0)-(2,0) line is 0.1.
    assert_eq!(
        simplify(&[(0.0, 0.0), (1.0, 0.1), (2.0, 0.0)], 0.5),
        vec![(0.0, 0.0), (2.0, 0.0)]
    );
    assert_eq!(
        simplify(&[(0.0, 0.0), (1.0, 0.1), (2.0, 0.0)], 0.05),
        vec![(0.0, 0.0), (1.0, 0.1), (2.0, 0.0)]
    );
    // Both (1,1) and (3,-1) sit at distance exactly 1 from the (0,0)-(4,0) line.
    assert_eq!(
        simplify(&[(0.0, 0.0), (1.0, 1.0), (3.0, -1.0), (4.0, 0.0)], 1.0),
        vec![(0.0, 0.0), (4.0, 0.0)],
        "distance equal to epsilon is not > epsilon"
    );
}

#[test]
fn rule_2_ties_go_to_the_smallest_index() {
    // Rule 2 tie between (1,1) and (3,-1): (1,1) wins; afterwards (3,-1) is
    // about 1.265 from the (1,1)-(4,0) line, so with epsilon 0.99 all stay.
    assert_eq!(
        simplify(&[(0.0, 0.0), (1.0, 1.0), (3.0, -1.0), (4.0, 0.0)], 0.99),
        vec![(0.0, 0.0), (1.0, 1.0), (3.0, -1.0), (4.0, 0.0)]
    );
    // The LARGEST distance wins: (2,3) is picked before anything else.
    assert_eq!(
        simplify(
            &[
                (0.0, 0.0),
                (1.0, 0.0),
                (2.0, 3.0),
                (3.0, 0.0),
                (4.0, 0.2),
                (5.0, 0.0)
            ],
            0.5,
        ),
        vec![(0.0, 0.0), (1.0, 0.0), (2.0, 3.0), (3.0, 0.0), (5.0, 0.0)]
    );
}

#[test]
fn rule_2_first_equals_last_measures_distance_from_that_point() {
    // Closed chain: every distance is measured from (0,0), not from a line.
    let closed = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)];
    assert_eq!(
        simplify(&closed, 0.5),
        vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]
    );
    // Farthest point is (2,2) at sqrt(8) ~= 2.83, not > 3.0.
    assert_eq!(simplify(&closed, 3.0), vec![(0.0, 0.0), (0.0, 0.0)]);
}

#[test]
fn rule_4_canonical_short_chain_returned_unchanged() {
    let chain = [(2.0, 0.0), (0.0, 0.0)];
    assert_eq!(simplify_canonical(&chain, 0.5), chain.to_vec());
}

#[test]
fn rule_6_reversed_chain_is_simplified_then_reversed_back() {
    // chain[n-1] = (0,0) is smaller by (y, x) than chain[0] = (2,0): rule 5 reverse.
    let backwards = [(2.0, 0.0), (1.0, 0.1), (0.0, 0.0)];
    assert_eq!(
        simplify_canonical(&backwards, 0.05),
        vec![(2.0, 0.0), (1.0, 0.1), (0.0, 0.0)]
    );
    assert_eq!(
        simplify_canonical(&backwards, 0.5),
        vec![(2.0, 0.0), (0.0, 0.0)],
        "the result comes back in the chain's given direction"
    );
}

#[test]
fn rule_5_6_both_directions_give_the_same_simplified_chain() {
    let forward = [(0.0, 0.0), (1.0, -1.0), (2.0, 0.0), (3.0, 1.0), (4.0, 0.0)];
    let backward = [(4.0, 0.0), (3.0, 1.0), (2.0, 0.0), (1.0, -1.0), (0.0, 0.0)];
    let forward_out = simplify_canonical(&forward, 0.5);
    let backward_out = simplify_canonical(&backward, 0.5);
    let flipped: Vec<Pt> = forward_out.iter().rev().copied().collect();
    assert_eq!(backward_out, flipped);
    // Hand check of the forward chain: the tie at distance 1 between (1,-1) and
    // (3,1) goes to (1,-1); on the right half (3,1) is 4/sqrt(10) ~= 1.265 > 0.5
    // from the (1,-1)-(4,0) line while (2,0) ends up on the (1,-1)-(3,1) line.
    assert_eq!(
        forward_out,
        vec![(0.0, 0.0), (1.0, -1.0), (3.0, 1.0), (4.0, 0.0)]
    );
}

#[test]
fn rule_5_closed_chain_reverses_on_chain_n_minus_2() {
    let clockwise = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)];
    let counter = [(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)];
    // For `counter`: chain[n-2] = (2,0) is smaller by (y, x) than chain[1] = (0,2),
    // so it is reversed to `clockwise` before simplifying.
    assert_eq!(
        simplify_canonical(&clockwise, 3.0),
        vec![(0.0, 0.0), (0.0, 0.0)]
    );
    assert_eq!(
        simplify_canonical(&counter, 3.0),
        vec![(0.0, 0.0), (0.0, 0.0)]
    );
    assert_eq!(
        simplify_canonical(&counter, 0.5),
        counter.to_vec(),
        "reverse, simplify (keeps all), reverse back"
    );
}

#[test]
fn rule_7_no_junction_one_chain_closed_back_on_start() {
    let chains = split_chains(&[(0, 0), (2, 0), (2, 1), (0, 1)], &[false; 4]);
    assert_eq!(
        chains,
        vec![vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (0.0, 1.0),
            (0.0, 0.0)
        ]]
    );
}

#[test]
fn rule_8_exactly_one_junction_one_chain_all_the_way_round() {
    let chains = split_chains(
        &[(0, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
        &[false, false, true, false, false],
    );
    assert_eq!(
        chains,
        vec![vec![
            (2.0, 1.0),
            (1.0, 1.0),
            (0.0, 1.0),
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0)
        ]]
    );
}

#[test]
fn rule_8_several_junctions_one_forward_chain_per_junction() {
    let chains = split_chains(
        &[(0, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
        &[false, false, true, true, true],
    );
    // The last chain wraps past the end of the list back to the first junction.
    assert_eq!(
        chains,
        vec![
            vec![(2.0, 1.0), (1.0, 1.0)],
            vec![(1.0, 1.0), (0.0, 1.0)],
            vec![(0.0, 1.0), (0.0, 0.0), (2.0, 0.0), (2.0, 1.0)],
        ]
    );
}

#[test]
fn rule_9_fewer_than_2_points_no_segments() {
    assert_eq!(smooth_loop(&[], &[], 0.5), Vec::<Cubic>::new());
    assert_eq!(smooth_loop(&[(5, 3)], &[false], 0.5), Vec::<Cubic>::new());
}

#[test]
fn rule_10_epsilon_zero_gives_straight_line_segments() {
    let segments = smooth_loop(&[(0, 0), (1, 0), (1, 1), (0, 1)], &[false; 4], 0.0);
    assert_eq!(
        segments,
        vec![
            // First segment: from (0,0), c1 (1/3, 0), c2 (2/3, 0), to (1,0).
            straight((0.0, 0.0), (1.0, 0.0)),
            straight((1.0, 0.0), (1.0, 1.0)),
            straight((1.0, 1.0), (0.0, 1.0)),
            // The last segment ends back at (0,0).
            straight((0.0, 1.0), (0.0, 0.0)),
        ]
    );
}

#[test]
fn rule_10_catmull_rom_closed_loop_wraps_p0_and_p3() {
    let segments = smooth_loop(&[(0, 0), (1, 0), (1, 1), (0, 1)], &[false; 4], 0.5);
    // q = all five points, m = 4, closed: segment 0 wraps p0 = q[3] = (0,1),
    // the last segment wraps p3 = q[1] = (1,0).
    assert_eq!(
        segments,
        vec![
            // c1 = (1/6, -1/6), c2 = (5/6, -1/6).
            catmull_rom((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)),
            catmull_rom((1.0, 0.0), (1.0, 1.0), (0.0, 0.0), (0.0, 1.0)),
            // Segment 2: c1 = (5/6, 7/6).
            catmull_rom((1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (0.0, 0.0)),
            catmull_rom((0.0, 1.0), (0.0, 0.0), (1.0, 1.0), (1.0, 0.0)),
        ]
    );
}

#[test]
fn rule_11_all_segments_of_all_chains_in_chain_order() {
    let segments = smooth_loop(
        &[(0, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
        &[false, false, true, true, true],
        0.5,
    );
    // Chains [(2,1),(1,1)] and [(1,1),(0,1)] are clamped at both ends;
    // chain (0,1) (0,0) (2,0) (2,1) keeps all four points under RDP.
    assert_eq!(
        segments,
        vec![
            // from (2,1), c1 = (2 - 1/6, 1), c2 = (1 + 1/6, 1).
            catmull_rom((2.0, 1.0), (1.0, 1.0), (2.0, 1.0), (1.0, 1.0)),
            catmull_rom((1.0, 1.0), (0.0, 1.0), (1.0, 1.0), (0.0, 1.0)),
            // p0 clamped to a, p3 = (2,0): c1 = (0, 1 - 1/6), c2 = (0 - 2/6, 1/6).
            catmull_rom((0.0, 1.0), (0.0, 0.0), (0.0, 1.0), (2.0, 0.0)),
            catmull_rom((0.0, 0.0), (2.0, 0.0), (0.0, 1.0), (2.0, 1.0)),
            // The last segment ends at (2,1).
            catmull_rom((2.0, 0.0), (2.0, 1.0), (0.0, 0.0), (2.0, 1.0)),
        ]
    );
}

#[test]
fn rule_5_6_walking_a_loop_backwards_gives_the_same_segments_reversed() {
    let forward = smooth_loop(&[(0, 0), (1, 0), (1, 1), (0, 1)], &[false; 4], 0.5);
    let backward = smooth_loop(&[(0, 0), (0, 1), (1, 1), (1, 0)], &[false; 4], 0.5);
    // Hand-computed segments for the backwards walk (q = the loop backwards).
    assert_eq!(
        backward,
        vec![
            catmull_rom((0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)),
            catmull_rom((0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)),
            catmull_rom((1.0, 1.0), (1.0, 0.0), (0.0, 1.0), (0.0, 0.0)),
            catmull_rom((1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (0.0, 1.0)),
        ]
    );
    // Bit for bit: the backward segments are the forward ones reversed and flipped.
    let flipped: Vec<Cubic> = forward
        .iter()
        .rev()
        .map(|s| Cubic {
            from: s.to,
            c1: s.c2,
            c2: s.c1,
            to: s.from,
        })
        .collect();
    assert_eq!(backward, flipped);
}
