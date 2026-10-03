# Rust Solitaire

Rust 製のクロンダイク。`egui` と `eframe` を使い、`wgpu` で描画します。WebAssembly の Web 版と macOS ネイティブ版で同じ画面と操作を共有し、ゲームのコアは画面に依存しない独立したクレートです。UI は英語です。

**Web 版:** [GitHub Pages で遊ぶ](https://adwd.github.io/rust-solitaire/)

## 起動

`rust-toolchain.toml` で Rust 1.94.0 を指定しています。ネイティブ版の実機確認は macOS で行っています。

ウィンドウは macOS 標準のタイトルバーと、閉じる・最小化・拡大の3つのボタンを使います。

```sh
cd ~/ghq/github.com/adwd/rust-solitaire
cargo run --release
```

配札を再現したい場合はシードを指定できます。

```sh
cargo run --release -- --seed 42 --draw 3
```

`--draw` は `1` または `3`。省略時は1枚めくりです。シードは符号なし64ビット整数で、省略時はランダムです。`--help` で引数を確認できます。

## 遊び方

- **場札:** 赤黒交互で降順に重ねます。表向きの連続列はまとめて移動できます。空の列には K、または K から始まる列を置けます。
- **組札:** スートごとに A から K へ積みます。52枚すべてを組札に移せば勝利です。組札の最上段を場札に戻す操作も可能です。
- **山札:** クリックして1枚または3枚めくります。捨て札から使えるのは最上段だけです。山札がなくなったらクリックして再巡回できます。回数制限はありません。
- **移動:** ドラッグ、またはカードを選んで移動先をクリックします。ダブルクリックすると可能な組札へ移動します。カードを選択・ドラッグしても移動先の候補は自動で強調しません。
- **New game:** めくり枚数とシードを指定します。空欄ならランダムな配札を作ります。ゲーム途中の場合はリセットする旨を表示します。
- **Restart:** 同じシードとルールで最初から再挑戦します。
- **Hint:** 合法な操作を表示します。繰り返し押すと候補が切り替わります。最善手や勝利の保証はありません。
- **自動クリア:** 裏向きの場札がなくなり、山札の巡回と組札への移動だけで完了する手順をコアが証明できたら、自動でカードを組札に送ります。開始時にタイマーが止まり、最後のカードが到着すると勝利画面と GPU の花火が表示されます。自動移動中も Undo で中断して手動プレイに戻れます。

| キー | 操作 |
| --- | --- |
| Cmd / Ctrl + Z | Undo |
| Cmd / Ctrl + Shift + Z | Redo |
| Esc | 選択やドラッグのキャンセル |

手数には山札をめくる・再巡回・カード移動を各1手として数えます。場札が露出したときの自動的な表返しは移動と同じ1手で、Undo もまとめて戻します。自動クリア中の操作も手数と履歴に含みます。経過時間は最初の操作で開始し、自動クリアが確定した時点（または手動で勝利した時点）で停止します。Undo では巻き戻さず、プレイに戻ると計測を再開します。

数字札には数字と同じ数のスートを描き、A は大きな1つのスート、J・Q・K はそれぞれ帽子・ティアラ・王冠を持つ絵札として描きます。上下の角はランクだけとし、余分なスートは表示しません。絵札には明るい背景に大きなスートの紋章を1つ置き、同じ赤のハート／ダイヤ、黒のスペード／クラブを形で区別できます。図形で描画するため、拡大や HiDPI でも鮮明です。

## クレート構成

| クレート | 責務 |
| --- | --- |
| `solitaire-core` | カード、配札、ルール、合法手、勝利判定、自動クリアの手順証明、履歴。GUI・GPU・時計・ファイル I/O への依存を持ちません |
| `solitaire-egui` | 共通 UI ライブラリ。カード描画、入力、設定、ヒント、経過時間、自動移動アニメーション、WGSL の勝利演出を扱います |
| `solitaire-desktop` | `rust-solitaire` 実行ファイル。CLI、OS 標準ウィンドウ、ネイティブ起動を扱います |
| `solitaire-web` | WASM ライブラリ。ブラウザの canvas と `WebRunner` の起動・終了を扱います |

依存方向は両方の起動クレート → `solitaire-egui` → `solitaire-core` です。プラットフォーム固有の設定は起動クレートに置きます。時計には `web-time`、ブラウザの乱数には `getrandom` の `wasm_js` を使い、コアの決定的な配札は共有します。macOS では Metal、Web では WebGPU を優先し、利用できない場合は WebGL にフォールバックします。

GUI は読み取り用の `Game::view()` を使い、変更は `Game::apply(Action)` を通します。無効な操作は盤面・手数・Undo / Redo 履歴を変更しません。ドラッグ中もコアは変えず、ドロップ時に操作を確定します。

```rust
use solitaire_core::{Action, Game, Rules};

let mut game = Game::new(42, Rules::default());
game.apply(Action::Draw).unwrap();
assert_eq!(game.view().stock.len(), 23);
assert!(game.undo());
assert_eq!(game.view().stock.len(), 24);
```

配札アルゴリズム v1 はスート順 C/D/H/S、ランク順 A〜K のデッキを ChaCha8 と u32 の Fisher–Yates でシャッフルし、末尾から各列へ配ります。同じ実装ではシードとルールが一致すれば配札も一致します。`Cargo.lock` と固定シードのテストで再現性を管理します。

## 検証

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release -p solitaire-desktop --locked
cargo clippy -p solitaire-web --target wasm32-unknown-unknown --locked -- -D warnings
```

コアだけのテストにはウィンドウや GPU は必要ありません。

```sh
cargo test -p solitaire-core --locked
cargo tree -p solitaire-core --edges normal
```

ルールの境界条件、再巡回の順序、無効操作、勝利と履歴を検証します。プロパティテストでは複数のシードと操作列で52枚の不変条件と履歴の往復を確認します。GUI の入力テストは実際の egui のポインターイベントでクリックとドラッグを検証します。

## Web 版のビルドと公開

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
bash scripts/build-web.sh
python3 -m http.server 8080 --directory dist
```

`http://localhost:8080/` で遊べます。`rust-toolchain.toml` が WASM ターゲットを導入します。CLI のバージョンは `Cargo.lock` と一致させ、ビルドスクリプトでも確認します。生成先は `dist/` で、HTML・JavaScript・CSS・WASM だけの静的サイトです。パスは相対指定のため、GitHub Pages の `/rust-solitaire/` 以下でも動作します。ブラウザは JavaScript と GPU 描画が必要です。リロードすると進行中のゲームは失われます。

`.github/workflows/pages.yml` は macOS 上のテストとネイティブビルド、Linux 上の WASM ビルドを行います。両方が成功した `main` の変更だけを GitHub Pages へ公開します。Pull request では検証だけを実行します。リポジトリの Settings → Pages → Source は **GitHub Actions** を使います。

## macOS アプリの作成

```sh
bash scripts/bundle-macos.sh
```

`target/Rust Solitaire.app` を生成します。`bash scripts/bundle-macos.sh '/任意の出力先/Rust Solitaire.app'` で出力先を指定できます。カードのアプリアイコンも生成し、アプリ全体を ad-hoc 署名します。`CARGO_TARGET_DIR` を指定したビルドにも対応します。

### macOS インストール用 DMG

```sh
bash scripts/package-macos.sh
```

ビルドした Mac のアーキテクチャ向けに `target/rust-solitaire-0.1.0-arm64.dmg`（Apple Silicon の場合）と SHA-256 ファイルを生成します。出力先も指定できます。

```sh
bash scripts/package-macos.sh '/任意の出力先/rust-solitaire.dmg'
```

DMG を開き、`Rust Solitaire.app` を `Applications` にドラッグしてください。ディスクイメージを取り出したら、Applications から起動します。遊ぶ際に Rust や Cargo は不要です。macOS 11 以降が必要で、今回の検証済みパッケージは Apple Silicon 用です。

パッケージ作成には Rust / Cargo、Python 3、Xcode Command Line Tools の Swift、および macOS 標準ツールを使います。アイコンは Swift / AppKit の図形で生成し、外部画像や追加の画像処理ライブラリは使いません。

現在の DMG はローカル利用向けの ad-hoc 署名です。Developer ID 署名と notarization は行っていません。他の Mac にダウンロードして一般配布する場合は、Apple の [Developer ID 署名・公証](https://developer.apple.com/developer-id/) を行う必要があります。

## 初版の範囲

ゲームの保存と再開、任意の盤面の解答ソルバー、勝てる配札の保証、スコア方式、サウンド、モバイル専用の操作・画面設計は含みません。Web の盤面は幅760ピクセル以上の表示領域を前提とします。終了すると進行中のゲームは失われます。Windows / Linux のネイティブ実機動作は未検証です。

計画は [docs/PLAN.md](docs/PLAN.md)、実際の検証結果は [docs/VALIDATION.md](docs/VALIDATION.md) に記載しています。

## 使用ライブラリ

- [egui と eframe](https://github.com/emilk/egui)
- [wgpu](https://github.com/gfx-rs/wgpu)
- [rand と rand_chacha](https://github.com/rust-random/rand)
- [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen)
- [web-time](https://github.com/daxpedda/web-time)
- [proptest](https://github.com/proptest-rs/proptest)

カードの柄と絵札はこのアプリ内で図形として描画しています。外部のカード画像や OS フォントは同梱しません。
