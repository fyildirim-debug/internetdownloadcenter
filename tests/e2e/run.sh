#!/bin/bash
# Uçtan uca yakalama testi (Linux, Xvfb gerekir).
#   cargo build -p internet-download-center -p idc-host --features internet-download-center/custom-protocol
#   node extension/build.mjs && tests/e2e/run.sh
# Denenenler: normal dosya (çok bağlantı), HttpOnly oturum çerezi isteyen dosya,
# indirme yöneticisini engelleyen tek kullanımlık bağlantı (tarayıcıya geri devretme).
set -e
T=$(cd "$(dirname "$0")"; pwd)
ROOT=$(cd "$T/../.."; pwd)
WORK=${WORK:-$(mktemp -d)}
export WORK
export HOME=$WORK/home XDG_CONFIG_HOME=$WORK/home/.config XDG_DATA_HOME=$WORK/home/.local/share
mkdir -p "$HOME/Downloads" "$HOME/.config/internet-download-center" "$HOME/.config/chromium/NativeMessagingHosts"
cat > "$HOME/.config/internet-download-center/settings.json" <<JSON
{"firstRunDone": true, "defaultSaveDir": "$HOME/Downloads",
 "downloads": {"showFileInfoDialog": false, "progressDialog": "none", "showCompleteDialog": false}}
JSON
cat > "$HOME/.config/chromium/NativeMessagingHosts/com.furkanyildirim.idc.json" <<JSON
{"name":"com.furkanyildirim.idc","description":"IDC","path":"$ROOT/target/debug/idc-host","type":"stdio","allowed_origins":["chrome-extension://khmbnekbmjoegchdepkfehmenkbfmbpd/"]}
JSON
node "$T/server.cjs" > "$WORK/server.log" 2>&1 & SRV=$!
Xvfb :91 -screen 0 1280x800x24 >/dev/null 2>&1 & XV=$!
trap 'kill $SRV $XV 2>/dev/null || true' EXIT
sleep 1
export DISPLAY=:91
cp "$T/e2e.cjs" "$WORK/e2e.cjs"
dbus-run-session -- bash -c "
  '$ROOT/target/debug/internet-download-center' > '$WORK/app.log' 2>&1 & APP=\$!
  sleep 4
  NODE_PATH='$ROOT/node_modules' node '$T/e2e.cjs'
  kill \$APP
"
