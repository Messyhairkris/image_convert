# Crew record - who built photo_tools, and how

20 October 2026. Every module in `src/` except `lib.rs`, `main.rs` and `gui.rs` was
written by a member of the ATLAS crew (small open models), and accepted only when it
passed specification tests it never saw (`tests/hidden_*.rs`). Another member then wrote
independent tests from the contract alone (`tests/test_*.rs`), and those had to pass
against the accepted module. The PA (Claude) wrote every contract, a reference
implementation of every module, the specification tests, and the I/O shells (`main.rs`,
the editor window `gui.rs`); the references were checked against the specification tests
and against seeded defects (mutation testing) before any member started.

## In four waves

1. **Pure algorithms** (std only): luma, dodge, hatch, dither, sobel, halftone,
   despeckle, k-means, junction, edges, contour (split into three jobs after the whole
   job hit the provider's 10-minute limit), smooth, svg, ascii, options.
2. **Crate-facing** (image, imageproc, clap): lineart, effects, vector, cli.
3. **Selections** (Kris's request the same day): select (lasso / polyline / brush masks),
   thin (Guo-Hall thinning), strokes (centerline tracing).
4. **Selections, assembled**: composite (transparent outside), shapes_file (saved
   selections), editor (the tool state machine), lineart_svg (SVG centerlines),
   vector_select (palette from the selection only), and a cli revision for `--select`
   and `--mask`.

## Reassignments, and why

- A member whose provider endpoint stopped answering had the job moved, and the
  stopped job is not counted against them: Larry (GLM-4.7-Flash) twice, Qwen
  (Qwen3.8-27B) on four jobs between 12:22 and 14:52, Barry once (three 504s).
- Nemotron's lineart tests failed nine attempts on his own Rust mistakes; Kimi then
  wrote them in one request.
- Hoss's cli tests and Nemotron's lineart tests each first failed partly through a
  contract gap or harness bug; those were fixed and the jobs restarted.

## Every job
| member | role | scope | task | state | attempts | requests | tokens | min | first failure |
|---|---|---|---|---|---|---|---|---|---|
| Barry | author | M | pt_despeckle | accepted | 1 | 1 | 19979 | 4.9 |  |
| Barry | author | M | pt_halftone | accepted | 1 | 1 | 8194 | 6.9 |  |
| Barry | tests | C | pt_effects-tests | accepted | 1 | 4 | 84845 | 27.2 |  |
| Barry | tests | M | pt_shapes_file-tests | accepted | 1 | 1 | 32810 | 7.4 |  |
| Barry | tests | M | pt_smooth-tests | stopped (provider) | 3 | - | - | - |  |
| Barry | tests | M | pt_svg-tests | accepted | 1 | 1 | 26562 | 6.6 |  |
| Gordon | author | C | pt_vector | accepted | 1 | 1 | 16868 | 0.9 |  |
| Gordon | author | C | pt_vector_select | accepted | 1 | 1 | 22431 | 6.8 |  |
| Gordon | author | L | pt_strokes | accepted | 1 | 6 | 181073 | 30.1 |  |
| Gordon | author | M | pt_kmeans | accepted | 1 | 4 | 60314 | 7.9 | unknown |
| Gordon | author | M | pt_smooth | accepted | 1 | 6 | 100655 | 17.2 |  |
| Gordon | tests | C | pt_lineart_svg-tests | accepted | 1 | 5 | 147944 | 13.4 |  |
| Gordon | tests | L | pt_editor-tests | accepted | 1 | 4 | 98748 | 12.6 |  |
| Gordon | tests | L | pt_select-tests | accepted | 1 | 1 | 34753 | 6.5 |  |
| Gordon | tests | M | pt_contour-tests | accepted | 1 | 3 | 62791 | 12.8 |  |
| Gordon | tests | M | pt_halftone-tests | accepted | 1 | 1 | 12137 | 2.3 |  |
| Gordon | tests | S | pt_hatch-tests | accepted | 1 | 1 | 10815 | 2.2 |  |
| Gordon | tests | S | pt_junction-tests | accepted | 1 | 1 | 13963 | 4.4 |  |
| Hoss | author | C | pt_lineart | accepted | 1 | 1 | 8369 | 0.5 |  |
| Hoss | author | M | pt_shapes_file | accepted | 1 | 3 | 33295 | 7.6 |  |
| Hoss | author | M | pt_svg | accepted | 1 | 1 | 5926 | 0.5 |  |
| Hoss | author | S | pt_dither | accepted | 2 | 1 | 4223 | 5.5 | unknown |
| Hoss | tests | C | pt_cli-tests | accepted | 1 | 2 | 24932 | 1.7 |  |
| Hoss | tests | C | pt_cli-tests | accepted | 2 | 8 | 60749 | 1.7 | rust gate: tests_failed: the tests fail against the real module: |
| Hoss | tests | L | pt_strokes-tests | accepted | 1 | 3 | 21091 | 1.6 |  |
| Hoss | tests | M | pt_ascii-tests | accepted | 1 | 1 | 5731 | 1.0 |  |
| Hoss | tests | M | pt_edges-tests | accepted | 1 | 1 | 6029 | 0.5 |  |
| Hoss | tests | S | pt_composite-tests | accepted | 1 | 1 | 8447 | 0.6 |  |
| Hoss | tests | S | pt_luma-tests | accepted | 1 | 2 | 9614 | 2.2 |  |
| Kimi | author | C | pt_effects | accepted | 1 | 1 | 9306 | 0.5 |  |
| Kimi | author | L | pt_editor | accepted | 1 | 1 | 10235 | 7.0 |  |
| Kimi | author | L | pt_select | accepted | 1 | 2 | 21175 | 10.3 |  |
| Kimi | author | S | pt_composite | accepted | 1 | 1 | 7356 | 0.7 |  |
| Kimi | author | S | pt_hatch | accepted | 1 | 1 | 4061 | 3.7 |  |
| Kimi | tests | C | pt_lineart-tests | accepted | 1 | 1 | 20654 | 10.1 |  |
| Kimi | tests | C | pt_vector-tests | accepted | 1 | 1 | 21765 | 25.1 |  |
| Kimi | tests | C | pt_vector_select-tests | accepted | 1 | 1 | 18245 | 1.3 |  |
| Kimi | tests | M | pt_despeckle-tests | accepted | 1 | 1 | 4458 | 3.4 |  |
| Kimi | tests | M | pt_smooth-tests | accepted | 1 | 2 | 37707 | 13.3 |  |
| Kimi | tests | M | pt_thin-tests | accepted | 1 | 1 | 4861 | 0.8 |  |
| Kimi | tests | S | pt_sobel-tests | accepted | 1 | 1 | 5368 | 3.3 |  |
| Larry | author | S | pt_dodge | accepted | 1 | 2 | 9933 | 1.6 |  |
| Larry | author | S | pt_luma | accepted | 1 | 1 | 4933 | 5.4 |  |
| Larry | tests | M | pt_despeckle-tests | stopped (provider) | 2 | - | - | - |  |
| Larry | tests | S | pt_sobel-tests | stopped (provider) | 2 | - | - | - |  |
| Nemotron | author | C | pt_cli | accepted | 1 | 1 | 13245 | 6.6 |  |
| Nemotron | author | M | pt_options | accepted | 1 | 1 | 6722 | 0.8 |  |
| Nemotron | author | M | pt_thin | accepted | 1 | 1 | 7835 | 3.4 |  |
| Nemotron | author | S | pt_sobel | accepted | 1 | 1 | 5901 | 5.2 |  |
| Nemotron | tests | C | pt_lineart-tests | reassigned | 9 | - | - | - | rust gate: tests_failed: the tests fail against the real module: |
| Nemotron | tests | S | pt_dodge-tests | accepted | 1 | 2 | 18189 | 2.6 |  |
| Qwen | author | C | pt_cli | accepted | 1 | 2 | 43742 | 15.0 |  |
| Qwen | author | C | pt_lineart_svg | accepted | 1 | 1 | 45308 | 8.1 |  |
| Qwen | author | M | pt_ascii | accepted | 1 | 1 | 14595 | 7.7 |  |
| Qwen | author | M | pt_contour | accepted | 1 | 1 | 37413 | 37.7 |  |
| Qwen | author | M | pt_edges | accepted | 1 | 4 | 73847 | 18.0 |  |
| Qwen | author | S | pt_junction | accepted | 1 | 3 | 28274 | 4.9 |  |
| Qwen | tests | C | pt_lineart-tests | stopped (provider) | 3 | - | - | - |  |
| Qwen | tests | L | pt_editor-tests | stopped (provider) | 1 | - | - | - |  |
| Qwen | tests | L | pt_strokes-tests | stopped (provider) | 1 | - | - | - |  |
| Qwen | tests | M | pt_kmeans-tests | accepted | 1 | 1 | 31274 | 30.6 |  |
| Qwen | tests | M | pt_options-tests | accepted | 1 | 4 | 83218 | 63.6 |  |
| Qwen | tests | S | pt_dither-tests | accepted | 1 | 1 | 17386 | 4.0 | rust gate: compile_failed: error: linking with `link.exe` failed: exit code: 1104 |

| member | accepted / jobs | first try | requests | by scope (accepted/jobs) |
|---|---|---|---|---|
| Barry | 5/5 | 5 | 8 | Ct 1/1, M 2/2, Mt 2/2 |
| Gordon | 12/12 | 12 | 34 | C 2/2, Ct 1/1, L 1/1, Lt 2/2, M 2/2, Mt 2/2, St 2/2 |
| Hoss | 11/11 | 9 | 24 | C 1/1, Ct 2/2, Lt 1/1, M 2/2, Mt 2/2, S 1/1, St 2/2 |
| Kimi | 12/12 | 12 | 14 | C 1/1, Ct 3/3, L 2/2, Mt 3/3, S 2/2, St 1/1 |
| Larry | 2/2 | 2 | 3 | S 2/2 |
| Nemotron | 5/6 | 5 | 6 | C 1/1, Ct 0/1, M 2/2, S 1/1, St 1/1 |
| Qwen | 9/9 | 9 | 18 | C 2/2, M 3/3, Mt 2/2, S 1/1, St 1/1 |

Scope keys: S small, M exact or byte-exact, L largest algorithm, C crate-facing with the API card; a trailing t means the member wrote the tests for it.

