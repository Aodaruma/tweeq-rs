# TweeqのSlint入力部品

`tweeq-slint`は、Slintの入力通知をRustの編集処理へつなぎます。
`Editor`にモデル値と制約を登録し、`number_view`から表示用データを取得します。
Undoや保存には、`drain_events`の編集イベントと`Editor`の現在値を使います。

組み込み時は、同じ版の`tweeq-slint-build`をbuild-dependencyに追加します。
`build.rs`で`export_ui`を呼び、返されたファイルをSlintのlibrary pathsで`tweeq`に割り当てます。
アプリのルート`.slint`で`TweeqBridge`を再exportし、Rustで入力callbackを接続します。

動く例はリポジトリの`crates/tweeq-slint-demo`と`tests/slint-consumer`にあります。
Slintを含むアプリの配布には、利用する[Slintのライセンス](https://github.com/slint-ui/slint/blob/master/LICENSE.md)の条件が適用されます。
