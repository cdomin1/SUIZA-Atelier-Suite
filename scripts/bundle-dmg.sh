#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
[ "$(uname -s)" = Darwin ] || { printf 'Build DMGs on macOS.\n' >&2; exit 1; }
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
target="${1:-universal}"
scripts/bundle-macos.sh release "$target"
case "$target" in
  x86_64-apple-darwin) arch=x86_64 ;;
  aarch64-apple-darwin) arch=arm64 ;;
  *) arch=universal ;;
esac
stage=$(mktemp -d "${TMPDIR:-/tmp}/suiza-dmg.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
ditto "dist/SUIZA Atelier Suite.app" "$stage/SUIZA Atelier Suite.app"
cp LICENSE "$stage/LICENSE.txt"
ln -s /Applications "$stage/Applications"
cat > "$stage/Read Me.txt" <<EOF
SUIZA Atelier Suite $version (Alpha) — macOS $arch

Drag SUIZA Atelier Suite into Applications to install.

This Alpha build is ad-hoc signed and is not Apple notarized.
On first launch, right-click the app and choose Open, or run:
  xattr -dr com.apple.quarantine "/Applications/SUIZA Atelier Suite.app"
EOF
output="dist/SUIZA-Atelier-Suite-${version}-macOS-${arch}.dmg"
hdiutil create -volname "SUIZA Atelier Suite $version" -srcfolder "$stage" -format UDZO -ov "$output"
hdiutil verify "$output"
(cd dist && shasum -a 256 "$(basename "$output")") > "$output.sha256"
printf 'Created %s\n' "$output"
