#!/usr/bin/env bash
# Build a drag-to-Applications DMG for the current Mac architecture.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'This package script requires macOS.' >&2
    exit 1
fi
package_work="$(mktemp -d "${TMPDIR:-/tmp}/rust-solitaire-dmg.XXXXXX")"
trap 'rm -rf "$package_work"' EXIT
stage="$package_work/stage"
mkdir "$stage"
bash scripts/bundle-macos.sh "$stage/Rust Solitaire.app"
version="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$stage/Rust Solitaire.app/Contents/Info.plist")"
architecture="$(uname -m)"
package_path="${1:-target/rust-solitaire-$version-$architecture.dmg}"
case "$package_path" in
    *.dmg) ;;
    *) echo 'The package output path must end in .dmg' >&2; exit 1 ;;
esac
mkdir -p "$(dirname "$package_path")"
ln -s /Applications "$stage/Applications"
cat > "$stage/Install Rust Solitaire.txt" <<EOF
Rust Solitaire $version ($architecture)

1. Drag Rust Solitaire.app to Applications.
2. Eject the Rust Solitaire disk image.
3. Open Rust Solitaire from Applications.

Requires macOS 11 or later and a compatible Mac architecture.
Rust and Cargo are not required to play.

This local build is ad-hoc signed. It is not Developer ID signed or notarized.
EOF
hdiutil create -volname 'Rust Solitaire' -srcfolder "$stage" -fs HFS+ \
    -format UDZO "$package_work/package.dmg"
hdiutil verify "$package_work/package.dmg"
mv -f "$package_work/package.dmg" "$package_path"
python3 - "$package_path" <<'PY'
from pathlib import Path
import hashlib
import sys

package = Path(sys.argv[1]).resolve()
digest = hashlib.sha256(package.read_bytes()).hexdigest()
package.with_suffix('.dmg.sha256').write_text(f'{digest}  {package.name}\n')
print(package)
print(f'SHA-256: {digest}')
PY
