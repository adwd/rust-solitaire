#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
bindgen="${WASM_BINDGEN:-wasm-bindgen}"
if ! command -v "$bindgen" >/dev/null; then
    echo 'Install wasm-bindgen-cli 0.2.129: cargo install wasm-bindgen-cli --version 0.2.129 --locked' >&2
    exit 1
fi
if [ "$("$bindgen" --version)" != 'wasm-bindgen 0.2.129' ]; then
    echo 'wasm-bindgen-cli must match Cargo.lock: version 0.2.129 is required.' >&2
    exit 1
fi
cargo build --release --locked --target wasm32-unknown-unknown -p solitaire-web
web_output="${1:-dist}"
mkdir -p "$web_output/pkg"
wasm_path="$(python3 - <<'PY'
import json
import subprocess
from pathlib import Path
metadata = json.loads(subprocess.check_output([
    'cargo', 'metadata', '--format-version', '1', '--no-deps', '--locked'
]))
print(Path(metadata['target_directory']) / 'wasm32-unknown-unknown/release/solitaire_web.wasm')
PY
)"
"$bindgen" --target web --no-typescript --out-dir "$web_output/pkg" "$wasm_path"
cp web/index.html web/main.js web/style.css web/favicon.svg "$web_output/"
touch "$web_output/.nojekyll"
printf 'Web build: %s\n' "$web_output"
