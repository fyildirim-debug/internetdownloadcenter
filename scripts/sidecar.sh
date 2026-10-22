#!/bin/sh
# idc-host köprüsünü derleyip Tauri'nin beklediği adla (hedef üçlüsü ekli) src-tauri/binaries altına koyar.
# Kullanım: scripts/sidecar.sh [release|debug] [hedef-üçlüsü]
set -e
PROFILE=${1:-release}
TRIPLE=${2:-$(rustc -vV | sed -n 's/^host: //p')}
cd "$(dirname "$0")/.."
if [ "$PROFILE" = "release" ]; then
  cargo build -p idc-host --release --target "$TRIPLE"
else
  cargo build -p idc-host --target "$TRIPLE"
fi
EXT=""
case "$TRIPLE" in *windows*) EXT=".exe" ;; esac
mkdir -p src-tauri/binaries
cp "target/$TRIPLE/$PROFILE/idc-host$EXT" "src-tauri/binaries/idc-host-$TRIPLE$EXT"
echo "src-tauri/binaries/idc-host-$TRIPLE$EXT"
