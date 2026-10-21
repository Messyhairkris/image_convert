//! The `photo_tools` command: parse arguments, read the image (and a selection or mask),
//! run the job, write the result. All file I/O lives here; the library itself never
//! touches the disk.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use photo_tools::{
    cli, keep_mask, lineart_to_svg, process_masked, shapes_file, Job, Mask, Output, StrokeOptions,
};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let inv = match cli::parse(&argv) {
        Ok(inv) => inv,
        Err(exit) => {
            if exit.code == 0 {
                print!("{}", exit.message);
            } else {
                eprint!("{}", exit.message);
                if !exit.message.ends_with('\n') {
                    eprintln!();
                }
            }
            return ExitCode::from(exit.code.clamp(0, 255) as u8);
        }
    };
    if let Err(e) = check_output_kind(&inv.job, &inv.output) {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    let img = match image::open(&inv.input) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", inv.input);
            return ExitCode::from(1);
        }
    };
    let keep = match region(&inv, img.width(), img.height()) {
        Ok(keep) => keep,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match &inv.job {
        Job::LineArt(o) if extension(&inv.output) == "svg" => {
            lineart_to_svg(&img, o, &StrokeOptions::default(), keep.as_ref()).map(Output::Svg)
        }
        job => process_masked(&img, job, keep.as_ref()),
    };
    let out = match result {
        Ok(out) => out,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    match write_output(&out, &inv.output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: cannot write {}: {e}", inv.output);
            ExitCode::from(1)
        }
    }
}

/// The pixels to keep, from `--select` (a selection saved by the editor) or `--mask` (an
/// image: gray 128 and above is kept). None: the whole image.
fn region(inv: &cli::Invocation, width: u32, height: u32) -> Result<Option<Mask>, String> {
    if let Some(path) = &inv.select {
        let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let sel = shapes_file::parse(&text).map_err(|e| format!("{path}: {e}"))?;
        return Ok(keep_mask(width, height, &sel.ops, &sel.erased));
    }
    if let Some(path) = &inv.mask {
        let mask = image::open(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        if (mask.width(), mask.height()) != (width, height) {
            return Err(format!(
                "the mask {path} is {}x{} but the image is {width}x{height}",
                mask.width(),
                mask.height()
            ));
        }
        return Ok(Some(Mask::from_luma(&mask.to_luma8())));
    }
    Ok(None)
}

/// Refuse an output path that cannot hold the job's result, before any work is done.
fn check_output_kind(job: &Job, output: &str) -> Result<(), String> {
    let ext = extension(output);
    let text_out = matches!(job, Job::Effect(o) if o.style == photo_tools::Style::Ascii);
    match job {
        Job::Vector(_) if output != "-" && ext != "svg" => Err(format!(
            "--mode vector writes SVG: give an .svg path or - (got {output})"
        )),
        Job::Vector(_) => Ok(()),
        _ if text_out => Ok(()),
        _ if output == "-" => Err("an image cannot be written to the terminal; give a .png, \
                                   .jpg (or, for line art, .svg) path"
            .to_string()),
        Job::LineArt(_) if ext == "svg" => Ok(()),
        _ if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") => Err(format!(
            "images are written as PNG or JPEG (line art also as SVG): give a .png or .jpg \
             path (got {output})"
        )),
        _ => Ok(()),
    }
}

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Write via a temporary file in the same folder and rename it into place, so a
/// failure never leaves a half-written file under the requested name. A selection's
/// transparency is kept in PNG; JPEG has none, so outside the selection becomes white.
fn write_output(out: &Output, path: &str) -> io::Result<()> {
    if path == "-" {
        let text = match out {
            Output::Svg(s) | Output::Text(s) => s,
            Output::Image(_) | Output::Layer(_) => unreachable!("checked by check_output_kind"),
        };
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())?;
        return stdout.flush();
    }
    let target = PathBuf::from(path);
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let ext = extension(path);
    let tmp = target.with_extension(format!("{ext}.partial"));
    let format = if ext == "png" {
        image::ImageFormat::Png
    } else {
        image::ImageFormat::Jpeg
    };
    let written = match out {
        Output::Svg(s) | Output::Text(s) => fs::write(&tmp, s),
        Output::Image(g) => g.save_with_format(&tmp, format).map_err(io::Error::other),
        Output::Layer(la) if format == image::ImageFormat::Png => {
            la.save_with_format(&tmp, format).map_err(io::Error::other)
        }
        Output::Layer(la) => photo_tools::composite::flatten(la)
            .save_with_format(&tmp, format)
            .map_err(io::Error::other),
    };
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if target.exists() {
        fs::remove_file(&target)?;
    }
    fs::rename(&tmp, &target)
}
