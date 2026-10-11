#!/usr/bin/env bash
# Build a signed .app or a drag-to-Applications DMG using macOS tools.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ]; then
    echo 'This package script requires macOS.' >&2
    exit 1
fi

frontend="solitaire-desktop"
executable="rust-solitaire"
app_name="Rust Solitaire"
bundle_id="adwd.rust-solitaire"
if [ "${1:-}" = --bevy ]; then
    frontend="solitaire-bevy"
    executable="solitaire-bevy"
    app_name="Rust Solitaire Bevy"
    bundle_id="adwd.rust-solitaire.bevy"
    shift
fi

# A development build lets local previews reuse Cargo's existing debug cache.
profile="${SOLITAIRE_BUILD_PROFILE:-release}"
case "$profile" in
    release) profile_dir=release ;;
    dev) profile_dir=debug ;;
    *) echo 'SOLITAIRE_BUILD_PROFILE must be release or dev' >&2; exit 1 ;;
esac
build_dir="${CARGO_TARGET_DIR:-target}"
version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
output="${1:-$build_dir/$executable-$version-$(uname -m).dmg}"
case "$output" in
    *.app) bundle="$output" ;;
    *.dmg)
        package_work="$(mktemp -d "${TMPDIR:-/tmp}/rust-solitaire-dmg.XXXXXX")"
        trap 'rm -rf "$package_work"' EXIT
        stage="$package_work/stage"
        bundle="$stage/$app_name.app"
        ;;
    *) echo 'The output path must end in .app or .dmg' >&2; exit 1 ;;
esac

cargo build --profile "$profile" --locked -p "$frontend" --target-dir "$build_dir"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
# Replace the executable atomically so a running copy can keep its old inode.
cp -p "$build_dir/$profile_dir/$executable" "$bundle/Contents/MacOS/$executable.new"
mv -f "$bundle/Contents/MacOS/$executable.new" "$bundle/Contents/MacOS/$executable"
cp assets/macos/AppIcon.icns "$bundle/Contents/Resources/"
cp assets/macos/Info.plist "$bundle/Contents/"
/usr/libexec/PlistBuddy -c "Set :CFBundleName $app_name" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName $app_name" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $bundle_id" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $executable" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $version" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $version" "$bundle/Contents/Info.plist"
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"

if [[ "$output" == *.app ]]; then
    printf '%s\n' "$output"
    exit 0
fi

ln -s /Applications "$stage/Applications"
sed "s/Rust Solitaire/$app_name/g" assets/macos/Install.txt > "$stage/Install $app_name.txt"
mkdir -p "$(dirname "$output")"
hdiutil create -volname "$app_name" -srcfolder "$stage" -fs HFS+ \
    -format UDZO "$package_work/package.dmg"
hdiutil verify "$package_work/package.dmg"
mv -f "$package_work/package.dmg" "$output"
(
    cd "$(dirname "$output")"
    shasum -a 256 "$(basename "$output")" > "$(basename "$output").sha256"
)
printf '%s\n' "$output"
