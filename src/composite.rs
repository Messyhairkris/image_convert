//! Selections and erasing applied to rendered images: outside is transparent white.

use image::{GrayAlphaImage, GrayImage, LumaA};

use crate::options::Error;
use crate::select::Mask;

/// Rule 1, shared with `erase`: refuse a mask that is not the image's size.
fn size_check(mask: &Mask, img_w: u32, img_h: u32) -> Result<(), Error> {
    if mask.width() != img_w || mask.height() != img_h {
        return Err(Error::InvalidOption {
            name: "selection",
            reason: format!(
                "is {}x{} but the image is {}x{}",
                mask.width(),
                mask.height(),
                img_w,
                img_h
            ),
        });
    }
    Ok(())
}

/// Every pixel opaque: (v, 255).
pub fn opaque(img: &GrayImage) -> GrayAlphaImage {
    GrayAlphaImage::from_fn(img.width(), img.height(), |x, y| {
        LumaA([img.get_pixel(x, y)[0], 255])
    })
}

/// Selected pixels keep their value, opaque; the rest become transparent white (255, 0).
pub fn keep(img: &GrayImage, keep: &Mask) -> Result<GrayAlphaImage, Error> {
    size_check(keep, img.width(), img.height())?;
    let out = GrayAlphaImage::from_fn(img.width(), img.height(), |x, y| {
        if keep.get(x, y) {
            LumaA([img.get_pixel(x, y)[0], 255])
        } else {
            LumaA([255, 0])
        }
    });
    Ok(out)
}

/// Erased pixels (true in `erased`) become transparent white (255, 0).
pub fn erase(img: &mut GrayAlphaImage, erased: &Mask) -> Result<(), Error> {
    size_check(erased, img.width(), img.height())?;
    for (pixel, bit) in img.pixels_mut().zip(erased.bits().iter()) {
        if *bit {
            *pixel = LumaA([255, 0]);
        }
    }
    Ok(())
}

/// For formats without transparency (JPEG): transparent pixels become white 255.
pub fn flatten(img: &GrayAlphaImage) -> GrayImage {
    GrayImage::from_fn(img.width(), img.height(), |x, y| {
        let pixel = img.get_pixel(x, y);
        image::Luma([if pixel[1] == 0 { 255 } else { pixel[0] }])
    })
}

/// Before ASCII art: unselected pixels of a row-major gray buffer become blank -
/// 255, or 0 when the art is inverted - so cells outside the selection print spaces.
pub fn blank_outside(gray: &mut [u8], keep: &Mask, invert: bool) -> Result<(), Error> {
    // Rule 2: length check instead of a size check.
    if gray.len() != keep.bits().len() {
        return Err(Error::InvalidOption {
            name: "selection",
            reason: format!(
                "has {} pixels but the image has {}",
                keep.bits().len(),
                gray.len()
            ),
        });
    }
    let blank = if invert { 0 } else { 255 };
    for (byte, bit) in gray.iter_mut().zip(keep.bits().iter()) {
        if !*bit {
            *byte = blank;
        }
    }
    Ok(())
}
