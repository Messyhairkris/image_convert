# photo_tools - non-AI image effects in Rust

A small, private, local toolkit that turns photos (PNG/JPEG) into line art, flat-colour
SVG vectors, pencil sketches, 1-bit dithers for laser engraving, halftone dots,
cross-hatching and ASCII art - with classical image processing only. No neural networks,
no network access.

You can render the whole photo, or only the areas you pick: select them with a
**lasso**, a **polyline** or a **blob brush** (Shift adds, Ctrl removes), and everything
outside is left out - transparent in PNG and SVG, white in JPEG. After rendering, the
**eraser** removes any part of the result you do not want. Line art can be saved as
**SVG centerlines** (one thin stroke down the middle of each line) as well as PNG/JPEG.

It is three things in one crate:

- an **editor window**, `photo_tools_gui`, for picking areas and erasing by hand;
- a **command-line tool**, `photo_tools`, for scripts and batches; and
- a **library** (`photo_tools`) that other Rust programs embed; it never touches files,
  so the host program decides where images come from and where results go. The
  selection tools themselves are library code too (`photo_tools::editor`), so a host
  program can offer the same lasso / polyline / brush in its own UI.

## Build

Rust stable (1.95 or newer). From this folder:

```
cargo build --release                    # the command-line tool
cargo build --release --features gui     # also the editor window
```

The programs are then `target\release\photo_tools.exe` and
`target\release\photo_tools_gui.exe`. If this folder is on a network share, keep the
build output on a local disk: set `CARGO_TARGET_DIR` (for example to
`C:\rust-target\photo_tools`) before building.

## The editor window

`photo_tools_gui [photo.jpg]`, or drop a photo on the window.

- **Lasso (L)**: drag around an area. **Polyline (P)**: click corner points; Enter, a
  double-click, or a click on the first point closes it (Backspace removes a point, Esc
  cancels). **Blob brush (B)**: paint the area; `[` and `]` change the size.
- No key replaces the selection, **Shift** adds to it, **Ctrl** removes from it. With
  nothing selected the whole photo is used; Ctrl+D goes back to the whole photo.
- Choose a mode on the left (line art, vector, or an effect) - the result renders as you
  change things. **V** flips between the photo and the result. **Auto** picks settings
  for this photo (inside the selection, if any); **Reset** goes back to the defaults.
- **Eraser (E)**: paint over the result to remove parts of it.
- Ctrl+Z / Ctrl+Y undo and redo every selection and erase step. Wheel zooms, middle drag
  or Space+drag pans, F fits the photo to the window.
- **Export...** writes PNG (transparent outside), JPEG, SVG (line art and vector) or text
  (ASCII). **Save selection...** writes the selection and eraser strokes as a small text
  file that the command line can use with `--select`.

## Use

```
photo_tools --input photo.jpg --output lineart.png --mode lineart --filter canny --blur 1.2 --threshold 45
photo_tools --input landscape.png --output vector.svg --mode vector --colors 8 --smoothness 1.5
photo_tools --input face.jpg --output sketch.png --mode fx --style sketch --blur 8
photo_tools --input logo.png --output engrave.png --mode fx --style dither
photo_tools --input photo.jpg --output dots.png --mode fx --style halftone --cell 8
photo_tools --input photo.jpg --output hatch.png --mode fx --style hatch --cell 6
photo_tools --input photo.jpg --output - --mode fx --style ascii --columns 100 --ansi
photo_tools --input photo.jpg --output lines.svg --mode lineart --select photo_selection.txt
photo_tools --input photo.jpg --output subject.png --mode fx --style hatch --mask subject_mask.png
```

`--select FILE` renders only a selection saved by the editor (its eraser strokes are
removed too). `--mask FILE` does the same with a mask image of the same size: gray 128
and above is kept. Line art written to `.svg` is traced into centerline strokes.

`--help` lists every option. Errors go to stderr with exit code 2 (bad arguments) or 1
(cannot read, process or write); nothing is written under the output name unless the
whole file was written.

| mode | options (default) | output |
|---|---|---|
| `lineart` | `--filter canny\|sobel` (canny), `--blur F` Gaussian sigma, 0 = none (1.0), `--threshold T` or `L,H` (50,100; one number T means T,2T; Sobel uses the first) | black lines on white, `.png`/`.jpg`, or `.svg` centerlines |
| every mode | `--select FILE` or `--mask FILE` | only that area; outside transparent (PNG, SVG) or white (JPEG) |
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
- **Selections**: lasso and polyline are polygons filled by the nonzero winding rule (a
  scribbled lasso never leaves holes); the blob brush selects every pixel within its
  radius of the stroke. Replace / add / remove apply in order; the eraser removes from
  whatever was selected.
- **SVG centerlines**: the line art inside the selection is thinned to one-pixel lines
  (Guo-Hall), traced into chains between line ends and forks, simplified (RDP) and
  drawn as Catmull-Rom curves with round caps; no background, so it is transparent.
- **Vector of a selection**: the palette is fitted to the selected pixels only, so the
  colours are spent on what you picked; nothing outside is drawn.

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

Selections, from shapes or from your own UI's pointer events:

```rust
use photo_tools::editor::{Editor, Event, Mods, Tool};
use photo_tools::{keep_mask, process_masked, Op, Shape};

// Shapes you already have (image pixel coordinates):
let ops = vec![
    (Op::Replace, Shape::Polygon(vec![[100.0, 80.0], [400.0, 90.0], [380.0, 300.0]])),
    (Op::Subtract, Shape::Stroke { points: vec![[200.0, 150.0], [260.0, 170.0]], radius: 12.0 }),
];
let keep = keep_mask(img.width(), img.height(), &ops, &[]);
let out = process_masked(&img, &job, keep.as_ref())?;   // Output::Layer = gray + alpha

// Or drive the editor with your UI's events and ask it for the mask:
let mut ed = Editor::new();
ed.set_tool(Tool::Brush);
ed.handle(Event::Press { at: [50.0, 60.0], mods: Mods { shift: true, ctrl: false } });
ed.handle(Event::Drag { at: [90.0, 64.0] });
ed.handle(Event::Release { at: [120.0, 70.0] });
let keep = keep_mask(img.width(), img.height(), ed.selection(), ed.erased());
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
