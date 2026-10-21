//! Command-line parsing into a Job.

use clap::error::ErrorKind;
use clap::{Parser, ValueEnum};

use crate::options::{
    parse_threshold, EdgeFilter, EffectOptions, Job, LineArtOptions, Style, VectorOptions,
};

/// The `--mode` value: which kind of job to run.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeArg {
    Lineart,
    Vector,
    Fx,
}

/// The `--filter` value for line art.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterArg {
    Sobel,
    Canny,
}

/// The `--style` value for effects.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleArg {
    Sketch,
    Dither,
    Halftone,
    Hatch,
    Ascii,
}

/// Non-AI image effects: line art, flat-colour SVG, sketch, dither, halftone,
/// hatching and ASCII art.
#[derive(Parser, Debug)]
#[command(name = "photo_tools", version)]
pub struct Args {
    /// Path of the image to read.
    #[arg(long)]
    pub input: String,
    /// Path to write the result to.
    #[arg(long)]
    pub output: String,
    /// Which kind of job to run.
    #[arg(long, value_enum)]
    pub mode: ModeArg,
    /// Edge filter for line art.
    #[arg(long, value_enum)]
    pub filter: Option<FilterArg>,
    /// Blur amount.
    #[arg(long)]
    pub blur: Option<f32>,
    /// Edge threshold, like 45 or 20,60.
    #[arg(long)]
    pub threshold: Option<String>,
    /// Number of colours for vector art.
    #[arg(long)]
    pub colors: Option<u8>,
    /// Smoothing amount for vector art.
    #[arg(long)]
    pub smoothness: Option<f32>,
    /// Speckle amount for vector art.
    #[arg(long)]
    pub speckle: Option<u32>,
    /// Maximum iterations for vector art.
    #[arg(long)]
    pub iterations: Option<u32>,
    /// Effect style.
    #[arg(long, value_enum)]
    pub style: Option<StyleArg>,
    /// Cell size for effects.
    #[arg(long)]
    pub cell: Option<u32>,
    /// Column count for ASCII art.
    #[arg(long)]
    pub columns: Option<u32>,
    /// Use ANSI colours for ASCII art.
    #[arg(long)]
    pub ansi: bool,
    /// Invert the output.
    #[arg(long)]
    pub invert: bool,
    /// Path to a file listing pixels to select.
    #[arg(long)]
    pub select: Option<String>,
    /// Path to a file listing pixels to mask.
    #[arg(long)]
    pub mask: Option<String>,
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub input: String,
    pub output: String,
    pub job: Job,
    pub select: Option<String>,
    pub mask: Option<String>,
}

/// Why parsing stopped: help or version text (code 0) or an error (code 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exit {
    pub code: i32,
    pub message: String,
}

/// Parse a command line into an `Invocation`, or an `Exit` if it stopped.
pub fn parse(argv: &[String]) -> Result<Invocation, Exit> {
    // Rule 1: clap does the lexical parsing; help and version are code 0, the rest 2.
    let args = match Args::try_parse_from(argv.iter()) {
        Ok(args) => args,
        Err(e) => {
            let code = if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                0
            } else {
                2
            };
            return Err(Exit {
                code,
                message: e.to_string(),
            });
        }
    };

    // Rule 3: flags are checked in `Args` field order before anything is built.
    if let Some(exit) = flag_violation(&args) {
        return Err(exit);
    }

    // Rule 3a: --select and --mask are mutually exclusive.
    if args.select.is_some() && args.mask.is_some() {
        return Err(Exit {
            code: 2,
            message: "error: use --select or --mask, not both".to_string(),
        });
    }

    // Rules 4 and 5: start from the mode's defaults, overwrite what was given,
    // then validate.
    let job = build_job(&args)?;

    // Rule 6.
    Ok(Invocation {
        input: args.input,
        output: args.output,
        job,
        select: args.select,
        mask: args.mask,
    })
}

/// Rule 3: the first given flag (in `Args` field order) that does not belong to
/// the chosen mode is an error.
fn flag_violation(args: &Args) -> Option<Exit> {
    let mode = args.mode;
    let mode_name = mode_name(mode);
    let checks: [(&str, bool, bool); 12] = [
        (
            "filter",
            args.filter.is_some(),
            matches!(mode, ModeArg::Lineart),
        ),
        (
            "blur",
            args.blur.is_some(),
            matches!(mode, ModeArg::Lineart | ModeArg::Fx),
        ),
        (
            "threshold",
            args.threshold.is_some(),
            matches!(mode, ModeArg::Lineart),
        ),
        (
            "colors",
            args.colors.is_some(),
            matches!(mode, ModeArg::Vector),
        ),
        (
            "smoothness",
            args.smoothness.is_some(),
            matches!(mode, ModeArg::Vector),
        ),
        (
            "speckle",
            args.speckle.is_some(),
            matches!(mode, ModeArg::Vector),
        ),
        (
            "iterations",
            args.iterations.is_some(),
            matches!(mode, ModeArg::Vector),
        ),
        ("style", args.style.is_some(), matches!(mode, ModeArg::Fx)),
        ("cell", args.cell.is_some(), matches!(mode, ModeArg::Fx)),
        (
            "columns",
            args.columns.is_some(),
            matches!(mode, ModeArg::Fx),
        ),
        ("ansi", args.ansi, matches!(mode, ModeArg::Fx)),
        ("invert", args.invert, matches!(mode, ModeArg::Fx)),
    ];
    for (flag, given, applies) in checks {
        if given && !applies {
            return Some(Exit {
                code: 2,
                message: format!("error: --{flag} does not apply to --mode {mode_name}"),
            });
        }
    }
    None
}

/// The mode's name on the command line, for rule 3's error text.
fn mode_name(mode: ModeArg) -> &'static str {
    match mode {
        ModeArg::Lineart => "lineart",
        ModeArg::Vector => "vector",
        ModeArg::Fx => "fx",
    }
}

/// Rule 5: a library `Error` becomes exit code 2 with its Display text.
fn to_exit(e: crate::options::Error) -> Exit {
    Exit {
        code: 2,
        message: format!("error: {e}"),
    }
}

/// Rules 4 and 5: build the mode's options from its defaults, overwriting only
/// what was given, then validate.
fn build_job(args: &Args) -> Result<Job, Exit> {
    match args.mode {
        ModeArg::Lineart => {
            let mut opts = LineArtOptions::default();
            if let Some(filter) = args.filter {
                opts.filter = match filter {
                    FilterArg::Sobel => EdgeFilter::Sobel,
                    FilterArg::Canny => EdgeFilter::Canny,
                };
            }
            if let Some(blur) = args.blur {
                opts.blur = blur;
            }
            if let Some(text) = &args.threshold {
                let (low, high) = parse_threshold(text).map_err(to_exit)?;
                opts.low = low;
                opts.high = high;
            }
            opts.validate().map_err(to_exit)?;
            Ok(Job::LineArt(opts))
        }
        ModeArg::Vector => {
            let mut opts = VectorOptions::default();
            if let Some(colors) = args.colors {
                opts.colors = colors;
            }
            if let Some(smoothness) = args.smoothness {
                opts.smoothness = smoothness;
            }
            if let Some(speckle) = args.speckle {
                opts.speckle = speckle;
            }
            if let Some(iterations) = args.iterations {
                opts.max_iterations = iterations;
            }
            opts.validate().map_err(to_exit)?;
            Ok(Job::Vector(opts))
        }
        ModeArg::Fx => {
            let mut opts = EffectOptions::default();
            if let Some(style) = args.style {
                opts.style = match style {
                    StyleArg::Sketch => Style::Sketch,
                    StyleArg::Dither => Style::Dither,
                    StyleArg::Halftone => Style::Halftone,
                    StyleArg::Hatch => Style::Hatch,
                    StyleArg::Ascii => Style::Ascii,
                };
            }
            if let Some(blur) = args.blur {
                opts.blur = blur;
            }
            if let Some(cell) = args.cell {
                opts.cell = cell;
            }
            if let Some(columns) = args.columns {
                opts.columns = columns;
            }
            opts.ansi = args.ansi;
            opts.invert = args.invert;
            opts.validate().map_err(to_exit)?;
            Ok(Job::Effect(opts))
        }
    }
}
