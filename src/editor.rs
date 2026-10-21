//! Selection and eraser tools as plain data: lasso, polyline and blob brush select
//! (Shift adds, Ctrl removes); the eraser removes parts of the rendered result.

use crate::select::{build, Mask, Op, Shape};

/// The active tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Lasso,
    Polyline,
    Brush,
    Eraser,
}

/// Modifier keys held at a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
}

/// A pointer or key event, in image coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Press { at: [f32; 2], mods: Mods },
    Drag { at: [f32; 2] },
    Release { at: [f32; 2] },
    Close,
    Back,
    Cancel,
    Undo,
    Redo,
    Deselect,
}

/// A finished (or in-progress) shape and what it does.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Select(Op, Shape),
    Erase(Shape),
}

/// The shape being drawn: its tool, its op, its points.
#[derive(Debug, Clone, PartialEq)]
struct Pending {
    tool: Tool,
    op: Op,
    points: Vec<[f32; 2]>,
}

/// One revertible action (rule 11).
#[derive(Debug, Clone, PartialEq)]
enum Entry {
    Select(Op, Shape),
    Erase(Shape),
    /// A deselect, remembering the selection list it cleared.
    Deselect(Vec<(Op, Shape)>),
}

/// The editor: tools, settings, pending shape, lists and history stacks.
#[derive(Debug, Clone, PartialEq)]
pub struct Editor {
    tool: Tool,
    radius: f32,
    close_radius: f32,
    pending: Option<Pending>,
    selection: Vec<(Op, Shape)>,
    erased: Vec<Shape>,
    done: Vec<Entry>,
    undone: Vec<Entry>,
}

impl Default for Editor {
    fn default() -> Editor {
        Editor::new()
    }
}

/// Rule 3: two points are far when the squared distance is at least `min * min`.
fn far(a: [f32; 2], b: [f32; 2], min: f32) -> bool {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    dx * dx + dy * dy >= min * min
}

/// Rule 2: the op of a press; Ctrl beats Shift. The eraser ignores the keys.
fn key_op(mods: Mods) -> Op {
    if mods.ctrl {
        Op::Subtract
    } else if mods.shift {
        Op::Add
    } else {
        Op::Replace
    }
}

impl Editor {
    /// A fresh editor: lasso, radius 12, close radius 8, everything empty.
    pub fn new() -> Editor {
        Editor {
            tool: Tool::Lasso,
            radius: 12.0,
            close_radius: 8.0,
            pending: None,
            selection: Vec::new(),
            erased: Vec::new(),
            done: Vec::new(),
            undone: Vec::new(),
        }
    }

    /// The current tool.
    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Rule 1: drop the pending shape, then set the tool.
    pub fn set_tool(&mut self, tool: Tool) {
        self.pending = None;
        self.tool = tool;
    }

    /// The brush radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Rule 1: finite radii are clamped to 1.0..=500.0; others are ignored.
    pub fn set_radius(&mut self, radius: f32) {
        if radius.is_finite() {
            self.radius = radius.clamp(1.0, 500.0);
        }
    }

    /// The close radius of the polyline tool.
    pub fn close_radius(&self) -> f32 {
        self.close_radius
    }

    /// Rule 1: finite radii are clamped to 0.0..=100.0; others are ignored.
    pub fn set_close_radius(&mut self, radius: f32) {
        if radius.is_finite() {
            self.close_radius = radius.clamp(0.0, 100.0);
        }
    }

    /// The selection: applied `(op, shape)` items in order.
    pub fn selection(&self) -> &[(Op, Shape)] {
        &self.selection
    }

    /// The erased strokes in order.
    pub fn erased(&self) -> &[Shape] {
        &self.erased
    }

    /// Rule 11: DONE is not empty.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// Rule 11: UNDONE is not empty.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Rule 12: the pending shape as an item, with the radius at this moment.
    pub fn pending(&self) -> Option<Item> {
        self.pending.as_ref().map(|p| item_of(p, self.radius))
    }

    /// Rule 13: the selection rasterized into a `width` x `height` mask.
    pub fn selection_mask(&self, width: u32, height: u32) -> Option<Mask> {
        build(width, height, &self.selection)
    }

    /// Rule 13: every erased stroke paired with `Op::Add`, in order.
    pub fn erase_mask(&self, width: u32, height: u32) -> Option<Mask> {
        let ops: Vec<(Op, Shape)> = self.erased.iter().map(|s| (Op::Add, s.clone())).collect();
        build(width, height, &ops)
    }

    /// Feed one pointer or key event (rules 5-11).
    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Press { at, mods } => self.press(at, mods),
            Event::Drag { at } => self.drag(at),
            Event::Release { at } => self.release(at),
            Event::Close => {
                if self.tool == Tool::Polyline && self.pending.is_some() {
                    self.finish();
                }
            }
            Event::Back => {
                if self.tool == Tool::Polyline {
                    if let Some(p) = self.pending.as_mut() {
                        p.points.pop();
                        if p.points.is_empty() {
                            self.pending = None;
                        }
                    }
                }
            }
            Event::Cancel => {
                self.pending = None;
            }
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            Event::Deselect => self.deselect(),
        }
    }

    /// Rule 5.
    fn press(&mut self, at: [f32; 2], mods: Mods) {
        if self.tool == Tool::Polyline {
            if let Some(p) = self.pending.as_mut() {
                if p.points.len() >= 3 && !far(p.points[0], at, self.close_radius) {
                    self.finish();
                } else {
                    let last = *p.points.last().unwrap_or(&at);
                    if far(last, at, 1.0) {
                        if let Some(p) = self.pending.as_mut() { p.points.push(at) }
                    }
                }
                return;
            }
        }
        self.pending = Some(Pending {
            tool: self.tool,
            op: key_op(mods),
            points: vec![at],
        });
    }

    /// Rule 6.
    fn drag(&mut self, at: [f32; 2]) {
        if self.tool == Tool::Polyline {
            return;
        }
        if let Some(p) = self.pending.as_mut() {
            let min = match p.tool {
                Tool::Lasso | Tool::Polyline => 1.0,
                Tool::Brush | Tool::Eraser => (self.radius / 4.0).max(1.0),
            };
            let last = p.points[p.points.len() - 1];
            if far(last, at, min) {
                p.points.push(at);
            }
        }
    }

    /// Rule 7.
    fn release(&mut self, at: [f32; 2]) {
        if self.tool == Tool::Polyline || self.pending.is_none() {
            return;
        }
        if let Some(p) = self.pending.as_mut() {
            if *p.points.last().unwrap_or(&[f32::NAN, f32::NAN]) != at {
                p.points.push(at);
            }
        }
        self.finish();
    }

    /// Rule 4: commit the pending shape, or drop a short lasso/polyline.
    fn finish(&mut self) {
        let p = match self.pending.take() {
            Some(p) => p,
            None => return,
        };
        match p.tool {
            Tool::Lasso | Tool::Polyline if p.points.len() < 3 => {}
            Tool::Lasso | Tool::Polyline | Tool::Brush => {
                let item = item_of(&p, self.radius);
                if let Item::Select(op, shape) = item {
                    self.selection.push((op, shape.clone()));
                    self.done.push(Entry::Select(op, shape));
                    self.undone.clear();
                }
            }
            Tool::Eraser => {
                let shape = Shape::Stroke {
                    points: p.points.clone(),
                    radius: self.radius,
                };
                self.erased.push(shape.clone());
                self.done.push(Entry::Erase(shape));
                self.undone.clear();
            }
        }
    }

    /// Rule 11.
    fn undo(&mut self) {
        if self.pending.is_some() {
            self.pending = None;
            return;
        }
        let entry = match self.done.pop() {
            Some(e) => e,
            None => return,
        };
        match &entry {
            Entry::Select(_, _) => {
                self.selection.pop();
            }
            Entry::Erase(_) => {
                self.erased.pop();
            }
            Entry::Deselect(saved) => {
                self.selection = saved.clone();
            }
        }
        self.undone.push(entry);
    }

    /// Rule 11.
    fn redo(&mut self) {
        self.pending = None;
        let entry = match self.undone.pop() {
            Some(e) => e,
            None => return,
        };
        match &entry {
            Entry::Select(op, shape) => {
                self.selection.push((*op, shape.clone()));
            }
            Entry::Erase(shape) => {
                self.erased.push(shape.clone());
            }
            Entry::Deselect(_) => {
                self.selection.clear();
            }
        }
        self.done.push(entry);
    }

    /// Rule 11.
    fn deselect(&mut self) {
        self.pending = None;
        if self.selection.is_empty() {
            return;
        }
        let saved = std::mem::take(&mut self.selection);
        self.done.push(Entry::Deselect(saved));
        self.undone.clear();
    }
}

/// Rule 12: the item of a pending shape, with its points in order.
/// `radius` is the editor's radius at that moment.
fn item_of(p: &Pending, radius: f32) -> Item {
    match p.tool {
        Tool::Lasso | Tool::Polyline => Item::Select(p.op, Shape::Polygon(p.points.clone())),
        Tool::Brush => Item::Select(
            p.op,
            Shape::Stroke {
                points: p.points.clone(),
                radius,
            },
        ),
        Tool::Eraser => Item::Erase(Shape::Stroke {
            points: p.points.clone(),
            radius,
        }),
    }
}
