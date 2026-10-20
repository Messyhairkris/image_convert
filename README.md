# photo_tools - non-AI image effects in Rust

A small, private, local toolkit that turns photos (PNG/JPEG) into line art, flat-colour
SVG vectors, pencil sketches, 1-bit dithers for laser engraving, halftone dots,
cross-hatching and ASCII art - with classical image processing only. No neural networks,
no network access.

It is two things in one crate:

- a **command-line tool**, `photo_tools`, and
- a **library** (`photo_tools`) that other Rust programs embed; it never touches files,
  so the host program decides where images come from and where results go.

## Build

Rust stable (1.80 or newer). From this folder:

```
cargo build --release
```

The program is then `target\release\photo_tools.exe`. If this folder is on a network
share, keep the build output on a local disk: set `CARGO_TARGET_DIR` (for example to
`C:\rust-target\photo_tools`) before building.

## Use

```
photo_tools --input photo.jpg --output lineart.png --mode lineart --filter canny --blur 1.2 --threshold 45
photo_tools --input landscape.png --output vector.svg --mode vector --colors 8 --smoothness 1.5
photo_tools --input face.jpg --output sketch.png --mode fx --style sketch --blur 8
photo_tools --input logo.png --output engrave.png --mode fx --style dither
photo_tools --input photo.jpg --output dots.png --mode fx --style halftone --cell 8
photo_tools --input photo.jpg --output hatch.png --mode fx --style hatch --cell 6
photo_tools --input photo.jpg --output - --mode fx --style ascii --columns 100 --ansi
```

`--help` lists every option. Errors go to stderr with exit code 2 (bad arguments) or 1
(cannot read, process or write); nothing is written under the output name unless the
whole file was written.

| mode | options (default) | output |
|---|---|---|
| `lineart` | `--filter canny\|sobel` (canny), `--blur F` Gaussian sigma, 0 = none (1.0), `--threshold T` or `L,H` (50,100; one number T means T,2T; Sobel uses the first) | black lines on white, `.png`/`.jpg` |
| `vector` | `--colors 2..64` (8), `--smoothness 0..10` path tolerance, 0 = straight segments (1.0), `--speckle N` merge regions under N pixels (8), `--iterations 1..100` K-means (20) | `.svg` or `-` for stdout |
| `fx` | `--style sketch\|dither\|halftone\|hatch\|ascii` (sketch), `--blur` sketch blur 0.5..100 (8), `--cell 2..64` halftone cell / hatch spacing (6), `--columns 1..1000` ASCII width (100), `--ansi` 24-bit colour, `--invert` for dark terminals | `.png`/`.jpg`, or text for ascii (`.txt` or `-`) |

## How each engine works

- **Line art**: gray (Rec. 601), optional Gaussian blur, then either a Sobel gradient
  magnitude with a threshold, or imageproc's Canny detector (non-maximum suppression and
  hysteresis, 1-pixel lines), inverted to black on white.
- **Vector**: deterministic K-means over 5-bit colour bins (farthest-first start, no
  randomness, so the same photo always gives the same SVG), small regions merged into
  their neighbours, region outlines traced along pixel edges, simplified with
  Ramer-Douglas-Peucker and fitted with Catmull-Rom cubic Beziers. Shared borders are
  simplified identically from both sides, so neighbouring colours meet without gaps.
- **Sketch**: gray, inverted, heavily blurred, Color-Dodge blended back.
- **Dither**: Floyd-Steinberg error diffusion to pure black and white.
- **Halftone**: one dot per cell, its area following the cell's darkness.
- **Hatch**: up to four layers of lines; darker pixels collect more layers.
- **ASCII**: cell brightness mapped onto ` .:-=+*#%@`, cells twice as tall as wide.

## Embedding in another Rust program

```toml
[dependencies]
photo_tools = { path = "../image_convert", default-features = false }   # no clap
```

```rust
use photo_tools::{process, image, Job, Output, VectorOptions, EffectOptions, Style};

let img = image::open("photo.jpg")?;                       // or any DynamicImage you already have
let svg = match process(&img, &Job::Vector(VectorOptions { colors: 6, ..Default::default() }))? {
    Output::Svg(s) => s,
    _ => unreachable!(),
};
let job = Job::Effect(EffectOptions { style: Style::Dither, ..Default::default() });
if let Output::Image(gray) = process(&img, &job)? { gray.save("engrave.png")?; }
```

Every engine is also callable on its own (`photo_tools::lineart::render`,
`vector::render`, `effects::render`), and the building blocks work on plain byte slices
(`sobel::edge_mask`, `dither::floyd_steinberg`, `kmeans::quantize`, `contour::trace`,
...). All options validate themselves (`LineArtOptions::validate()` etc.) and every
failure is a `photo_tools::Error` with a readable message.

## Tests

`cargo test` runs the specification tests (`tests/hidden_*.rs`, written first, against
which every module was accepted) and the independent tests the crew wrote from the
contracts alone (`tests/test_*.rs`).

## How it was built

Planned by the PA (Claude): contracts, reference implementation and specification tests
for every module. Each module was then written by a member of the ATLAS crew (small
open models) and accepted only when it passed tests it never saw; another member wrote
independent tests from the contract alone. Who wrote what, and how many attempts it
took, is in `CREW_RECORD.md`.
