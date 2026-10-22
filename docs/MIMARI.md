# Internet Download Center — Mimari

Internet Download Center (IDC), Internet Download Manager'dan esinlenen, Windows ve Linux için
yazılmış bağımsız bir indirme yöneticisidir. Bu belge bileşenleri, aralarındaki sözleşmeleri ve
alınan kararları tanımlar. Kodla çelişkide kod esastır; belge güncellenir.

## Bileşenler

```
internetdownloadcenter/
├── crates/idc-core      İndirme motoru (Rust kütüphanesi, GUI'den bağımsız, testli)
├── crates/idc-host      Tarayıcı yerel mesajlaşma köprüsü (küçük konsol ikilisi)
├── src-tauri            Masaüstü uygulaması (Tauri 2): komutlar, pencereler, tepsi, yerel API
├── ui                   Arayüz (Svelte 5 + TypeScript + Vite), IDM görünümlü çoklu pencere
├── extension            Chrome/Edge/Brave/Opera/Vivaldi (MV3) ve Firefox/Zen/LibreWolf eklentisi
├── packaging            Linux .desktop, simgeler, deb/rpm betikleri, NSIS kancaları
└── .github/workflows    CI ve sürüm derlemeleri (NSIS, zip, AppImage, deb, rpm, eklenti zip/xpi)
```

### Veri akışı

```
Tarayıcı eklentisi ──sendNativeMessage──▶ idc-host ──HTTP 127.0.0.1 + belirteç──▶ IDC uygulaması
                                                                                     │
       Svelte arayüzü ◀──Tauri olayları── idc-core motoru ◀──Tauri komutları─────────┘
```

* Uygulama açılışta `127.0.0.1` üzerinde rastgele bir portta yerel API başlatır ve
  `<veri dizini>/ipc.json` dosyasına `{ "port": 49152, "token": "<64 hex>", "pid": 1234 }` yazar.
* `idc-host` tarayıcı tarafından her mesajda başlatılır, `ipc.json`'u okur, isteği
  `POST /api/ext` ile iletir, cevabı tarayıcıya döner. Uygulama çalışmıyorsa `--background`
  ile başlatır ve 10 sn'ye kadar `ipc.json`'un oluşmasını bekler.
* Uygulama tek örnektir (single-instance). İkinci çalıştırma argümanlarını (URL) ilk örneğe iletir.

## Veri dizinleri

| | Linux | Windows |
|---|---|---|
| Ayarlar + liste | `~/.config/internet-download-center/` | `%APPDATA%\InternetDownloadCenter\` |
| Araçlar (yt-dlp, ffmpeg, deno) | `~/.local/share/internet-download-center/tools/` | `%LOCALAPPDATA%\InternetDownloadCenter\tools\` |
| Varsayılan indirme klasörü | `~/İndirilenler` (XDG_DOWNLOAD_DIR) | `%USERPROFILE%\Downloads` |

Dosyalar: `settings.json`, `downloads.json`, `ipc.json`. Yazımlar atomiktir (geçici dosya + rename).

## İndirme motoru (idc-core)

### Dinamik bölümleme (IDM tarzı)

1. İlk bağlantı `Range: bytes=0-` ile açılır. Cevap başlıklarından boyut, sürdürme desteği
   (206 + `Content-Range` ya da `Accept-Ranges: bytes`), dosya adı (`Content-Disposition`, RFC 6266
   `filename*` dahil), MIME, `ETag` ve `Last-Modified` okunur.
2. Sürdürülebilir ve boyutu bilinen dosyada geri kalan bağlantılar en büyük kalan parçayı ortadan
   bölerek açılır (varsayılan 8 bağlantı, 1–16 arası, ayarlanabilir).
3. Bir bağlantı parçasını bitirince en büyük kalan parçayı ikiye böler ve ikinci yarıyı üstlenir
   (iş çalma). Parça bölme eşiği 2 × 512 KiB; daha küçük parça bölünmez.
4. Veri doğrudan, önceden ayrılmış tek bir `<ad>.idcpart` dosyasına konumlu yazımla (`pwrite`)
   yazılır; birleştirme adımı yoktur. Bitince dosya `<ad>` olarak yeniden adlandırılır.
5. Parça konumları her 2 sn'de bir `downloads.json`'a işlenir; elektrik kesintisinde bile kaldığı
   yerden devam eder. Devamda `ETag`/`Last-Modified`/boyut değiştiyse kullanıcıya bildirilir.

### Engellemeye karşı katmanlar

Birçok site indirme yöneticilerini engeller. IDC şu sırayla dener:

1. **Tarayıcı oturumunun birebir kopyası**: eklenti isteğin gerçek başlıklarını
   (`webRequest.onBeforeSendHeaders`, `extraHeaders`), HttpOnly dahil çerezleri (`cookies` API),
   `User-Agent`, `Referer`, `sec-ch-ua*`, `Authorization` gibi başlıkları gönderir. Motor çerezleri
   alan adına göre bir çerez kavanozuna koyar; yönlendirmelerde doğru alan adına doğru çerez gider.
2. **Uyarlanır bağlantı sayısı**: ek bağlantılar 403/429/503 alırsa bağlantı sayısı düşürülür ve
   parça geri havuza konur; gerekirse tek bağlantıya iner.
3. **Kimliğe bürünme**: doğrudan istek HTML/engel sayfası döndürürse (Cloudflare, TLS parmak izi)
   indirme `yt-dlp --impersonate chrome` ile yeniden denenir.
4. **Tarayıcıya geri devretme**: hepsi başarısız olursa eklentiye `rejected` döner, eklenti
   duraklattığı tarayıcı indirmesini sürdürür. Kullanıcı hiçbir indirme kaybetmez.
5. **Adres yenileme**: süresi dolan bağlantıda (403/410) kullanıcı tarayıcıda aynı dosyaya tekrar
   tıklarsa IDC aynı ad + boyuttaki yarım indirmeyi bulur ve adresini güncelleyip devam eder.

### Video

* Doğrudan medya (mp4/webm/mp3...) eklentinin ağ gözleminden yakalanır ve normal motorla iner.
* YouTube, Vimeo, X, Instagram, TikTok vb. ile HLS (`.m3u8`) / DASH (`.mpd`) akışları `yt-dlp` ile
  iner. Biçim listesi `yt-dlp -J` ile alınır; çözünürlük, FPS, kap ve boyut gösterilir. Ses
  (M4A/MP3) seçeneği vardır. Bağlantı sayısı `-N` ile parça sayısına eşlenir.
* Araç yöneticisi `yt-dlp`, `ffmpeg` ve `deno`'yu (YouTube imza çözümü için) ilk ihtiyaçta indirir
  ve günceller. Linux'ta sistemde varsa sistemdekini kullanır.

## Tauri komutları (arayüz → çekirdek)

Tüm alan adları JSON'da camelCase'dir.

| Komut | Parametreler | Döner |
|---|---|---|
| `list_downloads` | – | `DownloadView[]` |
| `get_download` | `id` | `DownloadDetail` |
| `probe_url` | `url, headers?` | `ProbeResult` |
| `add_download` | `req: AddRequest` | `string` (id) |
| `start_downloads` / `pause_downloads` | `ids: string[]` | – |
| `remove_downloads` | `ids, deleteFiles: bool` | – |
| `start_all` / `stop_all` / `delete_completed` | – | – |
| `redownload` | `id` | – |
| `update_download` | `id, patch: DownloadPatch` | – |
| `open_file` / `open_folder` | `id` | – |
| `get_settings` / `save_settings` | `settings` | `Settings` |
| `media_probe` | `url, headers?` | `MediaProbe` |
| `tools_status` / `tools_update` | – | `ToolsStatus` |
| `browsers_status` / `browsers_install` | `ids?` | `BrowserInfo[]` |
| `os_integration_status` / `os_integration_apply` | `opts` | `OsIntegration` |
| `open_window` | `kind, params` | – |
| `start_queue` / `stop_queue` | `queueId` | – |
| `batch_preview` | `pattern, from, to, step, padding` | `string[]` |
| `grab_links` | `url, depth, extensions` | `GrabbedLink[]` |
| `app_info` | – | `{ version, os, dataDir, firstRun }` |
| `quit_app` | – | – |

### Olaylar (çekirdek → arayüz)

* `idc://progress` → `ProgressTick[]` (saniyede 4 kez, yalnız etkin indirmeler)
* `idc://changed` → `DownloadView` (durum/ad/boyut değişince)
* `idc://removed` → `string[]`
* `idc://settings` → `Settings`

## Eklenti protokolü

Eklenti `chrome.runtime.sendNativeMessage("com.furkanyildirim.idc", msg)` ile tek mesaj gönderir.
Ana alanlar: `type`, `reqId`. Uygulama aynı `reqId` ile döner.

| `type` | Gönderilen | Cevap |
|---|---|---|
| `hello` | `version, browser` | `{ type: "config", config: ExtConfig, appVersion }` |
| `download` | `DownloadCapture` | `{ type: "accepted", id }` ya da `{ type: "rejected", reason }` |
| `media_probe` | `pageUrl, mediaUrl?, headers, cookies` | `{ type: "media", probe: MediaProbe }` |
| `media_download` | `pageUrl, mediaUrl?, formatId, audioOnly, headers, cookies, title` | `accepted/rejected` |
| `links` | `links: {url, text}[], referrer` | `accepted` (seçim penceresi açılır) |
| `show` | – | uygulama ön plana gelir |

`rejected.reason`: `disabled`, `excluded`, `blocked`, `unsupported`, `user`.
