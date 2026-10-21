use photo_tools::dodge::{color_dodge, dodge_layers};

#[test]
fn test_rule_1_base_zero_blend_255() {
    assert_eq!(color_dodge(0, 255), 0);
}

#[test]
fn test_rule_1_base_zero_blend_0() {
    assert_eq!(color_dodge(0, 0), 0);
}

#[test]
fn test_rule_1_base_zero_blend_other() {
    assert_eq!(color_dodge(0, 100), 0);
}

#[test]
fn test_rule_2_blend_255_base_10() {
    assert_eq!(color_dodge(10, 255), 255);
}

#[test]
fn test_rule_2_blend_255_base_200() {
    assert_eq!(color_dodge(200, 255), 255);
}

#[test]
fn test_rule_3_capping_128_128() {
    assert_eq!(color_dodge(128, 128), 255);
}

#[test]
fn test_rule_3_general_100_100() {
    assert_eq!(color_dodge(100, 100), 165);
}

#[test]
fn test_rule_3_general_200_0() {
    assert_eq!(color_dodge(200, 0), 200);
}

#[test]
fn test_rule_3_general_1_254() {
    assert_eq!(color_dodge(1, 254), 255);
}

#[test]
fn test_rule_3_general_50_200() {
    assert_eq!(color_dodge(50, 200), 232);
}

#[test]
fn test_rule_3_general_77_3() {
    assert_eq!(color_dodge(77, 3), 78);
}

#[test]
fn test_rule_3_rounding_1_253() {
    assert_eq!(color_dodge(1, 253), 128);
}

#[test]
fn test_rule_3_general_255_0() {
    assert_eq!(color_dodge(255, 0), 255);
}

#[test]
fn test_rule_4_dodge_layers_example1() {
    assert_eq!(dodge_layers(&[0, 100, 200], &[255, 100]), vec![0, 165]);
}

#[test]
fn test_rule_4_dodge_layers_example2() {
    assert_eq!(dodge_layers(&[10], &[255, 1, 2]), vec![255]);
}

#[test]
fn test_rule_4_dodge_layers_example3() {
    assert_eq!(dodge_layers(&[], &[1]), vec![]);
}

#[test]
fn test_rule_4_dodge_layers_lengths() {
    assert_eq!(dodge_layers(&[200, 77], &[0, 3]), vec![200, 78]);
}
