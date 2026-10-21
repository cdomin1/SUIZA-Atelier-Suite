#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile="${1:-release}"
target="${2:-}"
case "$target" in
  ""|aarch64-apple-darwin|x86_64-apple-darwin|universal) ;;
  *) printf 'Unsupported macOS target: %s\n' "$target" >&2; exit 2 ;;
esac
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
case "$profile" in
  release|debug) ;;
  *) printf 'Usage: %s [release|debug] [aarch64-apple-darwin|x86_64-apple-darwin|universal]\n' "$0" >&2; exit 2 ;;
esac

bundle="dist/SUIZA Atelier Suite.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"

if [ "$target" = universal ]; then
  cargo build --release --locked --target aarch64-apple-darwin
  cargo build --release --locked --target x86_64-apple-darwin
  lipo -create \
    target/aarch64-apple-darwin/release/suiza-atelier-suite \
    target/x86_64-apple-darwin/release/suiza-atelier-suite \
    -output "$bundle/Contents/MacOS/SUIZA Atelier.new"
else
  if [ -n "$target" ]; then set -- --target "$target"; else set --; fi
  case "$profile" in
    release) cargo build --release --locked "$@" ;;
    debug) cargo build --locked "$@" ;;
  esac
  binary_dir="target/$profile"
  [ -z "$target" ] || binary_dir="target/$target/$profile"
  cp "$binary_dir/suiza-atelier-suite" "$bundle/Contents/MacOS/SUIZA Atelier.new"
fi

# Replace the executable with a new inode: overwriting a previously launched
# Mach-O can leave macOS with cached code-signing pages from the old build.
rm -f "$bundle/Contents/MacOS/SUIZA Atelier"
mv -f "$bundle/Contents/MacOS/SUIZA Atelier.new" "$bundle/Contents/MacOS/SUIZA Atelier"

cp assets/suiza.icns "$bundle/Contents/Resources/Suiza.icns"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>SUIZA Atelier Suite</string>
<key>CFBundleDisplayName</key><string>SUIZA Atelier Suite</string>
<key>CFBundleExecutable</key><string>SUIZA Atelier</string>
<key>CFBundleIdentifier</key><string>local.suiza.atelier</string>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>SUIZA Atelier Document</string>
<key>CFBundleTypeRole</key><string>Editor</string>
<key>CFBundleTypeExtensions</key><array><string>szt</string></array>
<key>CFBundleTypeIconFile</key><string>Suiza.icns</string>
</dict></array>
<key>CFBundleIconFile</key><string>Suiza.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>${version}</string>
<key>CFBundleVersion</key><string>${version}</string>
<key>CFBundleGetInfoString</key><string>SUIZA Atelier Suite ${version}</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

# Local builds use an ad-hoc signature; distribution signing is separate.
codesign --force --sign - "$bundle"
codesign --verify --deep --strict "$bundle"
printf 'Created %s\n' "$bundle"
