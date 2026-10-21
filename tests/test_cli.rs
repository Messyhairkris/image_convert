use photo_tools::cli::{parse, Invocation};
use photo_tools::options::{EffectOptions, Job, LineArtOptions, VectorOptions};

#[test]
fn rule_1_help_mentions_input_and_mode() {
    // --help should exit with code 0 and mention required flags.
    let argv = vec!["photo_tools".to_string(), "--help".to_string()];
    let err = parse(&argv).expect_err("help should return Err");
    assert_eq!(err.code, 0);
    assert!(err.message.contains("--input"));
    assert!(err.message.contains("--mode"));
}

#[test]
fn rule_1_version_contains_name() {
    // --version should exit with code 0 and contain the crate name.
    let argv = vec!["photo_tools".to_string(), "--version".to_string()];
    let err = parse(&argv).expect_err("version should return Err");
    assert_eq!(err.code, 0);
    assert!(err.message.contains("photo_tools"));
}

#[test]
fn rule_2_invalid_flag_for_mode_vector_blur() {
    // Rule 3: first invalid flag (blur) for mode vector triggers error before select/mask check.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b".to_string(),
        "--mode".to_string(),
        "vector".to_string(),
        "--blur".to_string(),
        "2".to_string(),
        "--select".to_string(),
        "s.txt".to_string(),
        "--mask".to_string(),
        "m.txt".to_string(),
    ];
    let err = parse(&argv).expect_err("invalid flag should error");
    assert_eq!(err.code, 2);
    assert_eq!(err.message, "error: --blur does not apply to --mode vector");
}

#[test]
fn rule_3_select_and_mask_mutual_exclusive() {
    // Both select and mask given for a mode where they are allowed should error.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b".to_string(),
        "--mode".to_string(),
        "lineart".to_string(),
        "--select".to_string(),
        "keep.txt".to_string(),
        "--mask".to_string(),
        "mask.txt".to_string(),
    ];
    let err = parse(&argv).expect_err("select+mask should error");
    assert_eq!(err.code, 2);
    assert_eq!(err.message, "error: use --select or --mask, not both");
}

#[test]
fn rule_3a_select_mask_before_validation() {
    // Valid flag (colors) present, but both select and mask cause error before validation.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b".to_string(),
        "--mode".to_string(),
        "vector".to_string(),
        "--colors".to_string(),
        "8".to_string(),
        "--select".to_string(),
        "s.txt".to_string(),
        "--mask".to_string(),
        "m.txt".to_string(),
    ];
    let err = parse(&argv).expect_err("select+mask should error before validation");
    assert_eq!(err.code, 2);
    assert_eq!(err.message, "error: use --select or --mask, not both");
}

#[test]
fn rule_4_defaults_lineart() {
    // Minimal lineart args produce default LineArtOptions.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b.png".to_string(),
        "--mode".to_string(),
        "lineart".to_string(),
    ];
    let inv = parse(&argv).expect("should parse defaults");
    let expected = Invocation {
        input: "a".to_string(),
        output: "b.png".to_string(),
        job: Job::LineArt(LineArtOptions::default()),
        select: None,
        mask: None,
    };
    assert_eq!(inv, expected);
}

#[test]
fn rule_4_defaults_vector() {
    // Minimal vector args produce default VectorOptions.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "v.svg".to_string(),
        "--mode".to_string(),
        "vector".to_string(),
    ];
    let inv = parse(&argv).expect("should parse defaults");
    let expected = Invocation {
        input: "a".to_string(),
        output: "v.svg".to_string(),
        job: Job::Vector(VectorOptions::default()),
        select: None,
        mask: None,
    };
    assert_eq!(inv, expected);
}

#[test]
fn rule_4_defaults_fx() {
    // Minimal fx args produce default EffectOptions.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "e.png".to_string(),
        "--mode".to_string(),
        "fx".to_string(),
    ];
    let inv = parse(&argv).expect("should parse defaults");
    let expected = Invocation {
        input: "a".to_string(),
        output: "e.png".to_string(),
        job: Job::Effect(EffectOptions::default()),
        select: None,
        mask: None,
    };
    assert_eq!(inv, expected);
}

#[test]
fn rule_5_invalid_colors_error() {
    // Vector mode with out-of-range colors triggers validation error.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b.svg".to_string(),
        "--mode".to_string(),
        "vector".to_string(),
        "--colors".to_string(),
        "1".to_string(),
    ];
    let err = parse(&argv).expect_err("invalid colors should error");
    assert_eq!(err.code, 2);
    assert_eq!(err.message, "error: invalid --colors: must be from 2 to 64");
}

#[test]
fn rule_5_invalid_threshold_error() {
    // Lineart mode with high < low triggers threshold validation error.
    let argv = vec![
        "photo_tools".to_string(),
        "--input".to_string(),
        "a".to_string(),
        "--output".to_string(),
        "b.png".to_string(),
        "--mode".to_string(),
        "lineart".to_string(),
        "--threshold".to_string(),
        "20,10".to_string(),
    ];
    let err = parse(&argv).expect_err("invalid threshold should error");
    assert_eq!(err.code, 2);
    assert_eq!(
        err.message,
        "error: invalid --threshold: the high limit must not be below the low limit"
    );
}
