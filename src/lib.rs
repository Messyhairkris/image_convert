//! Non-AI image effects as a library: line art (PNG or SVG centerlines), flat-colour SVG,
//! pencil sketch, Floyd-Steinberg dither, halftone, cross-hatching and ASCII art - for the
//! whole image or only a selected area.
//!
//! Everything here is pure: images come in as [`image::DynamicImage`], results go out
//! as values ([`Output`]). Reading and writing files is the caller's business (the
//! `photo_tools` binary does it in `main.rs`), so the library can be embedded in other
//! programs - a GUI, a server, a batch tool - without touching their I/O.
//!
//! ```no_run
//! use photo_tools::{process, Job, Output, VectorOptions};
//!
//! let img = image::open("photo.jpg").unwrap();
//! let job = Job::Vector(VectorOptions { colors: 8, ..VectorOptions::default() });
//! if let Ok(Output::Svg(svg)) = process(&img, &job) {
//!     std::fs::write("photo.svg", svg).unwrap();
//! }
//! ```
//!
//! Selections: build a [`Mask`] from lasso / polyline polygons and brush strokes
//! ([`select`]), or drive the tools with pointer events ([`editor::Editor`]), then call
//! [`process_masked`]. Outside the mask is transparent white.
//!
//! Embedding without the command line (no clap):
//! `photo_tools = { path = "...", default-features = false }`.

pub mod ascii;
pub mod composite;
pub mod contour;
pub mod despeckle;
pub mod dither;
pub mod dodge;
pub mod edges;
pub mod editor;
pub mod effects;
pub mod halftone;
pub mod hatch;
pub mod junction;
pub mod kmeans;
pub mod lineart;
pub mod lineart_svg;
pub mod luma;
pub mod options;
pub mod select;
pub mod shapes_file;
pub mod smooth;
pub mod sobel;
pub mod strokes;
pub mod svg;
pub mod thin;
pub mod vector;
pub mod vector_select;

#[cfg(feature = "cli")]
pub mod cli;

pub use effects::EffectOutput;
pub use lineart_svg::StrokeOptions;
pub use options::{
    parse_threshold, EdgeFilter, EffectOptions, Error, Job, LineArtOptions, Style, VectorOptions,
};
pub use select::{Mask, Op, Shape};

/// The `image` crate this library is built against, so callers can use the same version.
pub use image;

/// What a job produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Line art, sketch, dither, halftone and hatch: an 8-bit gray image.
    Image(image::GrayImage),
    /// The same for a selection: gray + alpha, transparent white outside it.
    Layer(image::GrayAlphaImage),
    /// Vector mode: a complete SVG document.
    Svg(String),
    /// ASCII mode: text, one line per row of cells (with ANSI colour if asked).
    Text(String),
}

/// Run one job on an image. Options are validated first; an image with no pixels is
/// refused with [`Error::EmptyImage`].
pub fn process(img: &image::DynamicImage, job: &Job) -> Result<Output, Error> {
    match job {
        Job::LineArt(o) => lineart::render(img, o).map(Output::Image),
        Job::Vector(o) => vector::render(img, o).map(Output::Svg),
        Job::Effect(o) => Ok(match effects::render(img, o)? {
            EffectOutput::Image(g) => Output::Image(g),
            EffectOutput::Text(t) => Output::Text(t),
        }),
    }
}

/// The pixels to keep: the selection (everything when there is none) minus the erased
/// strokes. `None` when there is neither a selection nor anything erased.
pub fn keep_mask(
    width: u32,
    height: u32,
    selection: &[(Op, Shape)],
    erased: &[Shape],
) -> Option<Mask> {
    let selected = select::build(width, height, selection);
    if erased.is_empty() {
        return selected;
    }
    let mut mask = selected.unwrap_or_else(|| Mask::new(width, height, true));
    for shape in erased {
        mask.apply(Op::Subtract, shape);
    }
    Some(mask)
}

/// Like [`process`], but only the pixels in `keep` are rendered: images come back as
/// [`Output::Layer`] (transparent white outside), vector SVG draws nothing outside (its
/// palette is fitted to the kept pixels), and ASCII prints spaces outside. `None` is
/// exactly [`process`].
pub fn process_masked(
    img: &image::DynamicImage,
    job: &Job,
    keep: Option<&Mask>,
) -> Result<Output, Error> {
    let Some(keep) = keep else {
        return process(img, job);
    };
    match job {
        Job::LineArt(o) => Ok(Output::Layer(composite::keep(
            &lineart::render(img, o)?,
            keep,
        )?)),
        Job::Vector(o) => vector_select::render_selected(img, o, keep).map(Output::Svg),
        Job::Effect(o) if o.style == Style::Ascii => {
            o.validate()?;
            if img.width() == 0 || img.height() == 0 {
                return Err(Error::EmptyImage);
            }
            let rgb = img.to_rgb8();
            let mut gray = luma::rgb_to_gray(rgb.as_raw());
            composite::blank_outside(&mut gray, keep, o.invert)?;
            let colour = if o.ansi {
                Some(rgb.as_raw().as_slice())
            } else {
                None
            };
            Ok(Output::Text(ascii::render(
                &gray,
                colour,
                img.width() as usize,
                img.height() as usize,
                o.columns as usize,
                o.invert,
            )))
        }
        Job::Effect(o) => match effects::render(img, o)? {
            EffectOutput::Image(g) => Ok(Output::Layer(composite::keep(&g, keep)?)),
            EffectOutput::Text(t) => Ok(Output::Text(t)),
        },
    }
}

/// Line art as SVG centerline strokes, only inside `keep` (see [`lineart_svg`]).
pub fn lineart_to_svg(
    img: &image::DynamicImage,
    opts: &LineArtOptions,
    stroke: &StrokeOptions,
    keep: Option<&Mask>,
) -> Result<String, Error> {
    lineart_svg::render_svg(img, opts, stroke, keep)
}
