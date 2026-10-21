//! Sketch, dither, halftone, hatch and ASCII effects.

use image::{DynamicImage, GrayImage};
use imageproc::filter::gaussian_blur_f32;

use crate::ascii;
use crate::dither::floyd_steinberg;
use crate::dodge::dodge_layers;
use crate::halftone::halftone;
use crate::hatch::hatch;
use crate::luma::{invert, rgb_to_gray};
use crate::options::{EffectOptions, Error, Style};

/// An effect's result: a gray image, or text for the ASCII style.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectOutput {
    Image(GrayImage),
    Text(String),
}

/// Apply `opts.style` to `img`.
pub fn render(img: &DynamicImage, opts: &EffectOptions) -> Result<EffectOutput, Error> {
    // Rule 1: validate options before anything else.
    opts.validate()?;
    // Rule 2: an empty image is refused.
    let w = img.width() as usize;
    let h = img.height() as usize;
    if w == 0 || h == 0 {
        return Err(Error::EmptyImage);
    }
    // Rule 3: gray comes from crate::luma (Rec. 601), never to_luma8.
    let rgb = img.to_rgb8();
    let gray = rgb_to_gray(rgb.as_raw());
    let cell = opts.cell as usize;
    // Rule 4: one branch per style.
    let pixels: Vec<u8> = match opts.style {
        Style::Sketch => {
            // from_raw is Some: the buffer is exactly w * h bytes.
            let inverted = GrayImage::from_raw(w as u32, h as u32, invert(&gray)).unwrap();
            let blurred = gaussian_blur_f32(&inverted, opts.blur);
            dodge_layers(&gray, blurred.as_raw())
        }
        Style::Dither => floyd_steinberg(&gray, w, h),
        Style::Halftone => halftone(&gray, w, h, cell),
        Style::Hatch => hatch(&gray, w, h, cell),
        Style::Ascii => {
            let colour = if opts.ansi {
                Some(rgb.as_raw().as_slice())
            } else {
                None
            };
            let text = ascii::render(&gray, colour, w, h, opts.columns as usize, opts.invert);
            return Ok(EffectOutput::Text(text));
        }
    };
    // Rule 5: the image styles return a GrayImage of the input's size.
    let out = GrayImage::from_raw(w as u32, h as u32, pixels).unwrap();
    Ok(EffectOutput::Image(out))
}
