# Rust Solitaire の検証結果

検証日: 2026年10月3日

## 環境

- macOS 27.0.1、Apple M1 Ultra、Apple Silicon
- Rust / Cargo 1.94.0
- `eframe = 0.35.0`、明示した `wgpu` レンダラー
- 実行時ログ: `rust-solitaire renderer: wgpu / Metal / Apple M1 Ultra`

## 自動検証

以下はすべて成功した。

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p solitaire-core --locked
cargo build --release -p solitaire-egui --locked
cargo tree -p solitaire-core --edges normal
```

| 対象 | 件数 | 主な確認内容 |
| --- | ---: | --- |
| コア単体・プロパティテスト | 12 | 固定配札、赤黒と降順、空列の K 制限、組札、表返し、3枚めくりの端数、再巡回、無効操作、履歴と52枚の不変条件 |
| コア統合テスト | 1 | シード6の238手を公開 API で実行して勝利し、全 Undo / Redo で初期状態と勝利を復元 |
| GUI 入力テスト | 6 | 山札クリック、クリック移動、単体と連続列のドラッグ、無効ドロップ、押下・移動・解放が同じフレームにまとめられる入力 |
| 公開 API のドキュメントテスト | 1 | 配札、山札をめくる、Undo の利用例 |

合計20件が成功。コア単体の通常依存は `rand`、`rand_chacha` とその依存だけで、GUI・ウィンドウ・GPU は含まれない。テストもウィンドウを開かず実行できる。

CLI の `--help` は終了コード0、無効な `--draw 2` と `--seed -1` は説明を出して終了コード2になることも確認した。

## macOS 実画面での検証

| 操作 | 結果 |
| --- | --- |
| GPU 描画 | Metal / Apple M1 Ultra でウィンドウを起動 |
| 英語 UI とカード | 既定フォントで英語のみを表示。ランクごとの柄、異なる絵札、上下の余分なスートの除去を確認 |
| 配札と設定 | シード42・3枚めくり、シード6・1枚めくりを設定して開始 |
| 山札と再巡回 | 3枚ずつめくり、24枚を使い切り、再巡回後も最初の捨て札の順序が一致 |
| クリック・ダブルクリック | カード選択から移動先クリック、A の組札へのダブルクリック移動 |
| ドラッグ | 7C を 8H に移動。3C / 2D の連続列を 4D に移動。無効な場所へのドロップで手数と盤面が変わらない |
| 履歴 | Cmd+Z と Cmd+Shift+Z で山札操作を復元。勝利直前の237手・51枚へ Undo し、238手・52枚の勝利へ Redo |
| ヒント | 合法な操作と移動元・移動先を表示 |
| 再挑戦 | 確認ダイアログから同じシード・ルールの初期配札へ復帰。手数・時間・履歴をリセット |
| 勝利 | シード6の238手を実際の UI から操作し、52 / 52 と `You won!` を表示。勝利中のタイマー停止も確認 |
| リサイズ / HiDPI | macOS の Zoom で拡大・元サイズへ復元し、盤面と統計が追従。2倍密度でも図形と文字を確認 |

素早いドラッグでは、押下と移動、または移動と解放がまとめて届くとドラッグ開始を取りこぼす問題が見つかった。押下時のカードと位置を保持して解放時まで判定するよう修正し、2件の回帰テストと実機で確認した。

## macOS パッケージの検証

2026年10月3日にインストール用 DMG の作成を追加した。

- `bash -n scripts/bundle-macos.sh scripts/package-macos.sh` が成功。
- アプリアイコンを16〜1024ピクセルの10種類で描画し、`iconutil` で ICNS に変換。描画結果を目視確認。
- `bash scripts/package-macos.sh` で約5.6 MiB の Apple Silicon 向け DMG を生成。アプリと DMG の内容は英語。
- `hdiutil verify` でディスクイメージのチェックサムが有効であることを確認。
- 読み取り専用でマウントし、アプリ、`/Applications` へのシンボリックリンク、インストール手順が含まれていることを確認。
- アプリの識別子、バージョン、アイコン、実行属性を確認。署名によるバイナリの変更を除き、実行コードがリリースビルドと一致することを確認。
- DMG と SHA-256 ファイルの一致を確認。
- `codesign --verify --strict` が DMG 内のアプリとコピー後のアプリの両方で成功。
- DMG から作業用フォルダへコピーしたアプリを実際に起動し、山札操作で手数と盤面が更新されることを確認。
- Mach-O の `LC_BUILD_VERSION` は `minos 11.0`、アーキテクチャは arm64。

パッケージは ad-hoc 署名で、Developer ID 署名・Apple の公証は未実施。インストール検証ではユーザーの Applications フォルダを変更せず、コピー後の起動を作業用フォルダで確認した。

## 移動先の自動強調を廃止

2026年10月3日の追加指定により、カードの選択・ドラッグ時に合法な移動先を自動で強調する処理を削除した。候補の強調は `Hint` ボタンを押したときだけ行う。

- `cargo test -p solitaire-egui --locked` の GUI 入力テスト6件が成功。
- `cargo clippy --workspace --all-targets --locked -- -D warnings` が成功。
- 更新したリリースビルドの実画面で、シード6の AH を選択しても Hearts の組札が強調されないことを確認。
- `Hint` を押すと AH と Hearts の組札が強調されることを確認。
- 7C を 8H にドラッグするとカードの移動・裏札の表返し・手数の更新が従来どおり行われることを確認。
- アプリと DMG を再生成し、ディスクイメージのチェックサムを検証。

macOS 標準のタイトルバーと閉じる・最小化・拡大ボタンを有効にする設定も明示した。標準の3ボタンがアクセシビリティ上に存在すること、拡大と最小化が動作することを確認済み。設定を追加した後も `cargo fmt --all -- --check` と Clippy が成功した。

## 確認範囲

Windows / Linux の実機、最小ウィンドウサイズへの縮小は未検証。Developer ID 署名と notarization は未実施。ゲームの保存・再開、解答ソルバー、自動完走、勝てる配札の保証は初版の範囲に含まない。

テスト用の勝利手順は `crates/solitaire-core/tests/fixtures/seed-6.moves` に保存している。アプリが自動でその手順を実行する機能はない。

## Web とネイティブの共通構成

2026年10月3日の追加依頼により、画面非依存のコア、共通 egui UI、デスクトップ起動、Web 起動の4クレートに分割した。既存のカード描画と入力処理は共通 UI に移し、タイマーを `web-time` に変更した。Web の乱数取得は `getrandom` の `wasm_js` を使う。デスクトップの CLI と標準ウィンドウ設定は起動クレートで維持している。

以下の検証が成功した。

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked -p solitaire-desktop
cargo clippy --locked -p solitaire-web --target wasm32-unknown-unknown -- -D warnings
bash scripts/build-web.sh
bash -n scripts/build-web.sh scripts/bundle-macos.sh scripts/package-macos.sh
```

- 既存の20件のテストが分割後も成功。デスクトップ実行ファイルの `--help` も成功。
- WASM リリースビルドから静的サイトを生成。WASM 本体は約8.5 MiB、JavaScript は約140 KiB。
- Pages と同じ `/rust-solitaire/` のサブパスでブラウザを起動し、WASM と静的ファイルの相対パスを確認。
- macOS のブラウザでシード6の1枚めくりを開始。7C → 8H のドラッグ、Cmd+Z と Cmd+Shift+Z、AH の組札へのダブルクリック、2D → 3C のクリック移動、3C / 2D → 4D の連続列ドラッグを確認。
- 操作に応じた手数・組札枚数・経過時間の更新を確認。ブラウザのエラーログはなし。
- シード42の3枚めくりで山札が24 → 21枚となり、8回の Draw の後に再巡回すると山札24枚・捨て札なしに戻ることを確認。

今回の構成変更に伴う macOS の画面キャプチャは実施せず、ネイティブ版は自動検証とリリースビルドで確認した。初版の実画面確認は上記の記録を参照。

GitHub Actions では macOS のチェック・テスト・ネイティブビルドと、Linux の WASM チェック・ビルドが両方成功した後に Pages へ公開する。Web は幅760ピクセル以上の盤面を前提とし、モバイル専用の画面設計、すべてのブラウザの動作保証、進行中ゲームの保存は対象外。
