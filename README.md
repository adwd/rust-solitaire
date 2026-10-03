# Rust Solitaire

Rust 製のクロンダイク。`egui` と `eframe` を使い、`wgpu` で描画するデスクトップゲームです。画面に依存しないコアを独立したクレートにしています。UI は英語です。

## 起動

Rust 1.94 以降が必要です。macOS での実行を対象にしています。

```sh
cd ~/ghq/github.com/adwd/rust-solitaire
cargo run --release -p solitaire-egui
```

配札を再現したい場合はシードを指定できます。

```sh
cargo run --release -p solitaire-egui -- --seed 42 --draw 3
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

| キー | 操作 |
| --- | --- |
| Cmd / Ctrl + Z | Undo |
| Cmd / Ctrl + Shift + Z | Redo |
| Esc | 選択やドラッグのキャンセル |

手数には山札をめくる・再巡回・カード移動を各1手として数えます。場札が露出したときの自動的な表返しは移動と同じ1手で、Undo もまとめて戻します。経過時間は最初の操作で開始し、勝利で停止します。Undo では巻き戻しません。

数字札には数字と同じ数のスートを描き、A は大きな1つのスート、J・Q・K はそれぞれ帽子・ティアラ・王冠を持つ絵札として描きます。上下の角はランクだけとし、余分なスートは表示しません。絵札のスートは衣装の紋章で判別できます。図形で描画するため、拡大や HiDPI でも鮮明です。

## クレート構成

| クレート | 責務 |
| --- | --- |
| `solitaire-core` | カード、配札、ルール、合法手、勝利判定、履歴。GUI・GPU・時計・ファイル I/O への依存を持ちません |
| `solitaire-egui` | `rust-solitaire` 実行ファイル。描画、座標から操作への変換、設定とシード取得、経過時間を扱います |

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
cargo build --release -p solitaire-egui --locked
```

コアだけのテストにはウィンドウや GPU は必要ありません。

```sh
cargo test -p solitaire-core --locked
cargo tree -p solitaire-core --edges normal
```

ルールの境界条件、再巡回の順序、無効操作、勝利と履歴を検証します。プロパティテストでは複数のシードと操作列で52枚の不変条件と履歴の往復を確認します。GUI の入力テストは実際の egui のポインターイベントでクリックとドラッグを検証します。

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

ゲームの保存と再開、解答ソルバー、自動完走、勝てる配札の保証、スコア方式、サウンド、Web やモバイル向け配布は含みません。終了すると進行中のゲームは失われます。Windows / Linux の実機動作は未検証です。

計画は [docs/PLAN.md](docs/PLAN.md)、実際の検証結果は [docs/VALIDATION.md](docs/VALIDATION.md) に記載しています。

## 使用ライブラリ

- [egui と eframe](https://github.com/emilk/egui)
- [wgpu](https://github.com/gfx-rs/wgpu)
- [rand と rand_chacha](https://github.com/rust-random/rand)
- [proptest](https://github.com/proptest-rs/proptest)

カードの柄と絵札はこのアプリ内で図形として描画しています。外部のカード画像や OS フォントは同梱しません。
