# Research and design background

Tweeq was created by [Baku Hashimoto](https://baku89.com/) and developed with
[Jun Kato](https://junkato.jp/) as a family of parameter-tuning GUI widgets for
creative professionals.

- Paper: [Tweeq: Parameter-Tuning GUI Widgets by/for Creative Professionals](https://doi.org/10.1145/3746059.3747723)
- Venue: ACM UIST 2025
- Original browser gallery: [baku89.github.io/tweeq](https://baku89.github.io/tweeq/)
- Original source: [baku89/tweeq](https://github.com/baku89/tweeq)
- Final Vue tree formerly bundled here: [`vue-final-reference`](https://github.com/Aodaruma/tweeq-rs/tree/vue-final-reference)

## Interaction principles retained by the Rust port

Tweeq treats input widgets as compact, expert-oriented adjustment surfaces.
The Rust port retains these common semantics where the host platform permits:

- dragging right or up increases a scalar value;
- vertical drag distance changes Number adjustment sensitivity;
- `Shift` accelerates and `Alt`/`Option` enables finer adjustment;
- `Q` selects a component-specific snap interval;
- `A` and `R` select absolute and relative Rotary behavior;
- vector controls accept axis constraints;
- `Ctrl`/`Command` and `Shift` extend parameter selection;
- each gesture has explicit begin, update, commit, and cancel boundaries.

The browser implementation used Pointer Lock, DOM overlays, CSS, Pinia, and
JavaScript expressions. The Rust port instead uses egui responses and
foreground painters, stable parameter IDs, typed edit events, and documented
pointer fallbacks. JavaScript `eval` is intentionally not part of the library.

## Original gesture reference

The following behavior remains the comparison baseline rather than a claim of
pixel-identical output on every platform:

| Input | Reference interaction |
|---|---|
| Number | Click to edit; arrows adjust while focused; vertical drag changes sensitivity; `Shift`, `Alt`, and `Q` modify the gesture |
| Rotary / Angle | Drag the indicator for absolute mode or the knob for relative mode; `A`, `R`, `Shift`, and `Q` change the mode or snap |
| Color | Click for the picker; drag for relative HSV/RGB/alpha editing selected by modifier keys |
| Boolean | Click to toggle; swipe left/right to set the value |
| Position | Two-dimensional drag with fast/fine modifiers and X/Y constraints |
| Timecode | Text edit, contextual unit selection, and drag by hours/minutes/seconds/frames |

## Study artifacts and archival policy

The original site included interactive examples for drop-shadow, spring,
timecode, and three-point-lighting tasks used in an informal expert study.
Those Vue-bound pages are not runtime dependencies of the Rust library. Their
exact source and assets remain recoverable from `vue-final-reference`, while
the paper and DOI are the stable scholarly record.

The Rust gallery is now the executable documentation. Port decisions,
substitutions, and known differences are recorded under [`docs/rust-port`](rust-port/README.md).
