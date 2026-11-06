# Slint対応とtweeq-rsへの移行方針

- 調査日: 2026-11-06
- 状態: 実装計画を確定
- Rustコードの基準: `96a02c4ddf09355756494093d8f306c144eb6159`（着手時のmain）
- Vue参照: `vue-final-reference` → `ab8e7aa6942ea5455cb585d034bbaff62c0fe7c5`（ローカルGitで解決したコミット）
- 検討の出発点: [参照チャット「Slint対応方針の比較」](chatgpt-conversation://6aecb6bc-0054-83ec-9ec5-92598b2f71f3)

## 1. 推奨する方針

Slint対応は実現可能と考えられます。**既存リポジトリを使い、将来の名称を`tweeq-rs`として、Core・egui版・Slint版を同じCargo workspaceで管理する方式**を推奨します。

Slint版は、原版Vueの見た目・操作仕様を参照して新しく設計します。egui版からはGUI非依存の計算と編集の仕組みを再利用し、レイアウトや描画、フォーカス管理は各GUIに合わせます。両者で同じWidget APIを実装する必要はありません。

実装はSlint版を優先して進め、egui版は既存機能とAPIを維持します。まずNumberで入力と配布を検証し、AngleとColorでコンポーネント合成・独自描画・ポップアップの成立を確かめます。この検証を通過してから、対応部品を拡充します。

| 判断対象 | 推奨 |
|---|---|
| リポジトリ | 現リポジトリを継続し、Slint採用判断後に`tweeq-rs`へ改名 |
| crate名 | `tweeq-core`、`tweeq-egui`を維持し、`tweeq-slint`を追加 |
| UIの設計基準 | 固定したVue参照の操作・視覚仕様と、各GUIに自然な実装方法 |
| 共通化の範囲 | 値、制約、ジェスチャー計算、編集ライフサイクル、操作テスト |
| Slint部品の配布 | `.slint`ソースを配布し、利用側のUIと一緒にコンパイル |
| 対象環境 | Windowsデスクトップと、Windows上のChrome・Firefox |
| 到達点 | 3部品のα版を公開し、無限ドラッグと複数色の相対編集を追加した更新版を公開 |

## 2. 現行コードから分かったこと

現在は`tweeq-core`、`tweeq-egui`、`tweeq-demo`の3クレートです。workspaceはRust 1.92、edition 2024、resolver 3を指定しています。

| 実装箇所 | 確認した内容 | Slint対応での扱い |
|---|---|---|
| [Core manifest](../crates/tweeq-core/Cargo.toml) | 外部依存なし。eguiにも非依存 | その境界を維持 |
| [edit.rs](../crates/tweeq-core/src/edit.rs) | `ParamValue`、`EditOperation`、`EditEvent`などの型 | 両GUIで共有 |
| [gesture.rs](../crates/tweeq-core/src/gesture.rs) | `TweakGesture`が移動量・修飾キーから感度とdeltaを計算 | 入力イベントを正規化して再利用 |
| [validation.rs](../crates/tweeq-core/src/validation.rs) | `NumberConstraints`、`quantize` | 制約適用・snap計算の基盤に使用 |
| [selection.rs](../crates/tweeq-core/src/selection.rs) | ID集合、範囲選択。表示順は呼び出し側から渡す | 選択ロジックを共有し、表示順の取得は各GUIへ |
| [egui context](../crates/tweeq-egui/src/context.rs) | 値・型・順序の登録、session採番、対象snapshot、イベント蓄積とUI状態が同居 | GUI非依存部分の抽出候補 |
| [Number](../crates/tweeq-egui/src/widgets/number.rs) | 計算、書式化、入力判定、描画が同じファイル内。値への直接反映も行う | 純粋な計算だけを段階的に抽出 |
| [Color](../crates/tweeq-egui/src/widgets/color.rs) | RGBA/HSVA・hex変換と描画が同居 | 色計算を共有候補にし、描画は新設 |
| [demoのapply_event](../crates/tweeq-demo/src/lib.rs) | snapshotへの操作適用とCancel時の復元 | 接続例として参照。完全なUndo/Redo実装ではない |
| [theme.rs](../crates/tweeq-egui/src/theme.rs) | テーマ型が`egui::Color32`などを使用 | Slintから依存させず、意味的な色・寸法を合わせる |

**Coreには編集イベントの型がありますが、セッション管理器一式はまだありません。** 参照チャットの「Coreをそのまま使う」という説明は、型・計算については当てはまります。Slintの接続層を追加するだけで、現在の`TweeqContext`全体を共有できるわけではありません。

また、現在のNumberは文字列を`f64`として解析しており、設計資料にある式評価器は未実装です。Coreの`validate`は非有限値を保持するため、UIでのNaN・Infinityの許可方針も別途必要です。`ParamValue::Number`とVector成分は`f64`、Colorは`[f32; 4]`です。

Colorの操作型は現在`SetColor`であり、相対色編集用の変換型はありません。相対的な複数色編集まで共有する段階では、操作型とホスト側の適用処理の拡張が必要です。

crateの公開状況は今回確認していません。既存の[公開チェックリスト](rust-port/release-checklist.md)にも未公開の記録があるため、参照チャットの「公開済み」は前提にせず、workspace内のpath依存で開発を始めます。

## 3. Vue・egui・Slintの関係

固定したVue参照の`src/InputAngle/InputAngle.vue`は、同じmodelを受け取るRotaryとNumberを合成し、幅に応じてNumberを表示します。`InputColor`もColorPad・String・Numberの合成です。この宣言的な部品構造は、Slintのコンポーネントとプロパティbindingで表現しやすいと判断します。[Slintのコンポーネント定義](https://docs.slint.dev/latest/docs/slint/guide/language/coding/file/)

| 観点 | egui | Slint | 今回の判断 |
|---|---|---|---|
| UIの組み立て | Rustの逐次処理とWidget | 宣言的なコンポーネントとbinding | 原版の部品合成はSlintと親和性が高い |
| 状態の寿命 | 毎フレームの構築と保持状態 | 存続するUIとプロパティ更新 | 共通のUI contextに統一しない |
| 独自描画 | Painterなどで直接描画 | Path、Rectangle、Imageなどで構成 | Slint向けに描き直す |
| 入力 | Responseと入力状態を処理 | TouchArea・FocusScope・TextInputから通知 | 取得方法を分け、意味を共有 |
| 配布 | Rust crateを利用 | Rust APIに加え`.slint`のimport解決が必要 | 配布試験を最初から含める |

これは「Slintなら簡単に完全移植できる」という判断ではありません。Vue版の`useDrag.ts`にはPointer Lock、pointer capture、移動量の補正などがあり、CSS・DOM・ブラウザーAPIをそのまま持ち込むことはできません。Slintでの高度な入力の成立は、実機検証で判断します。

## 4. リポジトリ構成の比較

| 案 | 利点 | 負担 | 採用判断 |
|---|---|---|---|
| A. 現repoを`tweeq-rs`に改名して同居 | 履歴・既存の開発資産を維持し、Coreと両GUIを同じ変更で更新できる | Pagesやメタデータの更新、CI分割が必要 | **推奨** |
| B. Slint専用repoを新設 | 保守担当、公開周期、対応環境を独立させやすい | 開発途中のCore変更をrepo間で同期する必要がある | Core APIが安定し、担当が分かれる段階で再検討 |
| C. 新しい`tweeq-rs` repoへ移行 | 独立したRustプロジェクトとして開始できる | 履歴以外のIssue・設定・公開導線などの移行が増える | 今回の目的に対して追加作業が多い |
| D. 単一crateのfeatureでegui/Slintを切替 | 入口を一つにできる | 型・生成処理・feature・ライセンスの説明が混在しやすい | 当面採用しない |

GUIごとの最適化は、同じrepo内でも独立して行えます。保守担当やCore APIがまだ固まっていない段階でrepoを分ける利点は小さく、共通仕様の変更を一つのPRでレビューできる方を優先します。

GitHub上のfork関係の解除は、Slint対応や改名とは別の判断です。今回確認したのはoriginとupstreamのURLであり、GitHub上のfork状態そのものではありません。fork解除や新repo作成を移行の前提にする必要はありません。

### 構成案

既存の`tweeq-demo`はeguiギャラリーとして残します。最初から`examples/`へ移動したり、名前を変更したりする必要はありません。次は**将来の構成案**です。

```text
tweeq-rs/
├── Cargo.toml / Cargo.lock
├── crates/
│   ├── tweeq-core/              # 共通の値・計算・編集基盤
│   ├── tweeq-egui/              # egui固有のUI
│   ├── tweeq-demo/              # 既存eguiギャラリー、publish=false
│   ├── tweeq-slint/             # Rust側のSlint接続層
│   │   └── src/{lib,bridge,context}.rs
│   ├── tweeq-slint-build/       # UIソース配布用の軽量helper
│   │   ├── src/lib.rs
│   │   └── ui/
│   │       ├── tweeq.slint      # 公開import入口
│   │       ├── theme.slint
│   │       └── components/{number,rotary,angle,color}.slint
│   └── tweeq-slint-demo/        # Slintギャラリー、publish=false
├── docs/
│   ├── slint-support-plan.md
│   └── rust-port/              # egui移植の記録を維持
└── .github/workflows/
```

## 5. 共通化する境界

通常依存の向きは、`tweeq-egui → tweeq-core`と`tweeq-slint → tweeq-core`です。eguiとSlintの間に依存は設けず、CoreからGUIへの依存も設けません。

| 層 | 担当するもの | 含めないもの |
|---|---|---|
| `tweeq-core` | 既存の型・計算、編集対象snapshot、セッションの開始・更新・終了、論理的な選択 | `egui::Id`、Color32、Slint生成型、描画・フォーカス、OS入力 |
| `tweeq-egui` | egui Widget、即時モードの登録、テキストbuffer、描画、カーソル制御 | Slintへの依存 |
| `tweeq-slint` | 入力の変換、Slint用表示データ、UIとホストモデルの同期 | eguiの`begin_frame`や描画API |
| `.slint` | レイアウト、テーマ、hover/focus、文字入力UI、アニメーション、ポップアップ | 正確なパラメーター値を`float`だけで保持すること |
| 利用アプリ | 正式なモデル、制約設定、変更の承認・反映、Undo履歴、外部更新との調停 | UI内部状態の直接操作 |

`TweeqContext`の丸ごとのCore移動は行わず、まずGUI非依存の編集管理器（仮称`EditCoordinator`）を抽出します。値・型・論理順序を明示的に登録し、session ID、snapshot、イベント列を管理する役割です。Slint側で毎フレームの登録を要求せず、モデル構築・変更・削除時に登録を更新できる形にします。

Numberのドラッグ計算・修飾キー倍率・表示書式などは、共有が必要になった順に抽出します。Rotaryの角度計算やColorの色変換も同様です。汎用Widget traitや汎用レイアウト木の新設は、現時点では必要ありません。

テーマは、意味的な色名、入力高、角丸、間隔の仕様を共有します。最初は各GUIに対応する値を持ち、テーマの共通型や生成器は、実際に重複が問題になってから検討します。

### 編集の契約

- 一つの操作で`Begin → Update* → Commit`、または`Begin → Update* → Cancel`を返します。終了後のUpdateや二重終了は受け付けません。
- `AddNumber`などはBegin時点のsnapshotに対する累積操作です。直前のpreview値に繰り返し加算しません。
- sourceを含む全対象に対して、ホストが一度だけ変更を適用します。egui版の直接更新とイベント適用を併用しても二重加算しないよう、snapshot基準を維持します。
- 複数対象の制約は対象ごとに適用します。数値入力での「同じ値を設定」と「相対量を加算」を区別し、確定時のstep処理でも意味を変えない契約にします。
- Cancelは全対象を復元し、Commitで一つのUndo単位を閉じます。Core自体はUndo履歴を所有しません。
- 初版は同じ編集context内の同時active sessionを一つに制限し、複数選択は一sessionの複数対象として扱います。
- 編集中の外部モデル更新は、いったんsessionをCancelしてから取り込みます。UI破棄、無効化、対象削除、ウィンドウの非アクティブ化でもsessionが残らないようにします。

制約の適用順や範囲外値などの細部は既存テストを基準にし、変更が必要な箇所は共通仕様として明示します。現行eguiの挙動と原版の挙動が異なる場合は、Slintだけに修正を閉じず、両者への影響を記録します。

## 6. SlintとRustの接続

### 数値とID

Slintの`float`は32bitです。**正式な値、min/max、step、snap、開始値と累積deltaはRust側に保持**し、UIには表示文字列、入力中の文字列、状態、描画用の正規化値を渡します。座標は論理ピクセルに統一し、差分を`TweakGesture`へ渡します。[Slintの数値型](https://docs.slint.dev/latest/docs/slint/reference/property-types/numeric-types/)

表示用文字列は編集開始時の正確な値と区別します。表示precisionで丸めた文字列を、フォーカスしただけ・変更せず確定しただけでモデルへ戻してはいけません。例えば`16777217`が`f32`経由で`16777216`になる経路を作らないことを受入条件にします。これは`f64`の精度を維持する方針であり、任意精度数を保証するものではありません。

`ParamId`は`u64`なので、Slintの数値に変換して渡しません。Slintには不透明な文字列キーを渡し、Rust側で`ParamId`に対応付けます。再利用・削除時の寿命も管理し、表示順や繰り返し行のindexを永続IDにはしません。

### callbackと表示更新

Slintの標準的なRustコード生成では、公開されたWindow/Dialogとglobalが接続点になります。入れ子の各NumberにRustから直接ハンドラーを取り付けられる前提にはせず、`.slint`内のcallbackを公開global（仮称`TweeqBridge`）へ集約する案を採ります。利用側のルート`.slint`でglobalを再exportする必要があります。[Rustコード生成](https://docs.slint.dev/latest/docs/rust/slint/)、[globalの公開方法](https://docs.slint.dev/latest/docs/slint/guide/language/coding/globals/)

```text
TouchArea / TextInput / FocusScope
    → TweeqBridge（キー付き入力通知）
    → tweeq-slint（座標・修飾キー・文字列の変換）
    → tweeq-core（計算・編集イベント）
    → ホストモデルへ反映
    → 表示用property / Modelの更新
```

生成されるRust型は利用アプリ内に置きます。ライブラリの公開APIが、別のコンパイル単位で生成した同名のSlint struct/enumを要求しないよう、接続層ではRust側の通常の型と明示的な変換を使います。初期段階では少量の接続コードを利用例として示し、自動接続マクロは後回しにします。

UI更新はUIスレッドで行い、callbackがモデルを変更した結果の表示更新から、新しい編集通知が再発火しないようにします。編集中bufferとモデル表示を分け、同期処理が入力途中の文字列やIME変換を上書きしない設計にします。UI handleの保持はweak参照を基本とし、終了時の循環参照を避けます。[Slintのスレッドとイベントループ](https://docs.slint.dev/latest/docs/rust/slint/)

### ポインター・キー・ポップアップ

| 要件 | 実装方針 | 初期検証 |
|---|---|---|
| clickとdragの区別 | 閾値を越えるまで値を変えず、clickで文字編集へ | 微小な手ぶれで値が変わらない |
| 横drag・縦感度 | 安定した座標系で差分を取り、Coreに渡す | 斜め移動とDPI変更。部品自体の移動をdeltaに混ぜない |
| Shift/Alt/Q | pointerイベントの修飾キーとkey press/releaseを同期 | ドラッグ途中の変更、Qの解放漏れ、フォーカス喪失 |
| 文字入力 | TextInputとFocusScopeを使い、Enter/blurで確定、Escapeで取消 | IME、選択・貼付、無効文字列、Tab移動 |
| 領域外drag | TouchAreaを基盤とし、対象backendの継続配送を試験 | 部品外・ウィンドウ外でのreleaseとCancel |
| 無限drag | 初版の必須条件にせず、通常dragを提供 | native/ブラウザーのPointer Lockと同等とは扱わない |
| overlay | ウィンドウ内の専用表示層。入力を遮らない構成を検証 | clip、スクロール、重なり順、ウィンドウ端 |
| Color popup | PopupWindowを候補にし、開閉と編集sessionを分離 | 内部クリックで閉じない、Escape、outside click、focus復帰 |

TouchAreaには`moved`、pointerイベント、修飾キー、無効化時のCancel通知がありますが、相対raw motionやOSのカーソルロックまで保証するAPIとはみなしません。通常dragが快適に動くことを先に確認し、無限dragが必要になった時点でbackend固有の拡張を検討します。[TouchArea](https://docs.slint.dev/latest/docs/slint/reference/gestures/toucharea/)

キーの受け取り順とpopupの閉じ方はSlintの仕様に合わせます。Color popupを閉じたことと、色編集を確定・取消することを同一視せず、仕様として決めます。[FocusScope](https://docs.slint.dev/latest/docs/slint/reference/keyboard-input/focusscope/)、[TextInput](https://docs.slint.dev/latest/docs/slint/reference/keyboard-input/textinput/)、[PopupWindow](https://docs.slint.dev/latest/docs/slint/reference/window/popupwindow/)

## 7. `.slint`部品の配布

Cargo依存の追加だけでは、利用側の`.slint`で`import ... from "@tweeq"`が解決されるわけではありません。Slintコンパイラーへ、ライブラリ名と入口ファイルの対応を渡す必要があります。[component libraryとlibrary paths](https://docs.slint.dev/latest/docs/slint/guide/language/coding/file/)

初版は、**UIソースを同梱する`tweeq-slint-build`をbuild-dependencyとして提供する方式**を推奨します。

1. helperに`.slint`と必要な画像などを同梱し、`include_str!` / `include_bytes!`で埋め込みます。UIソースの正本は一か所に置きます。
2. 利用アプリの`build.rs`から、helperが`OUT_DIR/tweeq-ui/`へ展開します。registry内や利用者のソースへ書き込みません。
3. `with_library_paths`で`@tweeq`を入口の`tweeq.slint`へ対応付け、利用アプリのUIを`compile_with_config`でコンパイルします。
4. エディター用には同じUIを取り出す手順と`slint.libraryPaths`の設定例を用意します。build.rsでの設定だけで補完まで解決した扱いにはしません。

helperを分ける理由は、利用アプリのbuild-dependencyとしてSlintランタイム側のcrateを丸ごとビルドすることを避けるためです。helperは標準ライブラリ中心のソース展開APIとし、利用アプリが使う`slint-build`の設定を奪いません。

次は**未実装APIの利用イメージ**です。`export_ui`は提案名であり、このまま現在のrepoで実行できるコードではありません。

```rust,ignore
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let entry = tweeq_slint_build::export_ui(&output.join("tweeq-ui"))?;
    let libraries = std::collections::HashMap::from([("tweeq".into(), entry)]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(libraries);
    slint_build::compile_with_config("ui/app.slint", config)?;
    Ok(())
}
```

公式には`as_library` / `rust_module`によるモジュール共有の仕組みもありますが、確認した1.18.1の文書ではexperimentalです。初版の必須条件にせず、安定化後にソース配布方式と再比較します。[CompilerConfiguration](https://docs.slint.dev/latest/docs/rust/slint_build/struct.CompilerConfiguration)

Slintの候補は、調査時の公式Rust API文書で確認できた1.18.1です。`slint`と`slint-build`は同じ版に揃えて試験し、採用時にmanifest・依存全体のMSRVを確認します。候補のmanifestにはRust 1.92の記載があり、現在のworkspace指定とは一致しますが、lockfileを含むビルドの成功は未確認です。[slint manifest](https://docs.rs/crate/slint/latest/source/Cargo.toml)、[slint-build manifest](https://docs.rs/crate/slint-build/latest/source/Cargo.toml)

公開前には、workspace外の小さな利用アプリをタスク内に作り、packageから展開したcrateだけでimport・ビルド・実行できることを確かめます。ローカルrepoへの相対パスや開発者PCの絶対パスに依存しないこと、同梱asset・ライセンス・再ビルド通知も確認対象です。runtimeとhelperは対応版を明示してリリースします。

## 8. 性能・ビルド・CI

monorepo自体に実行時のオーバーヘッドはありません。egui利用アプリの依存にSlintを加えなければ、Slint UIをリンクする必要もありません。ただし共有lockfileの解決・更新にはworkspace全体の依存が関わるため、ビルド対象の分離と依存解決の完全な独立は同じではありません。[Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)

初期は`default-members`を既存3クレートにして、引数なしのビルドの対象を維持する案が適切です。Slintは`-p tweeq-slint-demo`で選びます。`--workspace`は引き続き全体を対象とします。[Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html)

| CI単位 | 方針 |
|---|---|
| Core | GUI依存なしでtest、MSRV、native/WASM check |
| egui | 現行のlint・test・WASM・docs・package検証を維持 |
| Slint native | helper、runtime、demo、外部利用例のビルド。Windowsで操作試験 |
| Slint WASM | WinitとFemtoVGでビルドし、Trunkで配布。Chrome・Firefoxで操作を検証 |
| 全体 | fmtと統合試験。GUI別のjobを残し、一方の成功で他方を代替しない |
| 公開 | Pagesへギャラリーを配備。crateはhelper/runtimeの対応版とpackage内容を検証し、公開準備まで進める |

現在の[CI](../.github/workflows/ci.yml)と[release workflow](../.github/workflows/release.yml)には`--workspace --all-features`があります。そのままSlintを追加すると、OS依存・backend/renderer依存まで一括で有効になるため、対応するfeatureの組合せを明示したjobへ整理します。releaseのcrate選択・検証対象も更新が必要です。

性能は両GUIの個別Cargo呼び出しで測定します。共通依存のfeatureは同時に選択したpackage間で統合される場合があり、resolver 3でも任意の統合がなくなるわけではありません。[feature統合の規則](https://doc.rust-lang.org/cargo/reference/resolver.html)

比較対象は起動時間、アイドル時のCPUとメモリー、100/1000個のNumber表示、ドラッグ中の応答と値通知頻度、Color描画、実行ファイルサイズ、ビルド時間とします。同じ画面密度・release設定・DPI・機能条件で測り、Slintが速いと先に断定しません。`TweakGesture`は内部状態を持つため、イベントの間引きやフレーム単位への集約で操作感が変わらないかも検証します。

## 9. ライセンスの扱い

Tweeq由来・新規作成部分はMITと原著作権表示を維持する方針です。ただしSlintランタイムは別ライセンスであり、Slintを含む配布物全体を「MITのみ」とは説明しません。公式案内ではGPLv3、Royalty-free、商用の選択肢があります。[Slint LICENSE](https://github.com/slint-ui/slint/blob/master/LICENSE.md)

Royalty-free 2.0には帰属表示に加え、Slint単体の再配布、Slint APIを公開するアプリケーション、組込み用途などの制限があります。再利用ライブラリのソース公開と、Slintを含む配布物の条件は区別し、採用版と具体的な配布形態に対して確認します。[Royalty-free 2.0原文](https://slint.dev/agreements/slint-royalty-free-license.pdf)

実装時には、crateごとの説明、NOTICE、利用アプリ向けの帰属表示例を用意します。Core・eguiの通常依存グラフへSlintを入れない構成を保ちます。今回の調査だけで、すべての商用・再配布用途を許諾済みとは判断していません。

## 10. 段階と完了条件

| 段階 | 完了条件 | コミット |
|---|---|---|
| 0. 設計確定 | 実装範囲、公開順、受入条件を確定 | `docs: finalize the Slint implementation plan` |
| 1. Numberと組み込み | Windows・Webのデモと別のSlintアプリでNumberを使用し、下記の試験を通過 | `feat(slint): add numeric editing and component imports` |
| 2. α版の3部品 | Angleの複数回転・スナップ・狭幅表示、Colorのポップアップ・HEX・チャンネル編集を検証 | `feat(slint): add angle and color controls` |
| 3. α版公開 | `tweeq-rs`へ改名し、Pagesで両ギャラリーの起動を確認 | `ci(pages): publish the Slint alpha gallery` |
| 4. 無限ドラッグ | WindowsとWebでNumberとAngleの相対編集を継続でき、ロックを安全に解除できる | `feat(slint): support unbounded pointer dragging` |
| 5. Colorの発展機能 | チャンネルのショートカット、複数色の相対編集、無限ドラッグを検証し、更新版をPagesで確認 | `feat(slint): add relative editing for selected colors` |

各フェーズの変更は検証後に個別にコミットし、`main`の履歴に残します。公開先の確認はデプロイ後に実施します。Goalはフェーズ5のコミットと公開先での動作確認が揃った時点で完了します。

発展フェーズではWindowsのWinitとWebのPointer Lockから得た移動量を、通常ドラッグと同じ計算に渡します。取得失敗時は通常ドラッグを続け、取得後にロックを失った場合は編集を取り消します。Colorの相対編集は各色の開始値に同じ変化量を適用し、色相を循環、他のチャンネルを範囲内に収めます。既存の`EditOperation`とは別にSlint側の色操作イベントを用意します。

### Numberの必須試験

- 範囲あり・なし、正負、step/snap、横dragと縦感度、Shift/Alt/Qを操作途中で変更する。
- 文字編集のEnter/blur/Tab、Escape、不正文字列、IME、未変更確定を確認する。
- `16777217`などの値がUI往復や表示丸めで変化しないことを確認する。
- 一操作のBegin/終了が各一回で、Cancelが全対象を復元し、ホストのUndo一回で操作全体を戻せる。
- 複数選択の絶対設定・相対設定と、対象ごとの制約を確認する。
- 対象削除、無効化、UI破棄、ウィンドウ非アクティブ化、領域外releaseでsessionが残らない。
- 正規化した同じ移動列と修飾キー列を両GUIの接続層へ渡し、最終値と編集イベントを比較する。
- 100/150/200% DPI、light/dark、複数の入力欄、狭幅表示で操作可能性を確認する。
- packageから利用するアプリが、開発用のpath依存なしで部品をimportできる。

OSのイベント列はフレームレートなどで異なるため、共通ロジックの試験と実機での操作試験を分けます。ウィジェットが表示できただけで移植完了とはしません。

### 採用を見直す条件

通常dragや文字入力の安定性、高精度値の保持、component libraryの配布、許容する配布条件に解消困難な問題があれば、段階2で対象環境・操作の代替案を見直します。無限dragだけが未対応であれば、その差異を明記して通常dragで進める余地があります。egui版があるため、Slintの検証結果に応じて対応範囲を調整できます。

## 11. `tweeq-rs`への改名

`tweeq-rs`はRust版全体を表すrepo名として適切です。repo名とcrate名は独立しているため、改名に伴って`tweeq-egui`というcrate名や利用者のimport名を変える必要はありません。

改名の時期はSlintの採用判断後を推奨します。技術的には先に改名しても問題ありませんが、Pages更新などを一回で揃えやすくなります。

| 対象 | 更新内容 |
|---|---|
| GitHub repo | `Aodaruma/tweeq-egui` → `Aodaruma/tweeq-rs`。履歴を引き継ぐ |
| ローカルremote | 改名後にoriginを新URLへ変更。upstreamは原版のまま維持 |
| [Cargo.toml](../Cargo.toml) | workspaceの`repository`、`homepage` |
| [deploy.yml](../.github/workflows/deploy.yml) | Trunkの`--public-url /tweeq-egui/`を新しい公開構成へ変更 |
| [README](../README.md)・crate README | プロジェクト名、CI badge、Pages・参照tagのURL、Slint利用入口 |
| [CITATION.cff](../CITATION.cff) | ソフトウェアのtitle、repo URL、公開URL。原論文の書誌は維持 |
| NOTICE・研究資料 | 現在の配布元を示すURLを更新。著作権表示と過去の記録は維持 |
| 公開済みcrateがある場合 | 過去版は変更できないため、次回リリースでmetadata更新 |

GitHubは通常のrepo URLやGit操作を転送しますが、project PagesのURLは同じ扱いではありません。新しいPages URLでasset・gallery・API docsを検証します。旧repo名を再利用すると転送が失われる点にも注意します。[GitHubの改名仕様](https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository)

改名後のPagesでは、ルートにeguiギャラリー、`/slint/`にSlintギャラリー、`/api/`にAPI文書を置きます。両ギャラリーの起動と、移行後のリンクを確認します。

ローカルフォルダ名やcheckoutの配置は、GitHubの改名に合わせて変更する必要はありません。この作業で既存checkoutを移動する計画は含めません。

## 12. 後続作業の入口

最初の実装単位は、**編集管理の最小抽出、Slint Number、UI配布helper、ギャラリーと外部利用例**です。これを一つの縦切りとして検証すると、Slint対応の主要な不確実性を早期に判断できます。

参照チャットの「UIは各GUIに合わせ、Coreと操作仕様を共有する」という方向性は妥当です。今回のコード確認を加えると、先に必要なのは大量の部品移植ではなく、現在egui側にある編集管理を整理し、Slintの配布・値の同期・入力の接続を実証することです。

`docs/rust-port/`はegui移植の経緯を示す資料です。複数GUI対応の進め方は本書に記録します。
