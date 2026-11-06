# Slintギャラリー

Windowsではリポジトリのルートで`cargo run -p tweeq-slint-demo`を実行します。
Webではこのディレクトリで`trunk serve index.html --locked`を実行し、表示されたURLを開きます。
Slint 1.18.1、Rust 1.92以降、Webビルドには`wasm32-unknown-unknown`が必要です。

Numberはクリックで文字編集、ドラッグで値調整、CtrlクリックまたはShiftクリックで複数選択できます。
ドラッグ中にShiftで加速、Altで微調整、Qでスナップします。Escapeで取り消します。

Angleはダイヤルの中心を回るようにドラッグすると回転します。Qで45°刻みに調整できます。
Colorは色見本からポップアップを開き、色相、彩度、明度、透明度を操作できます。
HEXとRGBA・HSVA欄はEnterで確定します。チャンネルの入力範囲は0〜1です。

`Unlimited relative drag`をオンにすると、相対NumberとAngleのドラッグ中にポインターをロックします。
ロックを取得できない場合も、通常のドラッグで調整できます。
Escapeで取り消し、フォーカスを失った場合も編集前の値に戻ります。

WebギャラリーにはSlintの帰属表示があります。バッジはSixtyFPS GmbHの公式素材です。[Slint Brand Guidelines](https://slint.dev/brand-guidelines) · [CC BY-ND 4.0](https://creativecommons.org/licenses/by-nd/4.0/)
