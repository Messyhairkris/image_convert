//! Integration tests for `photo_tools::options`: defaults, validation rules at their
//! boundaries, the `Display` text, and every `parse_threshold` worked-example row.

use photo_tools::options::{
    parse_threshold, EdgeFilter, EffectOptions, Error, Job, LineArtOptions, Style, VectorOptions,
};

/// `bad(name, reason)` from the contract: `Err(Error::InvalidOption { name, reason })`.
fn bad(name: &'static str, reason: &str) -> Error {
    Error::InvalidOption {
        name,
        reason: reason.to_string(),
    }
}

#[test]
fn rule_1_display_text() {
    // Worked examples: both variants, whole strings.
    assert_eq!(
        Error::InvalidOption {
            name: "colors",
            reason: "must be from 2 to 64".to_string()
        }
        .to_string(),
        "invalid --colors: must be from 2 to 64"
    );
    assert_eq!(Error::EmptyImage.to_string(), "the image has no pixels");
}

#[test]
fn rule_2_defaults() {
    // Rule 2: exact default values for all three options structs.
    assert_eq!(
        LineArtOptions::default(),
        LineArtOptions {
            filter: EdgeFilter::Canny,
            blur: 1.0,
            low: 50.0,
            high: 100.0
        }
    );
    assert_eq!(
        VectorOptions::default(),
        VectorOptions {
            colors: 8,
            smoothness: 1.0,
            speckle: 8,
            max_iterations: 20
        }
    );
    assert_eq!(
        EffectOptions::default(),
        EffectOptions {
            style: Style::Sketch,
            blur: 8.0,
            cell: 6,
            columns: 100,
            ansi: false,
            invert: false,
        }
    );
}

#[test]
fn rule_3_lineart_blur() {
    // Rule 3a: blur must be finite and in 0.0..=50.0 (closed range).
    let ok = [0.0f32, 1.0, 50.0];
    for blur in ok {
        assert_eq!(
            LineArtOptions {
                blur,
                ..LineArtOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    let bad_blur = [-0.001f32, 50.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    for blur in bad_blur {
        assert_eq!(
            LineArtOptions {
                blur,
                ..LineArtOptions::default()
            }
            .validate(),
            Err(bad("blur", "must be a number from 0 to 50"))
        );
    }
    // Worked example: blur 50.1 fails.
    assert_eq!(
        LineArtOptions {
            blur: 50.1,
            ..LineArtOptions::default()
        }
        .validate(),
        Err(bad("blur", "must be a number from 0 to 50"))
    );
    // Worked example: blur 50.0 passes.
    assert_eq!(
        LineArtOptions {
            blur: 50.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Ok(())
    );
}

#[test]
fn rule_3_lineart_threshold() {
    // Rule 3b: low/high must be finite and >= 0.0.
    let bad_threshold = [
        LineArtOptions {
            low: -0.001,
            ..LineArtOptions::default()
        },
        LineArtOptions {
            high: -0.001,
            ..LineArtOptions::default()
        },
        LineArtOptions {
            low: f32::NAN,
            ..LineArtOptions::default()
        },
        LineArtOptions {
            high: f32::NAN,
            ..LineArtOptions::default()
        },
        LineArtOptions {
            low: f32::NEG_INFINITY,
            ..LineArtOptions::default()
        },
        LineArtOptions {
            high: f32::NEG_INFINITY,
            ..LineArtOptions::default()
        },
    ];
    for opts in bad_threshold {
        assert_eq!(
            opts.validate(),
            Err(bad("threshold", "must be a number of 0 or more"))
        );
    }
    // Rule 3c: high < low fails; high == low is allowed (strict comparison).
    assert_eq!(
        LineArtOptions {
            low: 60.0,
            high: 50.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Err(bad(
            "threshold",
            "the high limit must not be below the low limit"
        ))
    );
    assert_eq!(
        LineArtOptions {
            low: 0.0,
            high: 0.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        LineArtOptions {
            low: 42.0,
            high: 42.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Ok(())
    );
    // Worked example: blur 99.0 and low -1.0 -> the blur error (3a before 3b).
    assert_eq!(
        LineArtOptions {
            blur: 99.0,
            low: -1.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Err(bad("blur", "must be a number from 0 to 50"))
    );
    // 3b before 3c: a negative low and an inverted pair -> the 0-or-more error.
    assert_eq!(
        LineArtOptions {
            low: -1.0,
            high: -2.0,
            ..LineArtOptions::default()
        }
        .validate(),
        Err(bad("threshold", "must be a number of 0 or more"))
    );
}

#[test]
fn rule_4_vector_colors() {
    // Rule 4a: colors in 2..=64 (closed range).
    for colors in [2u8, 8, 64] {
        assert_eq!(
            VectorOptions {
                colors,
                ..VectorOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for colors in [0u8, 1, 65] {
        assert_eq!(
            VectorOptions {
                colors,
                ..VectorOptions::default()
            }
            .validate(),
            Err(bad("colors", "must be from 2 to 64"))
        );
    }
    // Worked example: colors 0 and max_iterations 0 -> the colors error (4a first).
    assert_eq!(
        VectorOptions {
            colors: 0,
            max_iterations: 0,
            ..VectorOptions::default()
        }
        .validate(),
        Err(bad("colors", "must be from 2 to 64"))
    );
}

#[test]
fn rule_4_vector_smoothness() {
    // Rule 4b: smoothness finite and in 0.0..=10.0 (closed range).
    for smoothness in [0.0f32, 1.0, 10.0] {
        assert_eq!(
            VectorOptions {
                smoothness,
                ..VectorOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for smoothness in [-0.001f32, 10.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            VectorOptions {
                smoothness,
                ..VectorOptions::default()
            }
            .validate(),
            Err(bad("smoothness", "must be a number from 0 to 10"))
        );
    }
    // 4a before 4b: bad colors and bad smoothness -> the colors error.
    assert_eq!(
        VectorOptions {
            colors: 1,
            smoothness: 11.0,
            ..VectorOptions::default()
        }
        .validate(),
        Err(bad("colors", "must be from 2 to 64"))
    );
}

#[test]
fn rule_4_vector_speckle() {
    // Rule 4c: speckle > 10000 fails; 0 and 10000 pass.
    for speckle in [0u32, 8, 10_000] {
        assert_eq!(
            VectorOptions {
                speckle,
                ..VectorOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    assert_eq!(
        VectorOptions {
            speckle: 10_001,
            ..VectorOptions::default()
        }
        .validate(),
        Err(bad("speckle", "must be from 0 to 10000"))
    );
    // 4b before 4c: bad smoothness and bad speckle -> the smoothness error.
    assert_eq!(
        VectorOptions {
            smoothness: -1.0,
            speckle: 10_001,
            ..VectorOptions::default()
        }
        .validate(),
        Err(bad("smoothness", "must be a number from 0 to 10"))
    );
}

#[test]
fn rule_4_vector_max_iterations() {
    // Rule 4d: max_iterations in 1..=100 (closed range).
    for max_iterations in [1u32, 20, 100] {
        assert_eq!(
            VectorOptions {
                max_iterations,
                ..VectorOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for max_iterations in [0u32, 101] {
        assert_eq!(
            VectorOptions {
                max_iterations,
                ..VectorOptions::default()
            }
            .validate(),
            Err(bad("iterations", "must be from 1 to 100"))
        );
    }
    // 4c before 4d: bad speckle and bad max_iterations -> the speckle error.
    assert_eq!(
        VectorOptions {
            speckle: 10_001,
            max_iterations: 0,
            ..VectorOptions::default()
        }
        .validate(),
        Err(bad("speckle", "must be from 0 to 10000"))
    );
}

#[test]
fn rule_5_effect_blur() {
    // Rule 5a: blur finite and in 0.5..=100.0 (closed range).
    for blur in [0.5f32, 8.0, 100.0] {
        assert_eq!(
            EffectOptions {
                blur,
                ..EffectOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for blur in [0.4f32, 100.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            EffectOptions {
                blur,
                ..EffectOptions::default()
            }
            .validate(),
            Err(bad("blur", "must be a number from 0.5 to 100"))
        );
    }
    // Worked example: blur 0.4 fails.
    assert_eq!(
        EffectOptions {
            blur: 0.4,
            ..EffectOptions::default()
        }
        .validate(),
        Err(bad("blur", "must be a number from 0.5 to 100"))
    );
}

#[test]
fn rule_5_effect_cell() {
    // Rule 5b: cell in 2..=64 (closed range).
    for cell in [2u32, 6, 64] {
        assert_eq!(
            EffectOptions {
                cell,
                ..EffectOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for cell in [1u32, 65] {
        assert_eq!(
            EffectOptions {
                cell,
                ..EffectOptions::default()
            }
            .validate(),
            Err(bad("cell", "must be from 2 to 64"))
        );
    }
    // 5a before 5b: bad blur and bad cell -> the blur error.
    assert_eq!(
        EffectOptions {
            blur: 0.4,
            cell: 1,
            ..EffectOptions::default()
        }
        .validate(),
        Err(bad("blur", "must be a number from 0.5 to 100"))
    );
}

#[test]
fn rule_5_effect_columns() {
    // Rule 5c: columns in 1..=1000 (closed range).
    for columns in [1u32, 100, 1000] {
        assert_eq!(
            EffectOptions {
                columns,
                ..EffectOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
    for columns in [0u32, 1001] {
        assert_eq!(
            EffectOptions {
                columns,
                ..EffectOptions::default()
            }
            .validate(),
            Err(bad("columns", "must be from 1 to 1000"))
        );
    }
    // Worked example: columns 0 fails.
    assert_eq!(
        EffectOptions {
            columns: 0,
            ..EffectOptions::default()
        }
        .validate(),
        Err(bad("columns", "must be from 1 to 1000"))
    );
    // 5b before 5c: bad cell and bad columns -> the cell error.
    assert_eq!(
        EffectOptions {
            cell: 1,
            columns: 0,
            ..EffectOptions::default()
        }
        .validate(),
        Err(bad("cell", "must be from 2 to 64"))
    );
}

#[test]
fn rule_5_effect_style_ansi_invert_always_valid() {
    // Rule 5: style, ansi and invert are always valid.
    for style in [
        Style::Sketch,
        Style::Dither,
        Style::Halftone,
        Style::Hatch,
        Style::Ascii,
    ] {
        assert_eq!(
            EffectOptions {
                style,
                ansi: true,
                invert: true,
                ..EffectOptions::default()
            }
            .validate(),
            Ok(())
        );
    }
}

#[test]
fn rule_6_parse_threshold_ok_rows() {
    // Worked examples, whole values.
    assert_eq!(parse_threshold("45"), Ok((45.0, 90.0)));
    assert_eq!(parse_threshold(" 20 , 60 "), Ok((20.0, 60.0)));
    assert_eq!(parse_threshold("0"), Ok((0.0, 0.0)));
    assert_eq!(parse_threshold("12.5"), Ok((12.5, 25.0)));
    // Rule 6b: one part doubles; two parts are taken as-is.
    assert_eq!(parse_threshold("100"), Ok((100.0, 200.0)));
    assert_eq!(parse_threshold("20,20"), Ok((20.0, 20.0)));
}

#[test]
fn rule_6_parse_threshold_expected_number() {
    // Rule 6a: more than 2 parts, or a part that is not an f32 (empty included).
    let want = Err(bad(
        "threshold",
        "expected a number like 45 or two like 20,60",
    ));
    for text in ["a", "", "1,2,3", "1,", ",2", "4 5", ",", "1,,2", "1,2,3,4"] {
        assert_eq!(parse_threshold(text), want);
    }
}

#[test]
fn rule_6_parse_threshold_zero_or_more() {
    // Rule 6c: negative, NaN and infinite values fail here (not in 6a).
    let want = Err(bad("threshold", "must be a number of 0 or more"));
    for text in ["-5", "nan", "inf", "1,-1", "-inf", "1,inf", "inf,1", "-nan"] {
        assert_eq!(parse_threshold(text), want);
    }
}

#[test]
fn rule_6_parse_threshold_high_limit() {
    // Rule 6d: high < low fails; high == low is fine (strict comparison).
    assert_eq!(
        parse_threshold("20,10"),
        Err(bad(
            "threshold",
            "the high limit must not be below the low limit"
        ))
    );
    assert_eq!(parse_threshold("10,10"), Ok((10.0, 10.0)));
}

#[test]
fn job_is_plain_data() {
    // Contract: Job must compile, compare with ==, and copy.
    let job = Job::Effect(EffectOptions {
        style: Style::Hatch,
        ..EffectOptions::default()
    });
    let copied = job;
    assert_eq!(job, copied);
    assert_eq!(
        Job::LineArt(LineArtOptions::default()),
        Job::LineArt(LineArtOptions::default())
    );
    assert_eq!(
        Job::Vector(VectorOptions::default()),
        Job::Vector(VectorOptions::default())
    );
    assert_ne!(
        Job::Effect(EffectOptions {
            style: Style::Hatch,
            ..EffectOptions::default()
        }),
        Job::Effect(EffectOptions::default())
    );
}
