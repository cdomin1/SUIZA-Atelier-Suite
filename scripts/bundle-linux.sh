#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
bundle=dist/suiza-atelier-linux
mkdir -p "$bundle/bin" "$bundle/share/applications"
cp target/release/suiza-atelier-suite "$bundle/bin/suiza-atelier"
mkdir -p "$bundle/share/licenses/suiza-atelier"
cp LICENSE "$bundle/share/licenses/suiza-atelier/"
cp packaging/suiza-atelier.desktop "$bundle/share/applications/suiza-atelier.desktop"
mkdir -p "$bundle/share/mime/packages"
cp packaging/suiza-szt.xml "$bundle/share/mime/packages/suiza-szt.xml"
for size in 16 24 32 48 64 128 256 512 1024; do
    folder="$bundle/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$folder"
    cp "assets/icons/$size/suiza.png" "$folder/suiza-atelier.png"
done
printf 'Created %s\n' "$bundle"
