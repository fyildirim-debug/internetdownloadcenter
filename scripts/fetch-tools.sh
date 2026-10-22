#!/bin/sh
# Kurulum paketine gömülecek yt-dlp'yi src-tauri/tools altına indirir.
# Kullanım: scripts/fetch-tools.sh [linux|windows]
set -e
cd "$(dirname "$0")/.."
OS=${1:-linux}
mkdir -p src-tauri/tools
case "$OS" in
  windows) curl -fL -o src-tauri/tools/yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe ;;
  *) curl -fL -o src-tauri/tools/yt-dlp https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux && chmod 755 src-tauri/tools/yt-dlp ;;
esac
ls -la src-tauri/tools
