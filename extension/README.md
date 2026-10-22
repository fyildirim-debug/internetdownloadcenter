# IDC tarayıcı eklentisi

Internet Download Center'ın Chrome/Edge/Brave/Opera/Vivaldi (Manifest V3) ve
Firefox/Zen/LibreWolf/Floorp eklentisi. İndirmeleri yakalar, videolara "IDC ile indir"
paneli ekler ve uygulamayla yerel mesajlaşma (`com.furkanyildirim.idc`) üzerinden konuşur.

## Derleme

```sh
node extension/build.mjs
```

| Çıktı | Kullanım |
|---|---|
| `build/chromium/` | Chromium tabanlı tarayıcılar, "Paketlenmemiş öğe yükle" |
| `build/firefox/` | Firefox ailesi (`about:debugging` → geçici eklenti) |
| `build/idc-chromium.zip` | Mağaza yüklemesi |
| `build/idc-firefox.xpi` | AMO imzası sonrası kurulum |

Kimlikler `ids.json` içindedir. Chromium kimliği manifestteki sabit `key` alanından gelir
(`khmbnekbmjoegchdepkfehmenkbfmbpd`); yerel mesajlaşma bildirimindeki `allowed_origins` bu kimliği kullanır.
Firefox kimliği `idc@furkanyildirim.com`'dur (`allowed_extensions`).

## Kurulum

* **Chrome/Edge/Brave/Opera/Vivaldi:** `chrome://extensions` → Geliştirici modu → Paketlenmemiş öğe yükle →
  IDC kurulum klasöründeki `extension/chromium`.
* **Firefox:** imzalı `.xpi` gerekir (`web-ext sign --channel unlisted`). Geliştirici sürümü, Nightly,
  Zen ve LibreWolf'ta `about:config` → `xpinstall.signatures.required = false` ile imzasız `.xpi` kurulabilir.
* Tarayıcı ayarlarındaki **"Her dosyanın nereye kaydedileceğini sor"** seçeneği kapalı olmalıdır; açıkken
  kaydetme penceresi yakalamadan önce görünebilir.

## Nasıl çalışır

* `downloads.onCreated` → kurallara uyan indirme duraklatılır → çerezler (HttpOnly ve CHIPS dahil) ve
  tarayıcının gerçekten gönderdiği başlıklar (`webRequest.onSendHeaders`) uygulamaya gönderilir →
  kabul edilirse tarayıcı indirmesi iptal edilip listeden silinir, reddedilirse kaldığı yerden sürer.
* Firefox'ta ek olarak `webRequest.onHeadersReceived` (blocking) yolu indirmeyi hiç başlamadan alır;
  uygulama reddederse dosya tarayıcıyla yeniden indirilir. Uygulama son 2 dakikada cevap vermediyse bu yol
  kullanılmaz.
* POST ile başlayan indirmeler, `blob:`/`data:`/`file:` adresleri ve atlama tuşu (varsayılan Alt) basılıyken
  tıklananlar tarayıcıda kalır. Zorlama tuşu ya da sağ tık menüsü kuralları atlar.
* Çok küçük dosyalar duraklatılamadan tamamlanabilir; bu durumda tarayıcıdaki dosyaya dokunulmaz.
