# Tweeq Slint UIのソース配布

アプリの`build.rs`で`tweeq_slint_build::export_ui`を呼び、`OUT_DIR`内へ部品を書き出します。
戻り値を`slint_build::CompilerConfiguration::with_library_paths`で`tweeq`に割り当てると、
`.slint`から`import { TweeqNumber } from "@tweeq";`と指定できます。
入力はアプリが`tweeq-slint`の編集処理へ接続します。
