#!/bin/sh
# Kaldırmada sistem geneli köprü kayıtlarını sil (yükseltmede dokunma).
case "$1" in
  upgrade|1) exit 0 ;;
esac
NAME=com.furkanyildirim.idc
for d in /etc/opt/chrome/native-messaging-hosts /etc/chromium/native-messaging-hosts /etc/opt/edge/native-messaging-hosts /etc/opt/brave/native-messaging-hosts /etc/opt/vivaldi/native-messaging-hosts /etc/opt/opera/native-messaging-hosts /usr/lib/mozilla/native-messaging-hosts /usr/lib64/mozilla/native-messaging-hosts /usr/lib/librewolf/native-messaging-hosts /usr/lib/zen/native-messaging-hosts; do
  rm -f "$d/$NAME.json" 2>/dev/null || true
done
exit 0
