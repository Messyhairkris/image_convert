//! Integration tests for `photo_tools::edges` module.

use photo_tools::edges::{boundary_edges, raw_loops, RawLoop};

#[test]
fn rule_1_edges_single_pixel() {
    // Single pixel label 3, 1x1 image.
    let labels = &[3];
    let got = boundary_edges(labels, 1, 1);
    let want = vec![
        (3, (0, 0), 0),
        (3, (1, 0), 1),
        (3, (0, 1), 3),
        (3, (1, 1), 2),
    ];
    assert_eq!(got, want);
}

#[test]
fn rule_2_order_shared_vertex() {
    // Two pixels with different labels, check ordering at shared vertex (1,0).
    let labels = &[0, 1];
    let got = boundary_edges(labels, 2, 1);
    let want = vec![
        (0, (0, 0), 0),
        (1, (1, 0), 0),
        (0, (1, 0), 1),
        (1, (2, 0), 1),
        (0, (0, 1), 3),
        (0, (1, 1), 2),
        (1, (1, 1), 3),
        (1, (2, 1), 2),
    ];
    assert_eq!(got, want);
}

#[test]
fn rule_3_linking_two_same_label() {
    // Two adjacent pixels with same label 0, 2x1 image.
    let labels = &[0, 0];
    let got = raw_loops(labels, 2, 1);
    let want = vec![RawLoop {
        label: 0,
        verts: vec![(0, 0), (1, 0), (2, 0), (2, 1), (1, 1), (0, 1)],
        dirs: vec![0, 0, 1, 2, 2, 3],
    }];
    assert_eq!(got, want);
}

#[test]
fn rule_4_linking_two_labels() {
    // Two adjacent pixels with different labels, 2x1 image.
    let labels = &[0, 1];
    let got = raw_loops(labels, 2, 1);
    let want = vec![
        RawLoop {
            label: 0,
            verts: vec![(0, 0), (1, 0), (1, 1), (0, 1)],
            dirs: vec![0, 1, 2, 3],
        },
        RawLoop {
            label: 1,
            verts: vec![(1, 0), (2, 0), (2, 1), (1, 1)],
            dirs: vec![0, 1, 2, 3],
        },
    ];
    assert_eq!(got, want);
}

#[test]
fn rule_5_hole_example() {
    // 3x3 image with a hole (center label 1, surrounding label 0).
    let labels = &[0, 0, 0, 0, 1, 0, 0, 0, 0];
    let got = raw_loops(labels, 3, 3);
    let want = vec![
        RawLoop {
            label: 0,
            verts: vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (3, 1),
                (3, 2),
                (3, 3),
                (2, 3),
                (1, 3),
                (0, 3),
                (0, 2),
                (0, 1),
            ],
            dirs: vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3],
        },
        RawLoop {
            label: 1,
            verts: vec![(1, 1), (2, 1), (2, 2), (1, 2)],
            dirs: vec![0, 1, 2, 3],
        },
        RawLoop {
            label: 0,
            verts: vec![(1, 1), (1, 2), (2, 2), (2, 1)],
            dirs: vec![1, 0, 3, 2],
        },
    ];
    assert_eq!(got, want);
}

#[test]
fn rule_6_saddle_example() {
    // 2x2 image with a saddle configuration.
    let labels = &[0, 1, 1, 0];
    let got = raw_loops(labels, 2, 2);
    let want = vec![
        RawLoop {
            label: 0,
            verts: vec![(0, 0), (1, 0), (1, 1), (0, 1)],
            dirs: vec![0, 1, 2, 3],
        },
        RawLoop {
            label: 1,
            verts: vec![(1, 0), (2, 0), (2, 1), (1, 1)],
            dirs: vec![0, 1, 2, 3],
        },
        RawLoop {
            label: 1,
            verts: vec![(0, 1), (1, 1), (1, 2), (0, 2)],
            dirs: vec![0, 1, 2, 3],
        },
        RawLoop {
            label: 0,
            verts: vec![(1, 1), (2, 1), (2, 2), (1, 2)],
            dirs: vec![0, 1, 2, 3],
        },
    ];
    assert_eq!(got, want);
}

#[test]
fn rule_7_empty_image() {
    // Empty image yields no edges and no loops.
    let labels: &[u8] = &[];
    let edges = boundary_edges(labels, 0, 0);
    let loops = raw_loops(labels, 0, 0);
    assert_eq!(edges, Vec::<(u8, (u32, u32), u8)>::new());
    assert_eq!(loops, Vec::<RawLoop>::new());
}
