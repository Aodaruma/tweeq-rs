# Tweeq egui gallery

Run the native gallery from the repository root:

```sh
cargo run -p tweeq-demo
```

The same `GalleryApp` runs in a browser without Node.js:

```sh
cd crates/tweeq-demo
trunk serve index.html --open
```

`index.html` starts the exported `WebHandle` on `wasm32-unknown-unknown`. CI
checks the target, and the Pages workflow bundles it with Trunk 0.21.14.

The gallery contains interactive examples for Number, Rotary, Angle, Boolean,
String, choice, Position, Translate, Vector, Size, Timecode, Drum, and Color
parameters. Number supports click-to-edit, drag-to-tweak, vertical sensitivity,
Shift/Alt/Q modifiers, explicit commit/cancel events, multi-select, and an
optional pointer-lock request for unbounded parameters. Expand “InputNumber
parity controls” to exercise bar origin, min/max, step, clamp, precision,
disabled, and invalid states. Angle exposes snap and angle-offset controls.

Phase 4 examples add CubicBezier, deterministic Shuffle, ComplexInput, tabs,
rulers, a timeline scrubber, a pan/zoom viewport, floating/collapsing panes,
a lightweight code input, and a host-owned command palette.

This gallery exercises the shared API and edit lifecycle. Platform and visual
differences are tracked in the repository's
[component parity audit](https://github.com/Aodaruma/tweeq-rs/blob/main/docs/rust-port/component-parity-audit.md).
