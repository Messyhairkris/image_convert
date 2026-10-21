//! photo_tools editor (feature `gui`): open a photo, choose the areas to keep with a
//! lasso, a polyline or a blob brush (Shift adds, Ctrl removes), render line art, a
//! flat-colour vector or an effect for the selection only, erase parts of the result, and
//! export PNG (transparent outside the selection), JPG, SVG or text.
//!
//! The selection and eraser logic is the library's (`photo_tools::editor`); this file maps
//! egui input to editor events, draws, and reads and writes files.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{
    Color32, ColorImage, Key, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2,
};
use photo_tools::editor::{Editor, Event, Item, Mods, Tool};
use photo_tools::image::{self, DynamicImage, GrayImage, ImageFormat, RgbaImage};
use photo_tools::shapes_file::{self, Selection};
use photo_tools::{
    composite, despeckle, keep_mask, kmeans, lineart_to_svg, process, process_masked, EdgeFilter,
    EffectOptions, Job, LineArtOptions, Mask, Op, Output, Shape, StrokeOptions, Style,
    VectorOptions,
};

/// Textures larger than this on a side are shown downscaled (edits stay full size).
const MAX_TEXTURE: u32 = 8192;

/// Smooth scaling with mipmaps, so a zoomed-out photo is averaged, not sampled.
const SMOOTH: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Linear,
    minification: egui::TextureFilter::Linear,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: Some(egui::TextureFilter::Linear),
};

/// Shrink by `factor`, keeping the darkest opaque pixel of each block (and the block's
/// highest alpha), so one-pixel lines survive a zoomed-out view.
fn shrink_dark(rgba: &RgbaImage, factor: u32) -> RgbaImage {
    let (w, h) = rgba.dimensions();
    let (sw, sh) = (w.div_ceil(factor).max(1), h.div_ceil(factor).max(1));
    RgbaImage::from_fn(sw, sh, |bx, by| {
        let (mut best, mut alpha) = (image::Rgba([255, 255, 255, 0]), 0u8);
        let mut darkest = u32::MAX;
        for y in by * factor..((by + 1) * factor).min(h) {
            for x in bx * factor..((bx + 1) * factor).min(w) {
                let p = rgba.get_pixel(x, y);
                alpha = alpha.max(p[3]);
                let v = p[0] as u32 + p[1] as u32 + p[2] as u32;
                if p[3] > 0 && v < darkest {
                    darkest = v;
                    best = *p;
                }
            }
        }
        best[3] = alpha;
        best
    })
}

/// The share of pixels that are dark (lines), to tell the user when nothing was found.
fn dark_share(g: &GrayImage) -> f64 {
    let n = g.as_raw().iter().filter(|&&v| v < 128).count();
    n as f64 / g.as_raw().len().max(1) as f64
}

fn main() -> eframe::Result {
    let path = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("photo_tools")
            .with_inner_size([1280.0, 860.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "photo_tools",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(&cc.egui_ctx, path)))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    LineArt,
    Vector,
    Effect,
}

/// A finished render, before the selection is applied (except where it changes the work).
enum Render {
    /// Line art and image effects: the whole image; the selection is applied on display.
    Raster(GrayImage),
    /// Vector mode: a raster preview of the flat-colour result (outside transparent).
    Vector(RgbaImage),
    /// ASCII.
    Text(String),
}

enum Done {
    Render {
        key: String,
        result: Result<Render, String>,
    },
    Export(Result<PathBuf, String>),
}

struct Work {
    what: &'static str,
    started: Instant,
    rx: Receiver<Done>,
}

struct Photo {
    path: PathBuf,
    img: Arc<DynamicImage>,
    tex: TextureHandle,
}

struct View {
    zoom: f32,
    offset: Vec2,
    fit_next: bool,
}

struct App {
    photo: Option<Photo>,
    editor: Editor,
    /// A selection loaded from a file; the editor's own operations come after it.
    loaded: Selection,
    seen: Option<(Vec<(Op, Shape)>, Vec<Shape>)>,
    keep: Option<Mask>,
    keep_version: u64,
    overlay: Option<TextureHandle>,
    mode: Mode,
    lineart: LineArtOptions,
    /// Canny's low threshold as a share of the high one.
    continuation: f32,
    vector: VectorOptions,
    effect: EffectOptions,
    stroke: StrokeOptions,
    live: bool,
    render: Option<(String, Render)>,
    result_tex: Option<TextureHandle>,
    /// The result as pixels (true = line art, shown dark-preserving when zoomed out).
    result_rgba: Option<(bool, RgbaImage)>,
    /// The downscale factor the result texture was built for (0 = rebuild).
    result_factor: u32,
    result_dirty: bool,
    /// Settings whose render failed; not retried until something changes.
    failed_key: Option<String>,
    /// The last render found no lines at all (shown over the picture).
    empty_result: bool,
    rendering: Option<Work>,
    exporting: Option<Work>,
    changed_at: Instant,
    show_result: bool,
    view: View,
    drawing: bool,
    checker: TextureHandle,
    status: String,
    error: Option<String>,
}

impl App {
    fn new(ctx: &egui::Context, path: Option<PathBuf>) -> App {
        let checker = ctx.load_texture(
            "checker",
            ColorImage::from_rgba_unmultiplied(
                [2, 2],
                &[
                    204, 204, 204, 255, 255, 255, 255, 255, 255, 255, 255, 255, 204, 204, 204, 255,
                ],
            ),
            TextureOptions::NEAREST_REPEAT,
        );
        let mut app = App {
            photo: None,
            editor: Editor::new(),
            loaded: Selection::default(),
            seen: None,
            keep: None,
            keep_version: 0,
            overlay: None,
            mode: Mode::LineArt,
            lineart: LineArtOptions::default(),
            continuation: 0.5,
            vector: VectorOptions::default(),
            effect: EffectOptions::default(),
            stroke: StrokeOptions::default(),
            live: true,
            render: None,
            result_tex: None,
            result_rgba: None,
            result_factor: 0,
            result_dirty: false,
            failed_key: None,
            empty_result: false,
            rendering: None,
            exporting: None,
            changed_at: Instant::now(),
            show_result: false,
            view: View {
                zoom: 1.0,
                offset: Vec2::ZERO,
                fit_next: true,
            },
            drawing: false,
            checker,
            status: "Open a photo (or drop one on the window) to start.".into(),
            error: None,
        };
        if let Some(p) = path {
            app.open(ctx, &p);
        }
        app
    }

    /// A fresh editor that keeps the chosen tool and brush size.
    fn reset_editor(&mut self) {
        let (tool, radius) = (self.editor.tool(), self.editor.radius());
        self.editor = Editor::new();
        self.editor.set_tool(tool);
        self.editor.set_radius(radius);
    }

    // -- files -------------------------------------------------------------------------

    fn open(&mut self, ctx: &egui::Context, path: &Path) {
        match image::open(path) {
            Ok(img) => {
                let tex = ctx.load_texture(
                    "photo",
                    texture_image(&img.to_rgba8()),
                    SMOOTH,
                );
                self.reset_editor();
                self.loaded = Selection::default();
                self.seen = None;
                self.render = None;
                self.result_tex = None;
                self.result_rgba = None;
                self.failed_key = None;
                self.show_result = false;
                self.view.fit_next = true;
                self.status = format!(
                    "Opened {} ({} x {}).",
                    path.display(),
                    img.width(),
                    img.height()
                );
                self.error = None;
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "photo_tools - {name}"
                )));
                self.photo = Some(Photo {
                    path: path.to_path_buf(),
                    img: Arc::new(img),
                    tex,
                });
                self.changed_at = Instant::now();
            }
            Err(e) => {
                self.error = Some(format!(
                    "Could not open {}: {e} (PNG, JPEG, WebP, BMP, TIFF and GIF are supported)",
                    path.display()
                ))
            }
        }
    }

    fn all_ops(&self) -> Vec<(Op, Shape)> {
        self.loaded
            .ops
            .iter()
            .chain(self.editor.selection())
            .cloned()
            .collect()
    }

    fn all_erased(&self) -> Vec<Shape> {
        self.loaded
            .erased
            .iter()
            .chain(self.editor.erased())
            .cloned()
            .collect()
    }

    fn stem(&self) -> String {
        self.photo
            .as_ref()
            .and_then(|p| p.path.file_stem())
            .map_or_else(|| "photo".into(), |s| s.to_string_lossy().into_owned())
    }

    fn save_selection(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Selection", &["txt"])
            .set_file_name(self.stem() + "_selection.txt")
            .save_file()
        else {
            return;
        };
        let text = shapes_file::write(&Selection {
            ops: self.all_ops(),
            erased: self.all_erased(),
        });
        match write_atomically(&path, text.as_bytes()) {
            Ok(()) => self.status = format!("Saved the selection to {}.", path.display()),
            Err(e) => self.error = Some(e),
        }
    }

    fn load_selection(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Selection", &["txt"])
            .pick_file()
        else {
            return;
        };
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| shapes_file::parse(&t).map_err(|e| e.to_string()));
        match parsed {
            Ok(sel) => {
                self.reset_editor();
                self.loaded = sel;
                self.status = format!("Loaded the selection from {}.", path.display());
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Could not load {}: {e}", path.display())),
        }
    }

    fn export(&mut self) {
        let Some(photo) = &self.photo else { return };
        let (filters, default): (Vec<(&str, &[&str])>, &str) = match (self.mode, self.effect.style)
        {
            (Mode::LineArt, _) => (
                vec![
                    ("PNG (transparent outside the selection)", &["png"]),
                    ("SVG centerlines", &["svg"]),
                    ("JPEG", &["jpg", "jpeg"]),
                ],
                "lineart.png",
            ),
            (Mode::Vector, _) => (vec![("SVG", &["svg"])], "vector.svg"),
            (Mode::Effect, Style::Ascii) => (vec![("Text", &["txt"])], "ascii.txt"),
            (Mode::Effect, _) => (
                vec![("PNG", &["png"]), ("JPEG", &["jpg", "jpeg"])],
                "effect.png",
            ),
        };
        let mut dialog = rfd::FileDialog::new().set_file_name(format!("{}_{default}", self.stem()));
        if let Some(dir) = photo.path.parent() {
            dialog = dialog.set_directory(dir);
        }
        for (name, ext) in &filters {
            dialog = dialog.add_filter(*name, ext);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let (img, job, stroke, keep) = (
            photo.img.clone(),
            self.job(),
            self.stroke,
            self.keep.clone(),
        );
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(Done::Export(export(
                &img,
                &job,
                &stroke,
                keep.as_ref(),
                &path,
            )));
        });
        self.exporting = Some(Work {
            what: "Exporting",
            started: Instant::now(),
            rx,
        });
    }

    // -- rendering -----------------------------------------------------------------------

    fn job(&self) -> Job {
        match self.mode {
            Mode::LineArt => Job::LineArt(self.lineart),
            Mode::Vector => Job::Vector(self.vector),
            Mode::Effect => Job::Effect(EffectOptions {
                ansi: false,
                ..self.effect
            }),
        }
    }

    /// What a render depends on. Raster jobs are rendered for the whole image and the
    /// selection is applied on display, so the selection only matters to vector and ASCII.
    fn render_key(&self) -> String {
        let job = self.job();
        let uses_keep = matches!(job, Job::Vector(_))
            || matches!(job, Job::Effect(o) if o.style == Style::Ascii);
        format!("{job:?}|{}", if uses_keep { self.keep_version } else { 0 })
    }

    fn start_render(&mut self) {
        let Some(photo) = &self.photo else { return };
        let (img, job, keep, key) = (
            photo.img.clone(),
            self.job(),
            self.keep.clone(),
            self.render_key(),
        );
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let result = render(&img, &job, keep.as_ref());
            let _ = tx.send(Done::Render { key, result });
        });
        self.rendering = Some(Work {
            what: "Rendering",
            started: Instant::now(),
            rx,
        });
    }

    fn poll_work(&mut self, ctx: &egui::Context) {
        if let Some(w) = &self.rendering {
            if let Ok(Done::Render { key, result }) = w.rx.try_recv() {
                let took = w.started.elapsed();
                self.rendering = None;
                match result {
                    Ok(r) => {
                        self.empty_result = matches!(&r, Render::Raster(g) if dark_share(g) < 0.0002);
                        self.status = match &r {
                            Render::Raster(g) if dark_share(g) < 0.0002 => format!(
                                "Rendered in {:.1} s - but nothing was found at these settings. \
                                 Press Auto, or lower the threshold.",
                                took.as_secs_f32()
                            ),
                            _ => format!("Rendered in {:.1} s.", took.as_secs_f32()),
                        };
                        self.render = Some((key, r));
                        self.failed_key = None;
                        self.result_dirty = true;
                        self.error = None;
                    }
                    Err(e) => {
                        // Keep the last good result on screen, and do not retry these
                        // same settings in a loop.
                        self.failed_key = Some(key);
                        self.error = Some(e);
                    }
                }
            }
        }
        if let Some(w) = &self.exporting {
            if let Ok(Done::Export(result)) = w.rx.try_recv() {
                self.exporting = None;
                match result {
                    Ok(p) => {
                        self.status = format!("Exported {}.", p.display());
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
        }
        let want = self.render_key();
        let current = self.render.as_ref().map(|(k, _)| k.as_str());
        let quiet = self.changed_at.elapsed() > Duration::from_millis(250);
        if self.live
            && self.photo.is_some()
            && self.rendering.is_none()
            && current != Some(want.as_str())
            && self.failed_key.as_deref() != Some(want.as_str())
            && quiet
        {
            self.start_render();
        }
        if self.rendering.is_some() || self.exporting.is_some() || !quiet {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Recompute the keep mask and the overlay when the selection or erasing changed.
    fn refresh_selection(&mut self, ctx: &egui::Context) {
        let now = (self.all_ops(), self.all_erased());
        if self.seen.as_ref() == Some(&now) {
            return;
        }
        let Some(photo) = &self.photo else { return };
        let (w, h) = (photo.img.width(), photo.img.height());
        self.keep = keep_mask(w, h, &now.0, &now.1);
        self.keep_version += 1;
        self.overlay = self.keep.as_ref().map(|m| {
            let mut rgba = RgbaImage::new(w, h);
            for (p, &k) in rgba.pixels_mut().zip(m.bits()) {
                *p = if k {
                    image::Rgba([0, 0, 0, 0])
                } else {
                    image::Rgba([0, 0, 0, 150])
                };
            }
            ctx.load_texture("overlay", texture_image(&rgba), SMOOTH)
        });
        self.seen = Some(now);
        self.result_dirty = true;
        self.changed_at = Instant::now();
    }

    /// Rebuild the result picture when the render or the selection changed, and its
    /// on-screen copy when the zoom changed. Line art is 1 pixel wide: a plain
    /// downscale skips most of those pixels and the result looks white, so a zoomed-out
    /// view keeps the darkest pixel of each block instead.
    fn refresh_result(&mut self, ctx: &egui::Context) {
        if self.result_dirty {
            self.result_dirty = false;
            self.result_rgba = match &self.render {
                Some((_, Render::Raster(g))) => {
                    let la = match &self.keep {
                        Some(m) => {
                            composite::keep(g, m).unwrap_or_else(|_| composite::opaque(g))
                        }
                        None => composite::opaque(g),
                    };
                    Some((
                        true,
                        RgbaImage::from_fn(la.width(), la.height(), |x, y| {
                            let p = la.get_pixel(x, y);
                            image::Rgba([p[0], p[0], p[0], p[1]])
                        }),
                    ))
                }
                Some((_, Render::Vector(rgba))) => Some((false, rgba.clone())),
                _ => None,
            };
            self.result_factor = 0;
        }
        let Some((lines, rgba)) = &self.result_rgba else {
            self.result_tex = None;
            return;
        };
        let factor = if *lines { (1.0 / self.view.zoom).floor().clamp(1.0, 64.0) as u32 } else { 1 };
        if factor != self.result_factor {
            self.result_factor = factor;
            let shown = if factor > 1 { shrink_dark(rgba, factor) } else { rgba.clone() };
            self.result_tex = Some(ctx.load_texture("result", texture_image(&shown), SMOOTH));
        }
    }

    // -- canvas ---------------------------------------------------------------------------

    fn to_screen(&self, canvas: Rect, p: [f32; 2]) -> Pos2 {
        canvas.min + self.view.offset + Vec2::new(p[0], p[1]) * self.view.zoom
    }

    fn to_image(&self, canvas: Rect, s: Pos2) -> [f32; 2] {
        let v = (s - canvas.min - self.view.offset) / self.view.zoom;
        [v.x, v.y]
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas = resp.rect;
        painter.rect_filled(canvas, 0.0, Color32::from_gray(60));
        let Some(size) = self
            .photo
            .as_ref()
            .map(|p| Vec2::new(p.img.width() as f32, p.img.height() as f32))
        else {
            painter.text(
                canvas.center(),
                egui::Align2::CENTER_CENTER,
                "Open a photo, or drop one here",
                egui::FontId::proportional(20.0),
                Color32::from_gray(200),
            );
            return;
        };
        if self.view.fit_next && canvas.width() > 0.0 {
            let zoom = (canvas.width() / size.x).min(canvas.height() / size.y) * 0.96;
            self.view = View {
                zoom,
                offset: (canvas.size() - size * zoom) / 2.0,
                fit_next: false,
            };
        }

        // Input first, so this frame draws its result.
        let (mods, pressed, released, pan, pos, delta, double, scroll, zoom_delta, space) = ui
            .input(|i| {
                (
                    i.modifiers,
                    i.pointer.primary_pressed(),
                    i.pointer.primary_released(),
                    i.pointer.button_down(egui::PointerButton::Middle)
                        || (i.key_down(Key::Space) && i.pointer.primary_down()),
                    i.pointer.latest_pos(),
                    i.pointer.delta(),
                    i.pointer
                        .button_double_clicked(egui::PointerButton::Primary),
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.key_down(Key::Space),
                )
            });
        if resp.hovered() {
            let factor = zoom_delta * (scroll * 0.0015).exp();
            if factor != 1.0 {
                if let Some(p) = pos {
                    let at = self.to_image(canvas, p);
                    self.view.zoom = (self.view.zoom * factor).clamp(0.02, 40.0);
                    self.view.offset = p - canvas.min - Vec2::new(at[0], at[1]) * self.view.zoom;
                }
            }
        }
        if pan && (resp.hovered() || resp.dragged()) {
            self.view.offset += delta;
        }
        self.editor.set_close_radius(8.0 / self.view.zoom);
        let mods = Mods {
            shift: mods.shift,
            ctrl: mods.ctrl || mods.command,
        };
        if let Some(p) = pos {
            let at = self.to_image(canvas, p);
            if pressed && resp.hovered() && !space {
                self.editor.handle(Event::Press { at, mods });
                self.drawing = true;
            } else if self.drawing && delta != Vec2::ZERO && !released {
                self.editor.handle(Event::Drag { at });
            }
            if released && self.drawing {
                self.editor.handle(Event::Release { at });
                self.drawing = false;
            }
            if double && resp.hovered() {
                self.editor.handle(Event::Close);
            }
        }

        // Pictures.
        let Some(photo) = &self.photo else { return };
        let photo_rect =
            Rect::from_min_size(self.to_screen(canvas, [0.0, 0.0]), size * self.view.zoom);
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let painter = painter.with_clip_rect(canvas);
        if self.show_result {
            let tiles = photo_rect.size() / 16.0;
            painter.image(
                self.checker.id(),
                photo_rect,
                Rect::from_min_max(Pos2::ZERO, tiles.to_pos2()),
                Color32::WHITE,
            );
            if let Some(t) = &self.result_tex {
                painter.image(t.id(), photo_rect, full_uv, Color32::WHITE);
            }
            if self.empty_result {
                painter.text(
                    canvas.center(),
                    egui::Align2::CENTER_CENTER,
                    "Nothing found at these settings.\nPress Auto, or lower the line strength / threshold.",
                    egui::FontId::proportional(20.0),
                    Color32::from_rgb(200, 60, 40),
                );
            }
        } else {
            painter.image(photo.tex.id(), photo_rect, full_uv, Color32::WHITE);
            if let Some(t) = &self.overlay {
                painter.image(t.id(), photo_rect, full_uv, Color32::WHITE);
            }
        }

        // The shape being drawn, and the brush cursor.
        let colour = |op: Option<Op>| match op {
            Some(Op::Replace) => Color32::WHITE,
            Some(Op::Add) => Color32::from_rgb(120, 230, 120),
            Some(Op::Subtract) => Color32::from_rgb(255, 120, 90),
            None => Color32::from_rgb(230, 120, 255),
        };
        match self.editor.pending() {
            Some(Item::Select(op, Shape::Polygon(pts))) => {
                let mut line: Vec<Pos2> = pts.iter().map(|&p| self.to_screen(canvas, p)).collect();
                if self.editor.tool() == Tool::Polyline {
                    if let Some(&first) = line.first() {
                        painter.circle_stroke(first, 8.0, Stroke::new(1.5, colour(Some(op))));
                    }
                    if let Some(p) = pos {
                        line.push(p);
                    }
                }
                painter.add(egui::Shape::line(
                    line.clone(),
                    Stroke::new(3.0, Color32::BLACK),
                ));
                painter.add(egui::Shape::line(line, Stroke::new(1.5, colour(Some(op)))));
            }
            Some(Item::Select(op, Shape::Stroke { points, radius })) => {
                self.paint_stroke(&painter, canvas, &points, radius, colour(Some(op)));
            }
            Some(Item::Erase(Shape::Stroke { points, radius })) => {
                self.paint_stroke(&painter, canvas, &points, radius, colour(None));
            }
            _ => {}
        }
        if matches!(self.editor.tool(), Tool::Brush | Tool::Eraser) && resp.hovered() {
            if let Some(p) = pos {
                let r = self.editor.radius() * self.view.zoom;
                painter.circle_stroke(p, r, Stroke::new(2.5, Color32::BLACK));
                painter.circle_stroke(p, r, Stroke::new(1.0, Color32::WHITE));
            }
        }
    }

    fn paint_stroke(
        &self,
        painter: &egui::Painter,
        canvas: Rect,
        points: &[[f32; 2]],
        radius: f32,
        c: Color32,
    ) {
        let fill = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 90);
        let r = radius * self.view.zoom;
        let pts: Vec<Pos2> = points.iter().map(|&p| self.to_screen(canvas, p)).collect();
        if pts.len() > 1 {
            painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.0 * r, fill)));
        }
        for p in pts.iter().step_by(4).chain(pts.last()) {
            painter.circle_filled(*p, r, fill);
        }
    }

    // -- panels ---------------------------------------------------------------------------

    fn keys(&mut self, ui: &egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let (enter, esc, back, undo, redo, deselect, l, p, b, e, smaller, bigger, fit, tab) = ui
            .input(|i| {
                let cmd = i.modifiers.command;
                (
                    i.key_pressed(Key::Enter),
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::Backspace),
                    cmd && i.key_pressed(Key::Z) && !i.modifiers.shift,
                    cmd && (i.key_pressed(Key::Y) || (i.key_pressed(Key::Z) && i.modifiers.shift)),
                    cmd && i.key_pressed(Key::D),
                    !cmd && i.key_pressed(Key::L),
                    !cmd && i.key_pressed(Key::P),
                    !cmd && i.key_pressed(Key::B),
                    !cmd && i.key_pressed(Key::E),
                    i.key_pressed(Key::OpenBracket),
                    i.key_pressed(Key::CloseBracket),
                    !cmd && i.key_pressed(Key::F),
                    !cmd && i.key_pressed(Key::V),
                )
            });
        for (pressed, event) in [
            (enter, Event::Close),
            (esc, Event::Cancel),
            (back, Event::Back),
            (undo, Event::Undo),
            (redo, Event::Redo),
        ] {
            if pressed {
                self.editor.handle(event);
            }
        }
        if deselect {
            self.select_all();
        }
        for (pressed, tool) in [
            (l, Tool::Lasso),
            (p, Tool::Polyline),
            (b, Tool::Brush),
            (e, Tool::Eraser),
        ] {
            if pressed {
                self.editor.set_tool(tool);
            }
        }
        if smaller {
            self.editor.set_radius(self.editor.radius() / 1.25);
        }
        if bigger {
            self.editor.set_radius(self.editor.radius() * 1.25);
        }
        if fit {
            self.view.fit_next = true;
        }
        if tab {
            self.show_result = !self.show_result;
        }
    }

    /// Drop the selection (the whole photo is used again); erasing is kept.
    fn select_all(&mut self) {
        self.editor.handle(Event::Deselect);
        self.loaded.ops.clear();
    }

    /// Settings chosen from the photo (inside the selection, if there is one).
    /// Line art: the edge strengths of the smoothed photo are measured and the line
    /// threshold put where about the strongest tenth of them are; the photo's size sets
    /// the smoothing. Vector and effects scale their sizes with the photo.
    fn auto_settings(&mut self) {
        let Some(photo) = &self.photo else { return };
        let img = &photo.img;
        let (w, h) = (img.width(), img.height());
        let long = w.max(h) as f32;
        match self.mode {
            Mode::LineArt => {
                let blur = (long / 1000.0).clamp(0.8, 3.0);
                let gray = GrayImage::from_raw(
                    w,
                    h,
                    photo_tools::luma::rgb_to_gray(img.to_rgb8().as_raw()),
                )
                .expect("one gray byte per pixel");
                let smooth = imageproc::filter::gaussian_blur_f32(&gray, blur);
                let mag = photo_tools::sobel::magnitude(smooth.as_raw(), w as usize, h as usize);
                let mut strong: Vec<u16> = mag
                    .iter()
                    .enumerate()
                    .filter(|&(i, &m)| m > 8 && self.keep.as_ref().is_none_or(|k| k.bits()[i]))
                    .map(|(_, &m)| m)
                    .collect();
                if strong.is_empty() {
                    self.status = "Auto: this area has no edges to find.".into();
                    return;
                }
                strong.sort_unstable();
                let at = |q: f32| strong[((strong.len() - 1) as f32 * q) as usize] as f32;
                self.lineart.blur = (blur * 10.0).round() / 10.0;
                match self.lineart.filter {
                    EdgeFilter::Canny => {
                        self.lineart.high = at(0.90).round().max(5.0);
                        self.continuation = 0.4;
                        self.lineart.low = (0.4 * self.lineart.high).round();
                    }
                    EdgeFilter::Sobel => {
                        self.lineart.low = at(0.93).round().max(10.0);
                        self.lineart.high = self.lineart.low;
                    }
                }
            }
            Mode::Vector => {
                self.vector.colors = 8;
                self.vector.smoothness = 1.0;
                self.vector.speckle = ((w as f32 * h as f32) / 4000.0).clamp(8.0, 5000.0) as u32;
                self.vector.max_iterations = 20;
            }
            Mode::Effect => {
                self.effect.blur = (long / 200.0).clamp(4.0, 60.0);
                self.effect.cell = ((long / 300.0) as u32).clamp(4, 32);
                self.effect.columns = 120;
            }
        }
        self.changed_at = Instant::now();
        self.show_result = true;
        self.status = "Auto settings chosen for this photo.".into();
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open...").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif"],
                    )
                    .pick_file()
                {
                    let ctx = ui.ctx().clone();
                    self.open(&ctx, &p);
                }
            }
            let has_photo = self.photo.is_some();
            if ui
                .add_enabled(
                    has_photo && self.exporting.is_none(),
                    egui::Button::new("Export..."),
                )
                .clicked()
            {
                self.export();
            }
            ui.separator();
            if ui
                .add_enabled(has_photo, egui::Button::new("Save selection..."))
                .clicked()
            {
                self.save_selection();
            }
            if ui
                .add_enabled(has_photo, egui::Button::new("Load selection..."))
                .clicked()
            {
                self.load_selection();
            }
            if ui
                .add_enabled(has_photo, egui::Button::new("Whole photo (Ctrl+D)"))
                .clicked()
            {
                self.select_all();
            }
            ui.separator();
            ui.selectable_value(&mut self.show_result, false, "Photo");
            ui.selectable_value(&mut self.show_result, true, "Result");
            ui.label("(V)");
            ui.separator();
            if ui.button("Fit (F)").clicked() {
                self.view.fit_next = true;
            }
            ui.label(format!("{:.0}%", self.view.zoom * 100.0));
        });
        ui.horizontal_wrapped(|ui| {
            let tool = self.editor.tool();
            for (t, name) in [
                (Tool::Lasso, "Lasso (L)"),
                (Tool::Polyline, "Polyline (P)"),
                (Tool::Brush, "Blob brush (B)"),
                (Tool::Eraser, "Eraser (E)"),
            ] {
                if ui.selectable_label(tool == t, name).clicked() {
                    self.editor.set_tool(t);
                }
            }
            ui.separator();
            let mut r = self.editor.radius();
            if ui
                .add(
                    egui::Slider::new(&mut r, 1.0..=500.0)
                        .logarithmic(true)
                        .text("brush radius ([ ])"),
                )
                .changed()
            {
                self.editor.set_radius(r);
            }
            ui.separator();
            if ui
                .add_enabled(self.editor.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.editor.handle(Event::Undo);
            }
            if ui
                .add_enabled(self.editor.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.editor.handle(Event::Redo);
            }
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        let before = (
            self.mode,
            format!("{:?}{:?}{:?}", self.lineart, self.vector, self.effect),
        );
        ui.heading("Mode");
        egui::ComboBox::from_id_salt("mode")
            .selected_text(match self.mode {
                Mode::LineArt => "Line art",
                Mode::Vector => "Vector (flat colour)",
                Mode::Effect => "Effect",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.mode, Mode::LineArt, "Line art");
                ui.selectable_value(&mut self.mode, Mode::Vector, "Vector (flat colour)");
                ui.selectable_value(&mut self.mode, Mode::Effect, "Effect");
            });
        ui.add_space(6.0);
        // Long sliders: a short one moved hundreds of units per pixel and was unusable.
        ui.spacing_mut().slider_width = 190.0;
        ui.horizontal(|ui| {
            let can = self.photo.is_some();
            if ui
                .add_enabled(can, egui::Button::new("Auto"))
                .on_hover_text("Choose settings for this photo")
                .clicked()
            {
                self.auto_settings();
            }
            if ui
                .add(egui::Button::new("Reset"))
                .on_hover_text("Back to the default settings for this mode")
                .clicked()
            {
                match self.mode {
                    Mode::LineArt => {
                        self.lineart = LineArtOptions::default();
                        self.continuation = 0.5;
                        self.stroke = StrokeOptions::default();
                    }
                    Mode::Vector => self.vector = VectorOptions::default(),
                    Mode::Effect => {
                        self.effect = EffectOptions { style: self.effect.style, ..EffectOptions::default() }
                    }
                }
            }
        });
        ui.add_space(4.0);
        match self.mode {
            Mode::LineArt => {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.lineart.filter, EdgeFilter::Canny, "Canny");
                    ui.selectable_value(&mut self.lineart.filter, EdgeFilter::Sobel, "Sobel");
                });
                ui.add(
                    egui::Slider::new(&mut self.lineart.blur, 0.0..=6.0)
                        .step_by(0.1)
                        .text("smooth first"),
                )
                .on_hover_text("Blurs the photo before finding lines: higher = fewer, cleaner lines");
                if self.lineart.filter == EdgeFilter::Canny {
                    ui.add(
                        egui::Slider::new(&mut self.lineart.high, 5.0..=400.0)
                            .logarithmic(true)
                            .step_by(1.0)
                            .text("line strength"),
                    )
                    .on_hover_text("Only edges this strong start a line: higher = fewer lines");
                    // Weak edges continue a line when they are at least this share of the
                    // line strength (Canny's low threshold), kept as a share so moving the
                    // strength never leaves low above high.
                    ui.add(
                        egui::Slider::new(&mut self.continuation, 0.1..=1.0)
                            .step_by(0.05)
                            .text("line continuation"),
                    )
                    .on_hover_text("Lower = lines run on further through faint edges");
                    self.lineart.low = (self.continuation * self.lineart.high).round();
                } else {
                    ui.add(
                        egui::Slider::new(&mut self.lineart.low, 10.0..=600.0)
                            .logarithmic(true)
                            .step_by(1.0)
                            .text("threshold"),
                    )
                    .on_hover_text("Edges this strong become lines: higher = fewer lines");
                    self.lineart.high = self.lineart.low;
                }
                ui.add_space(6.0);
                ui.label("SVG export");
                ui.add(egui::Slider::new(&mut self.stroke.tolerance, 0.0..=4.0).step_by(0.1).text("smoothing"));
                ui.add(egui::Slider::new(&mut self.stroke.width, 0.25..=10.0).logarithmic(true).text("stroke width"));
                ui.add(
                    egui::Slider::new(&mut self.stroke.min_pixels, 0..=200)
                        .logarithmic(true)
                        .text("drop specks under"),
                );
            }
            Mode::Vector => {
                ui.add(egui::Slider::new(&mut self.vector.colors, 2..=32).text("colours"));
                ui.add(
                    egui::Slider::new(&mut self.vector.smoothness, 0.0..=4.0)
                        .step_by(0.1)
                        .text("smoothness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.vector.speckle, 0..=5000)
                        .logarithmic(true)
                        .text("merge specks under"),
                );
                ui.add(
                    egui::Slider::new(&mut self.vector.max_iterations, 1..=100).text("iterations"),
                );
            }
            Mode::Effect => {
                ui.horizontal_wrapped(|ui| {
                    for (s, name) in [
                        (Style::Sketch, "Sketch"),
                        (Style::Dither, "Dither"),
                        (Style::Halftone, "Halftone"),
                        (Style::Hatch, "Hatch"),
                        (Style::Ascii, "ASCII"),
                    ] {
                        ui.selectable_value(&mut self.effect.style, s, name);
                    }
                });
                match self.effect.style {
                    Style::Sketch => {
                        ui.add(
                            egui::Slider::new(&mut self.effect.blur, 0.5..=100.0)
                                .logarithmic(true)
                                .text("blur"),
                        );
                    }
                    Style::Halftone | Style::Hatch => {
                        ui.add(egui::Slider::new(&mut self.effect.cell, 2..=64).text("cell"));
                    }
                    Style::Ascii => {
                        ui.add(
                            egui::Slider::new(&mut self.effect.columns, 10..=400).text("columns"),
                        );
                        ui.checkbox(&mut self.effect.invert, "invert");
                    }
                    Style::Dither => {}
                }
            }
        }
        ui.add_space(10.0);
        ui.checkbox(&mut self.live, "Render as I change things");
        if ui
            .add_enabled(
                self.photo.is_some() && self.rendering.is_none(),
                egui::Button::new("Render now"),
            )
            .clicked()
        {
            self.start_render();
        }
        let after = (
            self.mode,
            format!("{:?}{:?}{:?}", self.lineart, self.vector, self.effect),
        );
        if before != after {
            self.changed_at = Instant::now();
            self.show_result = true;
        }
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new("Selecting").strong());
        ui.label("No key: replace. Shift: add. Ctrl: remove.\nNothing selected = the whole photo.");
        ui.label("Polyline: click points; Enter, a double-click or a click on the first point closes it. Backspace removes a point, Esc cancels.");
        ui.label("Eraser: paint over the result to remove it.");
        ui.label("Wheel: zoom. Middle drag or Space+drag: pan. Ctrl+Z / Ctrl+Y: undo / redo.");
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for w in [&self.rendering, &self.exporting].into_iter().flatten() {
                ui.spinner();
                ui.label(format!(
                    "{}... {:.1} s",
                    w.what,
                    w.started.elapsed().as_secs_f32()
                ));
                ui.separator();
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::from_rgb(255, 110, 90), e);
            } else {
                ui.label(&self.status);
            }
            if let Some(p) = &self.photo {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let sel = match &self.keep {
                        Some(m) => format!(
                            "{:.1}% kept",
                            100.0 * m.count() as f64 / m.bits().len().max(1) as f64
                        ),
                        None => "whole photo".into(),
                    };
                    ui.label(format!("{} x {} | {sel}", p.img.width(), p.img.height()));
                });
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(path) =
            ui.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.open(&ctx, &path);
        }
        self.keys(ui);
        self.poll_work(&ctx);
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("options")
            .resizable(false)
            .show(ui, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            let text = match (&self.render, self.show_result) {
                (Some((_, Render::Text(t))), true) if !t.is_empty() => Some(t.clone()),
                _ => None,
            };
            if let Some(t) = text {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.label(egui::RichText::new(t).monospace().size(8.0));
                });
            } else {
                self.canvas(ui);
            }
            // Something is being dragged over the window: say so, so a drop is expected.
            if ui.input(|i| !i.raw.hovered_files.is_empty()) {
                let r = ui.max_rect();
                ui.painter()
                    .rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(30, 80, 160, 140));
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop to open",
                    egui::FontId::proportional(28.0),
                    Color32::WHITE,
                );
            }
        });
        self.refresh_selection(&ctx);
        self.refresh_result(&ctx);
    }
}

// -- work done off the UI thread -----------------------------------------------------------

fn render(img: &DynamicImage, job: &Job, keep: Option<&Mask>) -> Result<Render, String> {
    match job {
        Job::Vector(o) => vector_preview(img, o, keep).map(Render::Vector),
        Job::Effect(o) if o.style == Style::Ascii => match process_masked(img, job, keep) {
            Ok(Output::Text(t)) => Ok(Render::Text(t)),
            Ok(_) => Err("ASCII art came back as an image".into()),
            Err(e) => Err(format!("{e}")),
        },
        _ => match process(img, job) {
            Ok(Output::Image(g)) => Ok(Render::Raster(g)),
            Ok(_) => Err("the render came back in an unexpected form".into()),
            Err(e) => Err(format!("{e}")),
        },
    }
}

/// The flat-colour picture vector mode draws, as pixels: the same palette, labels and
/// despeckling as `vector_select`, without tracing outlines.
fn vector_preview(
    img: &DynamicImage,
    o: &VectorOptions,
    keep: Option<&Mask>,
) -> Result<RgbaImage, String> {
    o.validate().map_err(|e| format!("{e}"))?;
    let (w, h) = (img.width(), img.height());
    let rgb = img.to_rgb8();
    let all = vec![true; (w * h) as usize];
    let kept: &[bool] = keep.map_or(&all, |m| m.bits());
    let chosen: Vec<u8> = rgb
        .as_raw()
        .chunks_exact(3)
        .zip(kept)
        .filter(|(_, &k)| k)
        .flat_map(|(p, _)| p.iter().copied())
        .collect();
    if chosen.is_empty() {
        return Ok(RgbaImage::new(w, h));
    }
    let palette = kmeans::quantize(&chosen, o.colors as usize, o.max_iterations as usize);
    let outside = palette.colors.len() as u8;
    let mut next = palette.labels.iter();
    let labels: Vec<u8> = kept
        .iter()
        .map(|&k| if k { *next.next().unwrap() } else { outside })
        .collect();
    let labels = despeckle::despeckle(&labels, w as usize, h as usize, o.speckle as usize);
    Ok(RgbaImage::from_fn(w, h, |x, y| {
        let i = (y * w + x) as usize;
        match palette.colors.get(labels[i] as usize) {
            Some(c) if kept[i] => image::Rgba([c[0], c[1], c[2], 255]),
            _ => image::Rgba([0, 0, 0, 0]),
        }
    }))
}

fn export(
    img: &DynamicImage,
    job: &Job,
    stroke: &StrokeOptions,
    keep: Option<&Mask>,
    path: &Path,
) -> Result<PathBuf, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let fail = |e: photo_tools::Error| format!("{e}");
    let bytes: Vec<u8> = match (job, ext.as_str()) {
        (Job::LineArt(o), "svg") => lineart_to_svg(img, o, stroke, keep)
            .map_err(fail)?
            .into_bytes(),
        (_, "svg" | "txt") => match process_masked(img, job, keep).map_err(fail)? {
            Output::Svg(s) | Output::Text(s) => s.into_bytes(),
            _ => return Err(format!("this mode does not make .{ext} files")),
        },
        (_, "png" | "jpg" | "jpeg") => {
            let format = if ext == "png" {
                ImageFormat::Png
            } else {
                ImageFormat::Jpeg
            };
            let out = match process_masked(img, job, keep).map_err(fail)? {
                Output::Image(g) => DynamicImage::ImageLuma8(g),
                Output::Layer(la) if format == ImageFormat::Png => DynamicImage::ImageLumaA8(la),
                Output::Layer(la) => DynamicImage::ImageLuma8(composite::flatten(&la)),
                _ => return Err(format!("this mode does not make .{ext} files")),
            };
            let mut buf = std::io::Cursor::new(Vec::new());
            out.write_to(&mut buf, format).map_err(|e| e.to_string())?;
            buf.into_inner()
        }
        _ => {
            return Err(format!(
                "unknown file type .{ext} (use png, jpg, svg or txt)"
            ))
        }
    };
    write_atomically(path, &bytes)?;
    Ok(path.to_path_buf())
}

/// Write next to the target, then rename, so a failed write never leaves half a file.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("Could not write {}: {e}", path.display())
        })
}

/// An egui image, downscaled when a side is over MAX_TEXTURE.
fn texture_image(rgba: &RgbaImage) -> ColorImage {
    let (w, h) = rgba.dimensions();
    if w.max(h) > MAX_TEXTURE {
        let s = MAX_TEXTURE as f64 / w.max(h) as f64;
        let small = image::imageops::resize(
            rgba,
            ((w as f64 * s) as u32).max(1),
            ((h as f64 * s) as u32).max(1),
            image::imageops::FilterType::Triangle,
        );
        ColorImage::from_rgba_unmultiplied(
            [small.width() as usize, small.height() as usize],
            small.as_raw(),
        )
    } else {
        ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw())
    }
}
