# Rust Solitaire

Rust 製のクロンダイク。`egui` / `eframe` の Web・デスクトップ版に加え、Bevy 0.20.0 の3Dデスクトップ版があります。どちらも `klondike-core` の配札・ルール・履歴・自動クリアを共有します。UI は英語です。

Web 版では同じ構成で作った数独・2048・マインスイーパー・倉庫番・ノノグラムも遊べます。画面上部の **GAMES** バーで切り替えられ、切り替えても各ゲームの進行は保持されます。

**Web 版:** [GitHub Pages で遊ぶ](https://adwd.github.io/rust-solitaire/)（[数独](https://adwd.github.io/rust-solitaire/#sudoku)・[2048](https://adwd.github.io/rust-solitaire/#2048)・[マインスイーパー](https://adwd.github.io/rust-solitaire/#minesweeper)・[倉庫番](https://adwd.github.io/rust-solitaire/#sokoban)・[ノノグラム](https://adwd.github.io/rust-solitaire/#nonogram) を直接開くこともできます）

## 起動

`rust-toolchain.toml` で Rust 1.99.0 を指定しています。ネイティブ版の実機確認は macOS で行っています。

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

## Bevy 3D 版

```sh
cargo run --release -p klondike-bevy
cargo run --release -p klondike-bevy -- --seed 42 --draw 3
```

定番の7列の場札、左上の山札・捨て札、右上の4つの組札を3Dのフェルト卓に配置します。カードは厚みのあるメッシュで、ドラッグ中は卓から浮き上がり、移動先へ滑らかに着地します。`V` または View で斜め俯瞰と真上視点を切り替えられます。画像は実行ファイルに埋め込まれており、起動時の外部アセットは不要です。

操作はドラッグ、または選択して移動先をクリック。ダブルクリックまたは右クリックで組札へ送ります。山札をクリック、または Space でめくり、空なら再巡回します。

| キー | Bevy版の操作 |
| --- | --- |
| Z / Cmd・Ctrl + Z | Undo |
| Y / Cmd・Ctrl + Shift + Z | Redo |
| H | 合法手のヒント（押すたびに切り替え） |
| A | コアが証明した手順で自動クリア |
| V | カメラ視点の切り替え |
| R | 同じシード・ルールで再開始 |
| N | 新しいランダム配札 |
| Esc | 選択・ドラッグ・自動クリアを中断 |
| F12 | `screenshots/` に画面を保存 |

Draw 1 / 3 は同じシードの新しいゲームとしてめくり枚数を変更します。Restart・New deal・Draw 1 / 3 は現在のゲームを即座に置き換えます。自動クリアは A / Finish から開始し、タイマーは開始時に止まります。Undo で中断でき、全操作が共有コアの履歴に残ります。

GPU描画のプレビューを保存して終了することもできます。

```sh
cargo run -p klondike-bevy -- --seed 42 --capture preview.png
```

Bevy版はネイティブ向けです。既存の Web 版と通常の `cargo run` は引き続き egui 版を起動します。配色は Web 版と共通です。クラシックなカード原画は CC0 の SVG を同梱し、再生成方法・出典・フォントライセンスは `crates/klondike-bevy/assets/README.md` に記載しています。

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

## 数独・2048・マインスイーパー

いずれも Web 版の GAMES バーから選びます。URL の `#klondike`・`#sudoku`・`#2048`・`#minesweeper` で開くゲームを指定でき、切り替えるとアドレスバーも追従します（履歴は増やしません）。New game ではシードを指定でき、同じシードなら同じ盤面になります。

- **数独:** 各行・列・3×3 ブロックに 1〜9 を1つずつ入れます。Easy / Medium / Hard の問題はすべて解が1つです（ヒント数の下限はそれぞれ 40 / 32 / 26）。重複した数字は赤く表示し、Show mistakes をオンにすると正解と異なる数字も赤くなります。数字を置くと同じ行・列・ブロックのメモからその数字を消します。
- **2048:** 同じ数字のタイルをぶつけて合体させます。1回の移動で合体するのは各タイル1回までで、移動ごとに 2（10% で 4）が出現します。2048 に到達すると勝利で、Keep going で続行できます。Undo しても出現するタイルは同じです。
- **マインスイーパー:** Beginner（9×9・10個）/ Intermediate（16×16・40個）/ Expert（30×16・99個）。地雷は最初のクリック時に配置され、最初のマスとその周囲（置ける場合）は必ず安全です。数字のマスをクリックすると、周囲の旗の数が一致していれば残りの周囲をまとめて開きます（コード）。Hint は、開いている数字だけから確実に安全、または確実に地雷と言えるマスを1つ強調し、根拠になった数字を枠で示します。1つの数字で決まるもの、2つの数字の包含関係で決まるもの、周辺の配置を全通り調べて決まるもの、残り地雷数で決まるものの順に探します。旗はプレイヤーの推測なので根拠に使わず、安全と分かったマスの旗は誤りとして指摘します。確実なマスがなければ、推測するしかないと伝えます。

| キー | 数独 | 2048 | マインスイーパー |
| --- | --- | --- | --- |
| 矢印 | セル選択 | スライド | — |
| 1〜9 / Backspace | 入力 / 消去 | — | — |
| W A S D | — | スライド | — |
| H | ヒント | — | ヒント |
| N | メモ切替 | — | — |
| F / R | — | — | 旗モード切替 / 同じ広さで新規 |
| Cmd・Ctrl + Z（+ Shift） | Undo（Redo） | Undo（Redo） | — |

2048 はスワイプ、マインスイーパーは右クリック・長押し・Flag mode で旗を立てられます。

## 倉庫番・ノノグラム

Web 版の GAMES バーの **Sokoban**（倉庫番）と **Nonogram**（ノノグラム）から選びます。URL の `#sokoban`・`#nonogram` でも直接開けます。両ゲームともオリジナルの固定問題とランダム生成に対応し、ゲーム間の切り替えでは進行を保持します。

- **倉庫番:** 固定8ステージに加え、New random で Small（7×7・箱2個）/ Medium（9×9・箱3個）/ Large（11×11・箱4個）の問題を生成できます。木箱をすべて金色のゴールへ押すとクリアです。箱は1つずつ押せますが、引くことはできません。ゴール上の箱は緑色のチェック付きになり、移動数・押した回数を表示します。矢印キー、WASD、画面の方向ボタン、またはプレイヤーに隣接するマスのクリック・タップで移動します。
- **ノノグラム:** 5×5・10×10 各5問に加え、New random では 5×5・10×10・15×15 の問題を生成できます。固定・ランダムともにすべて一意解です。行・列の数字は連続して塗るマスの数を順番に示し、数字の間には1マス以上の空白が必要です。0 の行・列はすべて空白です。Fill / Cross / Erase から操作を選んでクリック・タップし、同じ印をもう一度押すと消せます。押したままドラッグすると通過したマスをまとめて同じ印にでき、同じ印のマスから始めた場合はまとめて消去します。1回のドラッグは Undo 1回で戻せます。右クリックまたは右ドラッグでも×印を付けられます。塗った並びが数字に一致した行・列はヒントが薄くなりますが、それだけで各マスの正解を保証するものではありません。絵を正しく塗るとクリアで、空白すべてに×を付ける必要はありません。

| 操作 | 倉庫番 | ノノグラム |
| --- | --- | --- |
| 矢印 | 1マス移動 | 選択マスを移動 |
| WASD | 1マス移動 | — |
| ドラッグ | — | 通過したマスをまとめて塗る / ×印 / 消去 |
| Space / X | — | 塗る / ×印（再入力で消す） |
| Backspace / Delete | — | 選択マスを消す |
| Cmd・Ctrl + Z（+ Shift） | Undo（Redo） | Undo（Redo） |

New random のシードは空欄なら毎回ランダム、整数を指定すると同じサイズ・シードで同じ問題を再現できます。倉庫番は完成状態から箱を引く逆操作で生成し、その操作を逆にたどると必ずクリアできます。ノノグラムは数字だけから解を数え、一意性を証明できた問題を採用します。探索回数に上限を設け、上限時には必ず一意になる構造の問題を生成します。ランダムなノノグラムは抽象的な模様です。

問題選択、Restart、Undo / Redo、How to play を両ゲームに用意しています。プレイ中の問題変更・Restart は確認後に盤面と履歴をリセットします。Restart はランダム問題でも同じ盤面に戻ります。クリアすると次の固定問題、または新しいランダム問題へ進めます。リロード後の進行保存はありません。

## クレート構成

| クレート | 責務 |
| --- | --- |
| `klondike-core` | カード、配札、ルール、合法手、勝利判定、自動クリアの手順証明、履歴。GUI・GPU・時計・ファイル I/O への依存を持ちません |
| `klondike-egui` | 共通 UI ライブラリ。カード描画、入力、設定、ヒント、経過時間、自動移動アニメーション、WGSL の勝利演出を扱います |
| `klondike-desktop` | `rust-solitaire` 実行ファイル。CLI、OS 標準ウィンドウ、ネイティブ起動を扱います |
| `klondike-web` | WASM ライブラリ。ブラウザの canvas と `WebRunner` の起動・終了を扱います |
| `klondike-bevy` | Bevy 0.20 の3Dデスクトップ版。卓・カード・カメラ・入力・演出を担当し、ルールと履歴は `klondike-core` を直接使用します |
| `sudoku-core` | 数独の問題生成（一意解の保証）、ソルバー、入力・メモ・重複判定、ヒント、履歴 |
| `sudoku-web` | 数独の egui 画面。盤面描画、キーボード・キーパッド入力、経過時間 |
| `game2048-core` | 2048 のスライド・合体・得点、タイル出現、終了判定、履歴（乱数状態を含む） |
| `game2048-web` | 2048 の egui 画面。タイル描画、キー・スワイプ入力、ベストスコア |
| `minesweeper-core` | マインスイーパーの地雷配置（初手安全）、開放・連鎖・旗・コード、勝敗判定、公開情報だけを使うヒント推論 |
| `minesweeper-web` | マインスイーパーの egui 画面。盤面描画、クリック・右クリック・長押し入力、経過時間 |
| `sokoban-core` | 倉庫番の8ステージとシード付きの可解なランダム生成、移動・押し出し、クリア判定、履歴 |
| `sokoban-web` | 倉庫番の egui 画面。ステージ選択、キー・クリック・方向ボタン操作 |
| `nonogram-core` | ノノグラムの10問とシード付きの一意解ランダム生成、数字ヒント、マスの状態、クリア判定、履歴 |
| `nonogram-web` | ノノグラムの egui 画面。問題選択、塗り・×印・消去、ドラッグ操作、キーボード操作 |
| `games-web` | GitHub Pages 用の WASM 起動クレート。Cargo feature でゲームを1つずつ含めてビルドし、HTML の GAMES バーから必要なモジュールだけを読み込みます |

依存方向は既存の両方の起動クレート → `klondike-egui` → `klondike-core`、3D版は `klondike-bevy` → `klondike-core` です。追加したゲームは `<game>-web` → `<game>-core` で、`games-web` はゲームごとに必要な画面クレートだけを含む6つのモジュールとしてビルドします。各コアは `klondike-core` と同じく GUI・GPU・時計・I/O に依存せず、不正な操作では状態を変えません。ランダムな盤面を扱うコアは ChaCha8 のシードで決定的に動き、倉庫番とノノグラムは固定問題とシード付きランダム問題を使います。プラットフォーム固有の設定は起動クレートに置きます。時計には `web-time`、ブラウザの乱数には `getrandom` の `wasm_js` を使い、コアの決定的な配札は共有します。クロンダイクは macOS では Metal、Web では WebGPU を優先し、利用できない場合は WebGL にフォールバックします。他の5ゲームの Web 版は軽量な `egui_glow` の WebGL 描画を使います。

GUI は読み取り用の `Game::view()` を使い、変更は `Game::apply(Action)` を通します。無効な操作は盤面・手数・Undo / Redo 履歴を変更しません。ドラッグ中もコアは変えず、ドロップ時に操作を確定します。

```rust
use klondike_core::{Action, Game, Rules};

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
cargo build --release -p klondike-desktop --locked
cargo clippy -p games-web -p klondike-web --target wasm32-unknown-unknown --locked -- -D warnings
node --test web/tests/*.test.mjs
```

コアだけのテストにはウィンドウや GPU は必要ありません。

```sh
cargo test -p klondike-core -p sudoku-core -p game2048-core -p minesweeper-core -p sokoban-core -p nonogram-core --locked
cargo tree -p klondike-core --edges normal
```

ルールの境界条件、再巡回の順序、無効操作、勝利と履歴を検証します。プロパティテストでは複数のシードと操作列で52枚の不変条件と履歴の往復を確認します。GUI の入力テストは実際の egui のポインターイベントでクリックとドラッグを検証します。追加したゲームのコアは、数独の解の一意性、2048 のタイル総和の保存と履歴の往復、マインスイーパーの数字と地雷の整合、ヒントだけに従って遊んでも一度も地雷を開かないことをプロパティテストで確認し、画面クレートはキー・クリック・スワイプのイベントで操作を検証します。

倉庫番は全ステージを独立した幅優先探索で解き、クリアまでの操作と全 Undo / Redo を検証します。ノノグラムは行・列のヒントだけから解を数え、全10問の一意性、空行、誤った塗り、クリア・履歴を検証します。ランダム生成も複数のシード・全サイズで可解性、一意性、再現性、履歴とRestartを検証します。両画面では実際の egui 入力イベントでキー・クリック・右クリック・ドラッグとダイアログ中の入力抑制を確認します。遅延読み込みは、進行保持、連続切り替えの競合、失敗後の再試行、ロード中の終了を JavaScript のテストで確認します。

## Web 版のビルドと公開

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
bash scripts/build-web.sh
```

`scripts/build-web.sh` は `klondike` / `sudoku` / `2048` / `minesweeper` / `sokoban` / `nonogram` の feature を1つずつ指定し、`dist/pkg/<game>/` に独立した WASM と JavaScript を生成します。`web` プロファイルのサイズ最適化・LTO と不要な名前セクションの除去を使用します。ネイティブの release 設定は変更しません。CLI の場所を指定する場合は `WASM_BINDGEN=/path/to/wasm-bindgen bash scripts/build-web.sh` とします。

初回は選んだゲームのモジュールだけを読み込みます。他のゲームは最初の切り替え時に読み込み、以後は同じランナーを再利用するので盤面とUndo履歴を保持します。非表示ゲームへの入力・更新は止めます。各モジュールに描画エンジンが含まれるため、全6ゲームを開く場合の合計ダウンロード量は単一モジュールより増えます。

2026年10月12日のビルドで、変更前の全ゲーム入り WASM は 9.64 MiB（10,107,885 bytes）でした。分割・サイズ最適化・2Dパズルの描画バックエンド変更後の実測値は以下のとおりです。HTTP 圧縮前の WASM 本体だけを比較しています。

| モジュール | WASM サイズ | 変更前からの削減 |
| --- | ---: | ---: |
| Klondike | 7.32 MiB | 24.1% |
| Sudoku | 5.44 MiB | 43.6% |
| 2048 | 5.43 MiB | 43.7% |
| Minesweeper | 5.47 MiB | 43.2% |
| Sokoban | 5.46 MiB | 43.3% |
| Nonogram | 5.47 MiB | 43.3% |

生成した `dist/` を静的 HTTP サーバーで配信すると、ローカルでも遊べます。`rust-toolchain.toml` が WASM ターゲットを導入します。CLI のバージョンは `Cargo.lock` と一致させます。出力先は `--target-dir target` で明示し、Python や Cargo メタデータの解析は使いません。`dist/` は HTML・JavaScript・CSS・WASM だけの静的サイトです。パスは相対指定のため、GitHub Pages の `/rust-solitaire/` 以下でも動作します。ブラウザは JavaScript と GPU 描画が必要です。リロードすると進行中のゲームと 2048 のベストスコアは失われます。Pages には `games-web` を配置します。`klondike-web` はクロンダイク単体の WASM 起動クレートとして残しています。

`.github/workflows/pages.yml` は上記スクリプトで Web をビルドし、Pages へ公開します。macOS 上のテストとアプリ作成、Linux 上の WASM ビルドが両方成功した `main` の変更だけを GitHub Pages へ公開します。Pull request では検証だけを実行します。リポジトリの Settings → Pages → Source は **GitHub Actions** を使います。

## macOS アプリの作成

```sh
bash scripts/package-macos.sh 'target/Rust Solitaire.app'
```

`target/Rust Solitaire.app` を生成します。末尾が `.app` の任意の出力先も指定できます。アイコンと plist は `assets/macos/` の固定アセットを使い、アプリ全体を ad-hoc 署名します。バイナリの出力先は `CARGO_TARGET_DIR`、省略時は `target` です。パッケージのバージョンは workspace の `Cargo.toml` から取得します。

Bevy版も同じトランプのアイコンを持つアプリとして作成できます。

```sh
bash scripts/package-macos.sh --bevy 'target/Rust Solitaire Bevy.app'
```

macOS の Dock・Finder のアイコンは `.app` 内の `Info.plist` と `AppIcon.icns` で設定します。`cargo run` で直接起動する実行ファイルにはこのバンドル設定が適用されません。ローカル確認で開発ビルドを使う場合は `SOLITAIRE_BUILD_PROFILE=dev` を付けて実行できます。既定は release ビルドです。

### macOS インストール用 DMG

```sh
bash scripts/package-macos.sh
```

ビルドした Mac のアーキテクチャ向けに `target/rust-solitaire-0.1.0-arm64.dmg`（Apple Silicon の場合）と SHA-256 ファイルを生成します。出力先も指定できます。

```sh
bash scripts/package-macos.sh '/任意の出力先/rust-solitaire.dmg'
```

DMG を開き、`Rust Solitaire.app` を `Applications` にドラッグしてください。ディスクイメージを取り出したら、Applications から起動します。遊ぶ際に Rust や Cargo は不要です。macOS 11 以降が必要で、今回の検証済みパッケージは Apple Silicon 用です。

パッケージ作成には Rust / Cargo と macOS 標準ツールを使います。plist のバージョン設定は `PlistBuddy`、署名は `codesign`、DMG は `hdiutil`、SHA-256 は `shasum` で処理します。Python・Swift・アイコンの都度生成は不要です。アプリと DMG の作成は1本のスクリプトにまとめています。

現在の DMG はローカル利用向けの ad-hoc 署名です。Developer ID 署名と notarization は行っていません。他の Mac にダウンロードして一般配布する場合は、Apple の [Developer ID 署名・公証](https://developer.apple.com/developer-id/) を行う必要があります。

## 初版の範囲

ゲームの保存と再開、任意の盤面の解答ソルバー、勝てる配札の保証、スコア方式、サウンド、モバイル専用の操作・画面設計は含みません。Web の盤面は幅760ピクセル以上の表示領域を前提とします。終了すると進行中のゲームは失われます。Windows / Linux のネイティブ実機動作は未検証です。

計画は [docs/PLAN.md](docs/PLAN.md)、実際の検証結果は [docs/VALIDATION.md](docs/VALIDATION.md) に記載しています。

## 使用ライブラリ

- [egui と eframe](https://github.com/emilk/egui)
- [Bevy](https://bevy.org/)
- [wgpu](https://github.com/gfx-rs/wgpu)
- [rand と rand_chacha](https://github.com/rust-random/rand)
- [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen)
- [web-time](https://github.com/daxpedda/web-time)
- [proptest](https://github.com/proptest-rs/proptest)

カードの柄と絵札はオリジナルの図形です。egui版は直接描画し、Bevy版は生成済みPNGをメッシュへ貼り付けます。外部のカード素材や OS フォントは同梱しません。
