# tweeq-egui

Tweeq-style parameter-tuning widgets for egui.

```sh
cargo add tweeq-egui@0.1.0-alpha.1
```

```rust,no_run
use tweeq_egui::{Number, ParamId, TweeqContext};

fn ui(ui: &mut egui::Ui, context: &mut TweeqContext, opacity: &mut f64) {
    Number::new(ParamId::from_static("opacity"), opacity)
        .range(0.0..=1.0)
        .step(0.01)
        .snap(0.1)
        .precision(3)
        .show(ui, context);
}
```

Run `cargo run -p tweeq-demo` from the repository root for the interactive
native gallery. Native and WASM use the same public widget API.

This is an alpha port. Platform pointer-grab behavior and some pixel-level
details differ from the browser reference. See the
[compatibility report](https://github.com/Aodaruma/tweeq-rs/blob/main/docs/rust-port/compatibility-report.md)
for the current contract and known substitutions.

Licensed under MIT. Tweeq was created by Baku Hashimoto; see `NOTICE.md` for
attribution and port status.
