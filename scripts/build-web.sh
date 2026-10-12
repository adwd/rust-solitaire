#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

bindgen="${WASM_BINDGEN:-wasm-bindgen}"
mkdir -p dist/pkg
# Remove only the obsolete, generated all-games module from earlier builds.
rm -f dist/pkg/games_web.js dist/pkg/games_web_bg.wasm
for game in klondike sudoku 2048 minesweeper sokoban nonogram; do
  cargo build --profile web --locked --target wasm32-unknown-unknown \
    -p games-web --no-default-features --features "$game" --target-dir target
  "$bindgen" --target web --no-typescript --remove-name-section --remove-producers-section \
    --out-dir "dist/pkg/$game" target/wasm32-unknown-unknown/web/games_web.wasm
done
cp web/index.html web/style.css web/main.js web/lazy-games.mjs web/favicon.svg dist/
