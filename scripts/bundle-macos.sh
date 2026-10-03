#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'This bundle script requires macOS.' >&2
    exit 1
fi
cargo build --release --locked -p solitaire-egui
bundle_path="${1:-target/Rust Solitaire.app}"
icon_work="$(mktemp -d "${TMPDIR:-/tmp}/rust-solitaire-icon.XXXXXX")"
trap 'rm -rf "$icon_work"' EXIT
mkdir "$icon_work/AppIcon.iconset"
xcrun swift -module-cache-path "$icon_work/modules" scripts/generate-icon.swift "$icon_work/AppIcon.iconset"
iconutil -c icns "$icon_work/AppIcon.iconset" -o "$icon_work/AppIcon.icns"
python3 - "$bundle_path" "$icon_work/AppIcon.icns" <<'PY'
from pathlib import Path
import plistlib
import shutil
import json
import subprocess
import sys

metadata = json.loads(subprocess.check_output([
    'cargo', 'metadata', '--format-version', '1', '--no-deps', '--locked'
]))
version = next(p['version'] for p in metadata['packages'] if p['name'] == 'solitaire-egui')
bundle = Path(sys.argv[1]).resolve()
if bundle.suffix != '.app':
    raise SystemExit('The bundle output path must end in .app')
(bundle / 'Contents/MacOS').mkdir(parents=True, exist_ok=True)
(bundle / 'Contents/Resources').mkdir(parents=True, exist_ok=True)
executable = bundle / 'Contents/MacOS/rust-solitaire'
# Replace atomically; an already running copy can finish using its old inode.
temporary = executable.with_suffix('.new')
shutil.copy2(Path(metadata['target_directory']) / 'release/rust-solitaire', temporary)
temporary.replace(executable)
shutil.copy2(sys.argv[2], bundle / 'Contents/Resources/AppIcon.icns')
info = {
    'CFBundleName': 'Rust Solitaire',
    'CFBundleDisplayName': 'Rust Solitaire',
    'CFBundleIdentifier': 'adwd.rust-solitaire',
    'CFBundleExecutable': 'rust-solitaire',
    'CFBundleVersion': version,
    'CFBundleShortVersionString': version,
    'CFBundlePackageType': 'APPL',
    'NSHighResolutionCapable': True,
    'LSMinimumSystemVersion': '11.0',
    'CFBundleIconFile': 'AppIcon.icns',
}
with (bundle / 'Contents/Info.plist').open('wb') as output:
    plistlib.dump(info, output)
print(bundle)
PY
codesign --force --sign - "$bundle_path"
codesign --verify --strict "$bundle_path"
