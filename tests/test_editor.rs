//! Integration tests for `photo_tools::editor`, written from the contract alone.

use photo_tools::editor::{Editor, Event, Item, Mods, Tool};
use photo_tools::select::{build, Op, Shape};

fn press(x: f32, y: f32) -> Event {
    Event::Press {
        at: [x, y],
        mods: Mods::default(),
    }
}

fn press_shift(x: f32, y: f32) -> Event {
    Event::Press {
        at: [x, y],
        mods: Mods {
            shift: true,
            ctrl: false,
        },
    }
}

fn press_ctrl(x: f32, y: f32) -> Event {
    Event::Press {
        at: [x, y],
        mods: Mods {
            shift: false,
            ctrl: true,
        },
    }
}

fn press_ctrl_shift(x: f32, y: f32) -> Event {
    Event::Press {
        at: [x, y],
        mods: Mods {
            shift: true,
            ctrl: true,
        },
    }
}

fn drag(x: f32, y: f32) -> Event {
    Event::Drag { at: [x, y] }
}

fn release(x: f32, y: f32) -> Event {
    Event::Release { at: [x, y] }
}

/// Lasso A: a committed triangle [[0,0],[4,0],[4,4]].
fn lasso_a(ed: &mut Editor) {
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.handle(release(4.0, 4.0));
}

/// Lasso B: a committed triangle [[10,10],[14,10],[14,14]].
fn lasso_b(ed: &mut Editor) {
    ed.handle(press(10.0, 10.0));
    ed.handle(drag(14.0, 10.0));
    ed.handle(release(14.0, 14.0));
}

fn poly(points: &[[f32; 2]]) -> Shape {
    Shape::Polygon(points.to_vec())
}

#[test]
fn rule_5_lasso_press_drag_release_skips_near_and_equal() {
    let mut ed = Editor::new();
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(0.5, 0.0)); // under 1.0 from (0,0): skipped
    ed.handle(drag(4.0, 0.0));
    ed.handle(drag(4.0, 4.0));
    ed.handle(release(4.0, 4.0)); // equals the last point: not added again
    let want = vec![(Op::Replace, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]))];
    assert_eq!(ed.selection(), want.as_slice());
    assert_eq!(ed.pending(), None);
}

#[test]
fn rule_4_two_point_lasso_is_dropped() {
    let mut ed = Editor::new();
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.handle(release(4.0, 0.0)); // equals last: not added, then FINISH with 2 points
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
    assert!(!ed.can_undo());
}

#[test]
fn rule_2_shift_is_add() {
    let mut ed = Editor::new();
    ed.handle(press_shift(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.handle(release(4.0, 4.0));
    let want = vec![(Op::Add, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]))];
    assert_eq!(ed.selection(), want.as_slice());
}

#[test]
fn rule_2_ctrl_beats_shift() {
    let mut ed = Editor::new();
    ed.handle(press_ctrl_shift(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.handle(release(4.0, 4.0));
    let want = vec![(Op::Subtract, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]))];
    assert_eq!(ed.selection(), want.as_slice());
}

#[test]
fn rule_5_polyline_closes_near_first_point() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Polyline);
    ed.handle(press(0.0, 0.0));
    ed.handle(press(10.0, 0.0));
    ed.handle(press(10.0, 10.0));
    ed.handle(press(0.5, 0.5)); // closer than close_radius 8.0 to (0,0): FINISH
    let want = vec![(Op::Replace, poly(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]))];
    assert_eq!(ed.selection(), want.as_slice());
    assert_eq!(ed.pending(), None);
}

#[test]
fn rule_5_polyline_double_press_adds_nothing() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Polyline);
    ed.handle(press(0.0, 0.0));
    ed.handle(press(10.0, 0.0));
    ed.handle(press(10.0, 0.0)); // same spot: adds nothing
    ed.handle(press(10.0, 10.0));
    ed.handle(Event::Close);
    let want = vec![(Op::Replace, poly(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]))];
    assert_eq!(ed.selection(), want.as_slice());
}

#[test]
fn rule_9_back_then_close_drops_two_point_polyline() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Polyline);
    ed.handle(press(0.0, 0.0));
    ed.handle(press(10.0, 0.0));
    ed.handle(Event::Back); // leaves (0,0)
    ed.handle(press(5.0, 5.0));
    ed.handle(Event::Close); // 2 points: dropped
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
    assert!(!ed.can_undo());
}

#[test]
fn rule_9_back_removes_only_point() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Polyline);
    ed.handle(press(0.0, 0.0));
    ed.handle(Event::Back);
    assert_eq!(ed.pending(), None);
}

#[test]
fn rule_5_close_radius_zero_never_closes() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Polyline);
    ed.set_close_radius(0.0);
    ed.handle(press(0.0, 0.0));
    ed.handle(press(10.0, 0.0));
    ed.handle(press(10.0, 10.0));
    ed.handle(press(0.0, 0.0)); // far(first, at, 0.0) is always true: not closed
    let want = Some(Item::Select(
        Op::Replace,
        poly(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]]),
    ));
    assert_eq!(ed.pending(), want);
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
}

#[test]
fn rule_7_brush_click_is_a_dot() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Brush);
    ed.handle(press(5.0, 5.0));
    ed.handle(release(5.0, 5.0)); // equal to last: not added, one-point stroke
    let want = vec![(
        Op::Replace,
        Shape::Stroke {
            points: vec![[5.0, 5.0]],
            radius: 12.0,
        },
    )];
    assert_eq!(ed.selection(), want.as_slice());
}

#[test]
fn rule_6_brush_min_step_is_radius_over_four() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Brush);
    ed.set_radius(8.0); // min step 2.0
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(1.0, 0.0)); // 1.0 < 2.0: skipped
    ed.handle(drag(2.0, 0.0));
    ed.handle(drag(3.0, 0.0)); // 1.0 < 2.0: skipped
    ed.handle(release(3.0, 0.0)); // release adds regardless of step
    let want = vec![(
        Op::Replace,
        Shape::Stroke {
            points: vec![[0.0, 0.0], [2.0, 0.0], [3.0, 0.0]],
            radius: 8.0,
        },
    )];
    assert_eq!(ed.selection(), want.as_slice());
}

#[test]
fn rule_12_eraser_ignores_keys() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Eraser);
    ed.handle(press_ctrl(1.0, 1.0));
    ed.handle(release(2.0, 1.0));
    let want = vec![Shape::Stroke {
        points: vec![[1.0, 1.0], [2.0, 1.0]],
        radius: 12.0,
    }];
    assert_eq!(ed.erased(), want.as_slice());
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
}

#[test]
fn rule_1_radius_clamps_and_ignores_nan() {
    let mut ed = Editor::new();
    assert_eq!(ed.radius(), 12.0);
    ed.set_radius(0.2);
    assert_eq!(ed.radius(), 1.0);
    ed.set_radius(f32::NAN);
    assert_eq!(ed.radius(), 1.0);
    ed.set_radius(1000.0);
    assert_eq!(ed.radius(), 500.0);
    assert_eq!(ed.close_radius(), 8.0);
    ed.set_close_radius(200.0);
    assert_eq!(ed.close_radius(), 100.0);
    ed.set_close_radius(f32::NAN);
    assert_eq!(ed.close_radius(), 100.0);
}

#[test]
fn rule_11_undo_with_shape_in_progress_only_drops_it() {
    let mut ed = Editor::new();
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.handle(drag(4.0, 4.0));
    ed.handle(Event::Undo); // pending exists: drop it, nothing else
    assert_eq!(ed.pending(), None);
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
    assert!(!ed.can_undo());
}

#[test]
fn rule_11_undo_and_redo_a_committed_selection() {
    let mut ed = Editor::new();
    lasso_a(&mut ed);
    lasso_b(&mut ed);
    ed.handle(Event::Undo);
    let a = vec![(Op::Replace, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]))];
    assert_eq!(ed.selection(), a.as_slice());
    assert!(ed.can_redo());
    ed.handle(Event::Redo);
    let ab = vec![
        (Op::Replace, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]])),
        (
            Op::Replace,
            poly(&[[10.0, 10.0], [14.0, 10.0], [14.0, 14.0]]),
        ),
    ];
    assert_eq!(ed.selection(), ab.as_slice());
}

#[test]
fn rule_11_commit_clears_redo() {
    let mut ed = Editor::new();
    lasso_a(&mut ed);
    ed.handle(Event::Undo);
    lasso_b(&mut ed);
    let b = vec![(
        Op::Replace,
        poly(&[[10.0, 10.0], [14.0, 10.0], [14.0, 14.0]]),
    )];
    assert_eq!(ed.selection(), b.as_slice());
    assert!(!ed.can_redo());
}

#[test]
fn rule_11_deselect_then_undo_then_redo() {
    let mut ed = Editor::new();
    lasso_a(&mut ed);
    lasso_b(&mut ed);
    ed.handle(Event::Deselect);
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
    ed.handle(Event::Undo);
    let ab = vec![
        (Op::Replace, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]])),
        (
            Op::Replace,
            poly(&[[10.0, 10.0], [14.0, 10.0], [14.0, 14.0]]),
        ),
    ];
    assert_eq!(ed.selection(), ab.as_slice());
    ed.handle(Event::Redo);
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
}

#[test]
fn rule_11_deselect_with_nothing_selected_does_nothing() {
    let mut ed = Editor::new();
    ed.handle(Event::Deselect);
    assert_eq!(ed.selection(), &[] as &[(Op, Shape)]);
    assert!(!ed.can_undo());
}

#[test]
fn rule_1_set_tool_drops_pending() {
    let mut ed = Editor::new();
    ed.handle(press(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    ed.set_tool(Tool::Brush);
    assert_eq!(ed.tool(), Tool::Brush);
    assert_eq!(ed.pending(), None);
}

#[test]
fn rule_12_pending_lasso_reports_op_and_points() {
    let mut ed = Editor::new();
    ed.handle(press_shift(0.0, 0.0));
    ed.handle(drag(4.0, 0.0));
    let want = Some(Item::Select(Op::Add, poly(&[[0.0, 0.0], [4.0, 0.0]])));
    assert_eq!(ed.pending(), want);
}

#[test]
fn rule_12_pending_eraser_reports_stroke() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Eraser);
    ed.handle(press(1.0, 1.0));
    ed.handle(drag(5.0, 1.0)); // 4.0 >= min step 3.0: added
    let want = Some(Item::Erase(Shape::Stroke {
        points: vec![[1.0, 1.0], [5.0, 1.0]],
        radius: 12.0,
    }));
    assert_eq!(ed.pending(), want);
}

#[test]
fn rule_13_selection_mask_matches_build() {
    let mut ed = Editor::new();
    assert_eq!(ed.selection_mask(8, 8), None);
    lasso_a(&mut ed);
    let want = build(
        8,
        8,
        &[(Op::Replace, poly(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]]))],
    );
    assert_eq!(ed.selection_mask(8, 8), want);
    assert_eq!(ed.erase_mask(8, 8), None);
}

#[test]
fn rule_13_erase_mask_pairs_strokes_with_add() {
    let mut ed = Editor::new();
    ed.set_tool(Tool::Eraser);
    ed.handle(press(1.0, 1.0));
    ed.handle(release(2.0, 1.0));
    let want = build(
        8,
        8,
        &[(
            Op::Add,
            Shape::Stroke {
                points: vec![[1.0, 1.0], [2.0, 1.0]],
                radius: 12.0,
            },
        )],
    );
    assert_eq!(ed.erase_mask(8, 8), want);
    assert_eq!(ed.selection_mask(8, 8), None);
}
