#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'This bundle script requires macOS.' >&2
    exit 1
fi
cargo build --release --locked -p solitaire-egui
bundle_path="${1:-target/Rust Solitaire.app}"
python3 - "$bundle_path" <<'PY'
from pathlib import Path
import plistlib
import shutil
import sys

bundle = Path(sys.argv[1]).resolve()
(bundle / 'Contents/MacOS').mkdir(parents=True, exist_ok=True)
executable = bundle / 'Contents/MacOS/rust-solitaire'
# Replace atomically; an already running copy can finish using its old inode.
temporary = executable.with_suffix('.new')
shutil.copy2('target/release/rust-solitaire', temporary)
temporary.replace(executable)
info = {
    'CFBundleName': 'Rust Solitaire',
    'CFBundleDisplayName': 'Rust Solitaire',
    'CFBundleIdentifier': 'adwd.rust-solitaire',
    'CFBundleExecutable': 'rust-solitaire',
    'CFBundleVersion': '1',
    'CFBundleShortVersionString': '0.1.0',
    'CFBundlePackageType': 'APPL',
    'NSHighResolutionCapable': True,
    'LSMinimumSystemVersion': '11.0',
}
with (bundle / 'Contents/Info.plist').open('wb') as output:
    plistlib.dump(info, output)
print(bundle)
PY
