//! Options, validation and the library's error type.

use std::fmt;

/// Everything the library can refuse.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidOption { name: &'static str, reason: String },
    EmptyImage,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidOption { name, reason } => write!(f, "invalid --{name}: {reason}"),
            Error::EmptyImage => f.write_str("the image has no pixels"),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeFilter {
    Sobel,
    Canny,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineArtOptions {
    pub filter: EdgeFilter,
    pub blur: f32,
    pub low: f32,
    pub high: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorOptions {
    pub colors: u8,
    pub smoothness: f32,
    pub speckle: u32,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Sketch,
    Dither,
    Halftone,
    Hatch,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectOptions {
    pub style: Style,
    pub blur: f32,
    pub cell: u32,
    pub columns: u32,
    pub ansi: bool,
    pub invert: bool,
}

/// What to make from an image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Job {
    LineArt(LineArtOptions),
    Vector(VectorOptions),
    Effect(EffectOptions),
}

impl Default for LineArtOptions {
    fn default() -> Self {
        Self {
            filter: EdgeFilter::Canny,
            blur: 1.0,
            low: 50.0,
            high: 100.0,
        }
    }
}
impl Default for VectorOptions {
    fn default() -> Self {
        Self {
            colors: 8,
            smoothness: 1.0,
            speckle: 8,
            max_iterations: 20,
        }
    }
}
impl Default for EffectOptions {
    fn default() -> Self {
        Self {
            style: Style::Sketch,
            blur: 8.0,
            cell: 6,
            columns: 100,
            ansi: false,
            invert: false,
        }
    }
}

impl LineArtOptions {
    pub fn validate(&self) -> Result<(), Error> {
        if !self.blur.is_finite() || self.blur < 0.0 || self.blur > 50.0 {
            return Err(Error::InvalidOption {
                name: "blur",
                reason: "must be a number from 0 to 50".to_string(),
            });
        }
        if !self.low.is_finite() || self.low < 0.0 {
            return Err(Error::InvalidOption {
                name: "threshold",
                reason: "must be a number of 0 or more".to_string(),
            });
        }
        if !self.high.is_finite() || self.high < 0.0 {
            return Err(Error::InvalidOption {
                name: "threshold",
                reason: "must be a number of 0 or more".to_string(),
            });
        }
        if self.high < self.low {
            return Err(Error::InvalidOption {
                name: "threshold",
                reason: "the high limit must not be below the low limit".to_string(),
            });
        }
        Ok(())
    }
}

impl VectorOptions {
    pub fn validate(&self) -> Result<(), Error> {
        if self.colors < 2 || self.colors > 64 {
            return Err(Error::InvalidOption {
                name: "colors",
                reason: "must be from 2 to 64".to_string(),
            });
        }
        if !self.smoothness.is_finite() || self.smoothness < 0.0 || self.smoothness > 10.0 {
            return Err(Error::InvalidOption {
                name: "smoothness",
                reason: "must be a number from 0 to 10".to_string(),
            });
        }
        if self.speckle > 10000 {
            return Err(Error::InvalidOption {
                name: "speckle",
                reason: "must be from 0 to 10000".to_string(),
            });
        }
        if self.max_iterations < 1 || self.max_iterations > 100 {
            return Err(Error::InvalidOption {
                name: "iterations",
                reason: "must be from 1 to 100".to_string(),
            });
        }
        Ok(())
    }
}

impl EffectOptions {
    pub fn validate(&self) -> Result<(), Error> {
        if !self.blur.is_finite() || self.blur < 0.5 || self.blur > 100.0 {
            return Err(Error::InvalidOption {
                name: "blur",
                reason: "must be a number from 0.5 to 100".to_string(),
            });
        }
        if self.cell < 2 || self.cell > 64 {
            return Err(Error::InvalidOption {
                name: "cell",
                reason: "must be from 2 to 64".to_string(),
            });
        }
        if self.columns < 1 || self.columns > 1000 {
            return Err(Error::InvalidOption {
                name: "columns",
                reason: "must be from 1 to 1000".to_string(),
            });
        }
        Ok(())
    }
}

pub fn parse_threshold(text: &str) -> Result<(f32, f32), Error> {
    let parts: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    if parts.len() > 2 || parts.iter().any(|s| s.is_empty()) {
        return Err(Error::InvalidOption {
            name: "threshold",
            reason: "expected a number like 45 or two like 20,60".to_string(),
        });
    }
    let nums: Result<Vec<f32>, _> = parts.iter().map(|s| s.parse::<f32>()).collect();
    let nums = match nums {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::InvalidOption {
                name: "threshold",
                reason: "expected a number like 45 or two like 20,60".to_string(),
            });
        }
    };
    let (low, high) = match nums.len() {
        1 => (nums[0], nums[0] * 2.0),
        2 => (nums[0], nums[1]),
        _ => unreachable!(),
    };
    if !low.is_finite() || low < 0.0 {
        return Err(Error::InvalidOption {
            name: "threshold",
            reason: "must be a number of 0 or more".to_string(),
        });
    }
    if !high.is_finite() || high < 0.0 {
        return Err(Error::InvalidOption {
            name: "threshold",
            reason: "must be a number of 0 or more".to_string(),
        });
    }
    if high < low {
        return Err(Error::InvalidOption {
            name: "threshold",
            reason: "the high limit must not be below the low limit".to_string(),
        });
    }
    Ok((low, high))
}
