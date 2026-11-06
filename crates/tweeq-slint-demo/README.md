# Slintギャラリー

Windowsではリポジトリのルートで`cargo run -p tweeq-slint-demo`を実行します。
Webではこのディレクトリで`trunk serve index.html --locked`を実行し、表示されたURLを開きます。
Slint 1.18.1、Rust 1.92以降、Webビルドには`wasm32-unknown-unknown`が必要です。

Numberはクリックで文字編集、ドラッグで値調整、CtrlクリックまたはShiftクリックで複数選択できます。
ドラッグ中にShiftで加速、Altで微調整、Qでスナップします。Escapeで取り消します。

Angleはダイヤルの中心を回るようにドラッグすると回転します。Qで45°刻みに調整できます。
Colorは色見本からポップアップを開き、色相、彩度、明度、透明度を操作できます。
HEXとRGBA・HSVA欄はEnterで確定します。チャンネルの入力範囲は0〜1です。
