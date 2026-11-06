# Tweeq Rust / egui 移植設計

Slint対応と複数GUI構成への拡張案は、[Slint対応とtweeq-rsへの移行方針](../slint-support-plan.md)を
参照してください。この文書群は、既存のegui移植の設計・経緯を記録しています。

## 現在地

この文書群は、TweeqのVue実装をRustへ再設計し、最終的に
`tweeq-egui`としてcrates.ioへ公開するための基準を定めるものです。
Phase 5の比較評価と主要入力のparity passを完了し、Phase 6でリポジトリを
Rust-onlyへ切り替えました。native/WASMの対話型ギャラリーとrustdocが正式な
ドキュメント入口です。

- 作業ブランチ: `codex/rust-egui-port`
- 参照実装: upstream Vue版と注釈付きtag `vue-final-reference`
- 最終参照コミット: `ab8e7aa6942b5a016c565dd4dc26b6d36130a17a`
- 基準日: 2026-07-22
- ライセンス: MIT（Baku Hashimotoによる原著作権表示を維持する）

## 進捗

| Phase | 状態 | 主な成果 |
|---:|---|---|
| 0 | 完了 | 設計基準、移植表、ロードマップ |
| 1 | 完了 | Cargo workspace、3クレート、CI、最小native/WASMデモ |
| 2 | 完了 | Number縦切り、型付き編集イベント、選択・gesture・validation |
| 3 | 完了 | P1/P2入力の基礎実装とパラメーターギャラリー |
| 4 | 試作完了 | 高度入力、ワークスペースprimitive、P4 adapter |
| 5 | 評価完了 | Vue比較、互換性分類、公開/切替チェックリスト（切替はNo-Go） |
| 5 follow-up | 実装・評価済み | Number/Angle active UI、主要入力のparity pass、Vue単体比較ページ |
| 6 | 完了 | Vue/Node削除、Rust-first README、Trunk gallery、rustdoc Pages、Rust-only CI |
| 7 | 公開準備完了 | crate metadata、MSRV/release gate、分割publish workflow、dry-run手順 |

## 目標

1. Tweeq固有の高速・高精度なパラメーター調整操作をRustで再現する。
2. ライブラリ本体は`egui`だけに依存し、eframe、Bevyなどのeguiホストで
   利用できるようにする。
3. nativeとWASMで同じ公開APIを提供し、機能差は明示的なフォールバックで
   吸収する。
4. 同時編集、Undo境界、式入力を、Rustの所有権と安全性に合うイベント方式へ
   再設計する。
5. Rust版の互換性・ドキュメント・公開準備が整った段階で、`main`からVue、
   Vite、npm依存を削除する。
6. `tweeq-egui`をcrates.ioへ公開する。

## 非目標

- Vueのコンポーネント構造やDOM処理を逐語的に翻訳すること。
- `eframe`をライブラリ利用者へ必須依存として課すこと。
- 全OSでPointer Lockの挙動を完全に同一にすること。
- Monaco EditorそのものをRustで再実装すること。
- 移植完了前にVue版を削除して比較対象を失うこと。

## 決定事項

| 項目 | 方針 |
|---|---|
| 公開クレート | `tweeq-egui`を主クレート、`tweeq-core`をUI非依存クレートとする |
| eframe | デモ、native/WASM統合試験にのみ使用する |
| 値の精度 | 意味論・公開値は原則`f64`、描画時のみeguiの`f32`へ変換する |
| UI状態 | 安定した`egui::Id`と`ParamId`で保持し、値への`&mut`をフレーム間に保存しない |
| 同時編集 | 値参照の登録ではなく、ID付き編集イベントをアプリケーションへ返す |
| Pointer Lock | `Response::drag_motion()`を優先し、grab不能時は通常ドラッグへ退化する |
| オーバーレイ | eguiのforeground layerへ描画し、DOM top layerは移植しない |
| Color描画 | 最初はCPU生成テクスチャ、必要性を計測後にGPU callbackを検討する |
| 式入力 | JavaScript `eval`は移植せず、制限付きパーサーを任意機能として実装する |
| Vue参照 | mainへ併設せず、`vue-final-reference` tagとupstreamで固定参照する |

## 文書

- [アーキテクチャ](./architecture.md): クレート境界、状態、イベント、入力、描画、API
- [コンポーネント移植表](./component-matrix.md): Vue版各機能の移植先と優先順位
- [ロードマップ](./roadmap.md): 段階、完了条件、ブランチ運用、公開手順
- [Phase 2検証記録](./phase-2-number.md): Number縦切りの操作と既知制限
- [Phase 3検証記録](./phase-3-gallery.md): 中核入力ギャラリーと互換性上の残課題
- [Phase 4検証記録](./phase-4-workspace.md): 高度入力、workspace primitive、adapter境界
- [互換性評価](./compatibility-report.md): Vue実測比較、移植/代替/制限/ブロッカー判定
- [全component parity監査](./component-parity-audit.md): 実装済み範囲と残件の最新一覧
- [Input parity pass](./phase-5-parity-pass.md): Number/Angleと主要入力の再現改善・検証記録
- [Phase 6切替記録](./phase-6-rust-switch.md): Rust-only化の範囲、受入差分、復元方法
- [公開・切替チェックリスト](./release-checklist.md): crates.io previewとmain完全Rust化の条件

## 「完全移植」の定義

完全移植とは、ソースファイル数を一致させることではありません。次をすべて満たす
ことを完了条件とします。

- P0〜P3の対象について、操作意味論または文書化した代替操作が存在する。
- Windows、macOS、Linux、WASMの検証結果と既知の差異が公開されている。
- キーボードだけで主要入力を確定・取消できる。
- 編集の`Begin`、`Update`、`Commit`、`Cancel`がUndo単位として利用できる。
- ライブラリの通常利用でunsafeコードを必要としない。
- RustのデモとAPI文書がVue版ドキュメントの役割を代替する。
- `cargo fmt`、`cargo clippy`、native/WASMテスト、`cargo publish --dry-run`が通る。
- Vue版を参照できるGitタグを作成したうえで、`main`からNode系資産を削除する。

Monaco、Markdown、アプリシェルなどのP4統合機能は、同一実装ではなく、egui向けの
置換手段と拡張点が用意されていれば完全移植に含めます。
