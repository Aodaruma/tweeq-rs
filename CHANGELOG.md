# Changelog

All notable Rust port changes are documented here. The Rust crates follow
Semantic Versioning after the `0.1.0-alpha` preview series.

## [Unreleased]

- Track alpha feedback, including the remaining Size constraint-icon visual
  mismatch and platform-specific pointer behavior.
- Add browser-backed WASM interaction tests and cross-platform native checks.

## [0.1.0-alpha.2]

- WindowsとWeb向けのSlintギャラリーと、外部アプリで組み込めるNumber・Angle・Color部品を追加。
- Number・Angle・Colorの相対操作でポインターのロックに対応。
- 色のチャンネル切り替えと、選択した複数色の相対編集に対応。
- リポジトリを `tweeq-rs` に改名し、Pagesでegui版・Slint版・API文書を公開。

## [0.1.0-alpha.1] - 2026-07-22

- Add the `tweeq-core`, `tweeq-egui`, and `tweeq-demo` Cargo workspace.
- Add typed edit sessions, parameter selection, validation, gesture math, and
  theme tokens.
- Add Number, Rotary, Angle, Boolean, text, choice, vector, time, and color
  parameter widgets.
- Add CubicBezier, deterministic Shuffle, ComplexInput, Ruler, Timeline,
  Viewport2D, pane, code-input, and command-palette prototypes.
- Add an interactive native/WASM-ready component gallery and Rust port design
  records.
- Complete the Phase 5 parity pass for Number, Angle/Rotary, Color, Timecode,
  Boolean, choice, geometry, and advanced input families.
- Replace the Vue/Vite/VuePress application with a Rust-only Cargo workspace,
  a Trunk WASM gallery, rustdoc Pages output, and Rust release automation.
- Preserve the final in-repository Vue reference as `vue-final-reference`.
