//! Integration tests for the `thin` module (Guo-Hall thinning), from its contract.

use photo_tools::thin::thin;

/// Build (`fg`, width, height) from rows of `#`/`.` separated by `/`.
fn parse(rows: &str) -> (Vec<bool>, usize, usize) {
    let parts: Vec<&str> = rows.split(" / ").collect();
    let height = parts.len();
    let width = parts[0].len();
    let mut fg = Vec::with_capacity(width * height);
    for part in &parts {
        for c in part.chars() {
            fg.push(c == '#');
        }
    }
    (fg, width, height)
}

/// Run `thin` on the picture `input` and compare with the picture `want`.
fn check(input: &str, want: &str) {
    let (fg, width, height) = parse(input);
    let (expect, w2, h2) = parse(want);
    assert_eq!((width, height), (w2, h2));
    assert_eq!(thin(&fg, width, height), expect);
}

/// Rule 1/2: a lone pixel has no neighbour pair, so `C == 0` and it is kept.
#[test]
fn rule_2_lone_pixel_has_c_zero_and_stays() {
    check("... / .#. / ...", "... / .#. / ...");
}

/// Rules 2-3: the 2 x 2 square sheds three pixels judged against the full square,
/// leaving the top-right pixel (the walk in the contract).
#[test]
fn rule_3_two_by_two_square_marks_three_at_once() {
    check("## / ##", ".# / ..");
}

/// Rules 2-3: a 3 x 3 block thins to its centre pixel.
#[test]
fn rule_2_three_by_three_block_thins_to_centre() {
    check("### / ### / ###", "... / .#. / ...");
}

/// Rule 4: a 5 x 5 block peels over iterations down to a 1 x 3 segment.
#[test]
fn rule_4_five_by_five_block_peels_to_segment() {
    check(
        "##### / ##### / ##### / ##### / #####",
        "..... / ..... / ..#.. / ..... / .....",
    );
}

/// Rule 4: the corner pixel goes while the L stays connected (diagonally).
#[test]
fn rule_4_corner_removed_line_stays_connected() {
    check("#.. / #.. / ###", "#.. / #.. / .##");
}

/// Rule 3: the staircase only reaches the contract's diagonal when sub-iteration 0
/// judges every pixel against the start-of-sub-iteration image (worked row 6).
#[test]
fn rule_3_staircase_thins_to_diagonal() {
    check(
        "##.... / .##... / ..##.. / ...##. / ....##",
        "##.... / ..#... / ...#.. / ....#. / .....#",
    );
}

/// Rule 2: the plus-shaped cross (worked row 7), with both `m` variants in play.
#[test]
fn rule_2_cross_thins_to_horizontal_stroke() {
    check(
        "..##.. / ..##.. / ###### / ###### / ..##.. / ..##..",
        "...#.. / ...#.. / .##### / ...... / ...... / ......",
    );
}

/// Rule 4: a ring stays a ring; thinning never breaks a line apart.
#[test]
fn rule_4_ring_stays_a_ring() {
    let ring = "##### / ##### / ##.## / ##### / #####";
    check(ring, "..... / ..#.. / .#.#. / ..#.. / .....");
}

/// Rules 2-3: two separate pieces thin independently (worked row 9); the isolated
/// pixel at the right edge has `C == 0` and is kept.
#[test]
fn rule_2_two_pieces_thin_independently() {
    check("##..# / ##... / .....", ".#..# / ..... / .....");
}

/// Rule 4/5: thinning a thinned result changes nothing (stable skeleton).
#[test]
fn rule_4_thinning_twice_changes_nothing() {
    for picture in [
        "### / ### / ###",
        "##.... / .##... / ..##.. / ...##. / ....##",
        "..##.. / ..##.. / ###### / ###### / ..##.. / ..##..",
        "..... / ..#.. / .#.#. / ..#.. / .....",
    ] {
        let (fg, width, height) = parse(picture);
        let once = thin(&fg, width, height);
        let twice = thin(&once, width, height);
        assert_eq!(once, twice);
    }
}

/// Rule 5: an empty image stays empty and nothing is added.
#[test]
fn rule_5_empty_image_returns_empty() {
    assert_eq!(thin(&[], 0, 0), Vec::<bool>::new());
}

/// Rule 5: background pixels are never turned on, even around a line.
#[test]
fn rule_5_background_is_never_added() {
    let (fg, width, height) = parse("#.. / #.. / ###");
    let out = thin(&fg, width, height);
    for (before, after) in fg.iter().zip(out.iter()) {
        if !before {
            assert!(!after);
        }
    }
    assert_eq!(out.len(), fg.len());
}
