//! Line art: black edges on white.

use image::{DynamicImage, GrayImage};

use crate::luma::rgb_to_gray;
use crate::options::{EdgeFilter, Error, LineArtOptions};
use crate::sobel::edge_mask;
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;

/// Validate, convert to gray (Rec. 601), blur, then Sobel or Canny.
///
/// Returns a gray image with black edges on a white background.
pub fn render(img: &DynamicImage, opts: &LineArtOptions) -> Result<GrayImage, Error> {
    // Rule 1: validate options first.
    opts.validate()?;

    // Rule 2: empty image check.
    if img.width() == 0 || img.height() == 0 {
        return Err(Error::EmptyImage);
    }

    // Rule 3: convert to gray using Rec. 601 weights.
    let rgb = img.to_rgb8();
    let gray_bytes = rgb_to_gray(rgb.as_raw());
    let w = img.width();
    let h = img.height();
    let mut gray =
        GrayImage::from_raw(w, h, gray_bytes).expect("gray buffer length matches image dimensions");

    // Rule 4: optional Gaussian blur.
    if opts.blur > 0.0 {
        gray = gaussian_blur_f32(&gray, opts.blur);
    }

    // Rule 5: edge detection.
    let result = match opts.filter {
        EdgeFilter::Sobel => {
            // threshold = rounded low, capped at u16::MAX.
            let mut thr = opts.low.round() as i64;
            if thr < 0 {
                thr = 0;
            }
            if thr > u16::MAX as i64 {
                thr = u16::MAX as i64;
            }
            let threshold = thr as u16;
            let mask = edge_mask(gray.as_raw(), w as usize, h as usize, threshold);
            GrayImage::from_raw(w, h, mask).expect("edge mask length matches dimensions")
        }
        EdgeFilter::Canny => {
            // Canny returns white edges on black; invert to get black on white.
            let canny_img = canny(&gray, opts.low, opts.high);
            let inverted: Vec<u8> = canny_img.into_raw().into_iter().map(|v| 255 - v).collect();
            GrayImage::from_raw(w, h, inverted).expect("inverted Canny length matches dimensions")
        }
    };

    // Rule 6: return the final gray image.
    Ok(result)
}
