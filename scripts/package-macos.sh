#!/usr/bin/env bash
# Build a signed .app or a drag-to-Applications DMG using macOS tools.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'This package script requires macOS.' >&2
    exit 1
fi

build_dir="${CARGO_TARGET_DIR:-target}"
version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
output="${1:-$build_dir/rust-solitaire-$version-$(uname -m).dmg}"
case "$output" in
    *.app) bundle="$output" ;;
    *.dmg)
        package_work="$(mktemp -d "${TMPDIR:-/tmp}/rust-solitaire-dmg.XXXXXX")"
        trap 'rm -rf "$package_work"' EXIT
        stage="$package_work/stage"
        bundle="$stage/Rust Solitaire.app"
        ;;
    *) echo 'The output path must end in .app or .dmg' >&2; exit 1 ;;
esac

cargo build --release --locked -p solitaire-desktop --target-dir "$build_dir"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
# Replace the executable atomically so a running copy can keep its old inode.
cp -p "$build_dir/release/rust-solitaire" "$bundle/Contents/MacOS/rust-solitaire.new"
mv -f "$bundle/Contents/MacOS/rust-solitaire.new" "$bundle/Contents/MacOS/rust-solitaire"
cp assets/macos/AppIcon.icns "$bundle/Contents/Resources/"
cp assets/macos/Info.plist "$bundle/Contents/"
/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $version" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $version" "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"

if [[ "$output" == *.app ]]; then
    printf '%s\n' "$output"
    exit 0
fi

ln -s /Applications "$stage/Applications"
cp assets/macos/Install.txt "$stage/Install Rust Solitaire.txt"
mkdir -p "$(dirname "$output")"
hdiutil create -volname 'Rust Solitaire' -srcfolder "$stage" -fs HFS+ \
    -format UDZO "$package_work/package.dmg"
hdiutil verify "$package_work/package.dmg"
mv -f "$package_work/package.dmg" "$output"
(
    cd "$(dirname "$output")"
    shasum -a 256 "$(basename "$output")" > "$(basename "$output").sha256"
)
printf '%s\n' "$output"
