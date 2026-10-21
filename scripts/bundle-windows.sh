#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
cargo build --release --locked
bundle="dist/suiza-atelier-windows"
rm -rf "$bundle"
mkdir -p "$bundle"
cp target/release/suiza-atelier-suite.exe "$bundle/SUIZA Atelier Suite.exe"
cp assets/suiza.ico "$bundle/suiza.ico"
cp LICENSE "$bundle/LICENSE.txt"
cat > "$bundle/Read Me.txt" <<EOF
SUIZA Atelier Suite $version (Alpha) — Windows x86_64

Run "SUIZA Atelier Suite.exe" directly — no installer required.
EOF
(cd dist && zip -qr "SUIZA-Atelier-Suite-${version}-Windows-x86_64.zip" "$(basename "$bundle")")
printf 'Created dist/SUIZA-Atelier-Suite-%s-Windows-x86_64.zip\n' "$version"
