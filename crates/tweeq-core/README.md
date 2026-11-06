# tweeq-core

Renderer-independent interaction semantics for Tweeq parameter widgets.

```sh
cargo add tweeq-core@0.1.0-alpha.1
```

This crate contains stable parameter IDs, explicit edit sessions, selection,
numeric validation, tweak gesture math, reproducible shuffle values, ruler
intervals, and viewport transforms. It does not depend on egui.

The API is an alpha preview. See the repository's
[compatibility report](https://github.com/Aodaruma/tweeq-rs/blob/main/docs/rust-port/compatibility-report.md)
before adopting it in a persistent project format.

Licensed under MIT. Tweeq was created by Baku Hashimoto; see `NOTICE.md` for
attribution and port status.
