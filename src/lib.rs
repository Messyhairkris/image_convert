//! Non-AI image effects as a library: line art, flat-colour SVG, pencil sketch,
//! Floyd-Steinberg dither, halftone, cross-hatching and ASCII art.
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
//! Embedding without the command line (no clap):
//! `photo_tools = { path = "...", default-features = false }`.

pub mod ascii;
pub mod contour;
pub mod despeckle;
pub mod dither;
pub mod dodge;
pub mod effects;
pub mod halftone;
pub mod hatch;
pub mod kmeans;
pub mod lineart;
pub mod luma;
pub mod options;
pub mod smooth;
pub mod sobel;
pub mod svg;
pub mod vector;

#[cfg(feature = "cli")]
pub mod cli;

pub use effects::EffectOutput;
pub use options::{
    parse_threshold, EdgeFilter, EffectOptions, Error, Job, LineArtOptions, Style, VectorOptions,
};

/// The `image` crate this library is built against, so callers can use the same version.
pub use image;

/// What a job produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Line art, sketch, dither, halftone and hatch: an 8-bit gray image.
    Image(image::GrayImage),
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
