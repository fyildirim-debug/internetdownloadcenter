#!/bin/sh
# Tarayıcı köprüsünü sistem genelinde kaydet (tüm kullanıcılar ve tüm tarayıcılar).
set -e
HOST=/usr/bin/idc-host
NAME=com.furkanyildirim.idc
CHROMIUM_ID=khmbnekbmjoegchdepkfehmenkbfmbpd
GECKO_ID=idc@furkanyildirim.com

chromium_manifest() {
  printf '{\n  "name": "%s",\n  "description": "Internet Download Center",\n  "path": "%s",\n  "type": "stdio",\n  "allowed_origins": ["chrome-extension://%s/"]\n}\n' "$NAME" "$HOST" "$CHROMIUM_ID"
}
firefox_manifest() {
  printf '{\n  "name": "%s",\n  "description": "Internet Download Center",\n  "path": "%s",\n  "type": "stdio",\n  "allowed_extensions": ["%s"]\n}\n' "$NAME" "$HOST" "$GECKO_ID"
}

if [ -x "$HOST" ]; then
  for d in /etc/opt/chrome/native-messaging-hosts /etc/chromium/native-messaging-hosts /etc/opt/edge/native-messaging-hosts /etc/opt/brave/native-messaging-hosts /etc/opt/vivaldi/native-messaging-hosts /etc/opt/opera/native-messaging-hosts; do
    mkdir -p "$d" && chromium_manifest > "$d/$NAME.json" || true
  done
  for d in /usr/lib/mozilla/native-messaging-hosts /usr/lib64/mozilla/native-messaging-hosts /usr/lib/librewolf/native-messaging-hosts /usr/lib/zen/native-messaging-hosts; do
    mkdir -p "$d" && firefox_manifest > "$d/$NAME.json" || true
  done
fi

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q /usr/share/applications || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t /usr/share/icons/hicolor || true
exit 0
