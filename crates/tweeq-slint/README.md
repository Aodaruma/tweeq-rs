# TweeqのSlint入力部品

`tweeq-slint`は、Slintの入力通知をRustの編集処理へつなぎます。
`Editor`にモデル値と制約を登録し、`number_view`から表示用データを取得します。
Undoや保存には、`drain_events`の編集イベントと`Editor`の現在値を使います。

色見本から相対編集する場合は、`begin_relative_color`で選択色の開始値を記録し、
`scrub_color`へポインターの差分と`ColorMode`を渡します。確定は`commit_color`、
取消は`cancel_from`で行います。`drain_slint_events`では共有の開始・確定・取消と、
Slint固有の`RelativeColor`を発生順に取得できます。

組み込み時は、同じ版の`tweeq-slint-build`をbuild-dependencyに追加します。
`build.rs`で`export_ui`を呼び、返されたファイルをSlintのlibrary pathsで`tweeq`に割り当てます。
アプリのルート`.slint`で`TweeqBridge`を再exportし、Rustで入力callbackを接続します。

動く例はリポジトリの`crates/tweeq-slint-demo`と`tests/slint-consumer`にあります。
Slintを含むアプリの配布には、利用する[Slintのライセンス](https://github.com/slint-ui/slint/blob/master/LICENSE.md)の条件が適用されます。
