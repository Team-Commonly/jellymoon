#!/bin/sh
# Wraps a built `jellymoon-app` as Jellymoon.app and packs it beside the `jellymoon` binary into the archive that
# install.sh downloads: jellymoon-macos-<arch>.tar.gz, holding Jellymoon.app/ and jellymoon at its top. Both are signed
# ad hoc, with no Apple Developer identity: an arm64 Mac starts no program that carries no signature at all.
# The release workflow runs this, and so does F33's test (docs/development/install.md).
#
#   app/macos/bundle.sh <jellymoon-app binary> <jellymoon binary> <version> <arm64|x86_64> <output folder>
set -eu
die() { echo "bundle: $*" >&2; exit 1; }

[ $# -eq 5 ] || die "usage: bundle.sh <jellymoon-app binary> <jellymoon binary> <version> <arm64|x86_64> <output folder>"
app_binary=$1 cli=$2 version=$3 arch=$4 out=$5
[ "$(uname -s)" = Darwin ] || die "a Mac app bundle is made on a Mac"
case "$arch" in arm64 | x86_64) ;; *) die "the architecture is arm64 or x86_64, not $arch" ;; esac
# The version goes into a property list, so it may hold nothing a property list would read as markup.
case "$version" in '' | *[!0-9A-Za-z.-]*) die "the version may hold letters, digits, dots and dashes only" ;; esac
[ -f "$app_binary" ] || die "no file at $app_binary"
[ -f "$cli" ] || die "no file at $cli"
# The oldest macOS the app runs on is written once, in install.sh, which refuses an older Mac before it downloads.
min_macos=$(sed -n 's/^MIN_MACOS=//p' "$(dirname "$0")/../../install.sh")
case "$min_macos" in '' | *[!0-9.]*) die "install.sh has no MIN_MACOS=<number> line" ;; esac

stage=$(mktemp -d "${TMPDIR:-/tmp}/jellymoon-bundle.XXXXXX")
trap 'rm -rf "$stage"' EXIT
trap 'exit 1' HUP INT TERM
bundle=$stage/Jellymoon.app
mkdir -p "$bundle/Contents/MacOS" "$out"
cp "$app_binary" "$bundle/Contents/MacOS/jellymoon-app"
cp "$cli" "$stage/jellymoon"
chmod 755 "$bundle/Contents/MacOS/jellymoon-app" "$stage/jellymoon"

cat >"$bundle/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Jellymoon</string>
  <key>CFBundleDisplayName</key><string>Jellymoon</string>
  <key>CFBundleIdentifier</key><string>me.commonly.jellymoon</string>
  <key>CFBundleExecutable</key><string>jellymoon-app</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>$min_macos</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

codesign --force --sign - "$stage/jellymoon"
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"

# COPYFILE_DISABLE keeps the Mac's extended attributes and `._` files out of the archive.
COPYFILE_DISABLE=1 tar -czf "$out/jellymoon-macos-$arch.tar.gz" -C "$stage" Jellymoon.app jellymoon
echo "$out/jellymoon-macos-$arch.tar.gz"
