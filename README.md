<div align="center">

<img src="./assets/logo.svg" width="200" alt="Tweeq logo" />
<h1>Tweeq for Rust</h1>

<a href="https://aodaruma.github.io/tweeq-rs/">eguiギャラリー</a> ⌇ <a href="https://aodaruma.github.io/tweeq-rs/slint/">Slintギャラリー</a> ⌇ <a href="https://aodaruma.github.io/tweeq-rs/api/">APIドキュメント</a> ⌇ <a href="https://github.com/sponsors/baku89">Become a Sponsor</a>

</div>

[![crates.io](https://img.shields.io/crates/v/tweeq-egui.svg)](https://crates.io/crates/tweeq-egui)
[![docs.rs](https://docs.rs/tweeq-egui/badge.svg)](https://docs.rs/tweeq-egui)
[![CI](https://github.com/Aodaruma/tweeq-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Aodaruma/tweeq-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> [!NOTE]
> For a live demo of [the UIST paper](https://dl.acm.org/doi/10.1145/3746059.3747723), see [the original Tweeq paper page](https://baku89.github.io/tweeq/uist2025.html).

Tweeqのパラメーター調整部品をRustアプリで利用できます。eguiとSlintのUIが
ジェスチャー計算と編集イベントを共有し、取消・Undo・複数選択の操作を提供します。

> `0.1.0-alpha.1` is an API preview. The public API and visual details may
> change before `0.1.0`.

## Install

```sh
cargo add tweeq-egui@0.1.0-alpha.1
```

The crate requires Rust 1.92 or newer. Applications that only need the
renderer-independent edit and gesture model can depend on
[`tweeq-core`](https://crates.io/crates/tweeq-core) directly.

```rust,no_run
use tweeq_egui::{Number, ParamId, TweeqContext};

fn parameter_ui(ui: &mut egui::Ui, tweeq: &mut TweeqContext, opacity: &mut f64) {
    Number::new(ParamId::from_static("opacity"), opacity)
        .range(0.0..=1.0)
        .step(0.01)
        .snap(0.1)
        .precision(3)
        .show(ui, tweeq);
}
```

The host application drains typed `Begin`, `Update`, `Commit`, and `Cancel`
events from `TweeqContext`. This keeps model mutation and undo ownership outside
the widgets.

## Demo and documentation

- Run the native component gallery: `cargo run -p tweeq-demo`
- Run the web gallery: `cd crates/tweeq-demo && trunk serve index.html --open`
- [eguiのWebギャラリー](https://aodaruma.github.io/tweeq-rs/)
- [SlintのWebギャラリー](https://aodaruma.github.io/tweeq-rs/slint/)
- [SlintのWindowsデモと利用方法](crates/tweeq-slint-demo/README.md)
- [API documentation](https://docs.rs/tweeq-egui)
- [Architecture and port records](docs/rust-port/README.md)
- [Slint対応とtweeq-rsへの移行方針](docs/slint-support-plan.md)
- [Research and design background](docs/research.md)

The gallery contains every implemented control and keeps its configuration
inside the component canvas so native and WASM behavior can be compared with
the same public API.

## Workspace

| Crate | Purpose | Distribution |
|---|---|---|
| [`tweeq-core`](https://crates.io/crates/tweeq-core) | Renderer-independent IDs, edit sessions, selection, gestures, validation, and transforms | crates.io |
| [`tweeq-egui`](https://crates.io/crates/tweeq-egui) | egui widgets, theme, overlays, pointer policy, and adapters | crates.io |
| `tweeq-demo` | Native/WASM interactive component gallery | Repository only |
| [`tweeq-slint`](crates/tweeq-slint/README.md) | Slintの編集処理とRust API | ソースで提供 |
| [`tweeq-slint-build`](crates/tweeq-slint-build/README.md) | アプリが読み込むSlint部品のソース | ソースで提供 |
| `tweeq-slint-demo` | Windows・WebのSlintギャラリー | デモ |

The library crate depends only on `egui` and `tweeq-core`; `eframe` is confined
to the demo. Native and WASM builds expose the same widget API.

## Slint α版

Number、Angle、ColorをWindowsとWebで試せます。ギャラリーの
「Unlimited relative drag」を有効にすると、相対操作でポインターをロックできます。

色見本をクリックするとピッカーが開きます。CtrlまたはShiftを押しながらクリックして
複数の色を選び、選択した色見本をドラッグすると、各色の開始値に同じ変化量を適用します。
ドラッグ中は次のキーで調整先を切り替えられます。

| キー | 調整先 |
|---|---|
| なし | 横で彩度、縦で明度 |
| H、F、Shift | 色相 |
| S、V | 彩度、明度 |
| R、G、B | 赤、緑、青 |
| A、Alt | 不透明度 |

Escで取り消せます。確定した複数色の変更はUndoでまとめて戻せます。
外部アプリへの組み込みは[パッケージからビルドする利用例](tests/slint-consumer)を参照してください。

## egui alpha compatibility

Number, Angle/Rotary, Boolean, text, choice, vector/geometry, time, color, and
advanced input families are implemented. Platform pointer-grab behavior and a
small number of pixel-level details still differ from the browser reference.
In particular, the Size constraint icon remains a known visual follow-up.
See the [compatibility report](docs/rust-port/compatibility-report.md) for the
current contract and documented substitutions.

## Original Tweeq and attribution

[Tweeq](https://baku89.github.io/tweeq/) was created by Baku Hashimoto and
developed with Jun Kato as parameter-tuning GUI research for creative
professionals. This Rust port is distributed under the same MIT license and
retains the original copyright notice.

The final Vue/TypeScript tree before the Rust-only switch is preserved by the
annotated Git tag [`vue-final-reference`](https://github.com/Aodaruma/tweeq-rs/tree/vue-final-reference).
The upstream browser implementation remains available at
[baku89/tweeq](https://github.com/baku89/tweeq).

If this work supports academic research, see [`CITATION.cff`](CITATION.cff) and
the [UIST 2025 paper](https://doi.org/10.1145/3746059.3747723).

## License

MIT. See [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md).
