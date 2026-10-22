# Test kayıtları

Son güncelleme: 2026-10-22. Ortam: Ubuntu 24.04 (x86_64) bulut makinesi, Xvfb sanal ekran,
Chromium 1194 (Playwright), Rust 1.97, Node 22.

## Otomatik testler

`cargo test --workspace` komutu 24 test çalıştırır.

### Motorun uçtan uca testleri (`crates/idc-core/tests/engine.rs`)

Testler yerel bir HTTP sunucusuna karşı gerçek indirme yapar ve dosya içeriğini bayt bayt
karşılaştırır.

| Test | Senaryo | Sonuç |
|---|---|---|
| `segmented_download` | 9 MB, 8 bağlantı, dinamik bölme, RFC 5987 dosya adı (`örnek dosya.bin`) | Geçti |
| `server_ignores_range` | Sunucu `Accept-Ranges` diyor ama aralığı yok sayıyor; tek bağlantıya düşer | Geçti |
| `adaptive_connection_limit` | Sunucu 2'den fazla bağlantıya 429 dönüyor; bağlantı sayısı düşer | Geçti |
| `dropped_connections_retry` | İlk iki bağlantı yarıda kopuyor; yeniden denenir | Geçti |
| `unknown_size_stream` | Boyut bildirmeyen, chunked akış | Geçti |
| `zero_byte_file` | 0 baytlık dosya (416 `*/0`) | Geçti |
| `cookies_across_redirect` | Tarayıcı çerezi + yönlendirmede gelen `Set-Cookie`, ikisi de gerekli | Geçti |
| `pause_and_resume` | Duraklat, motoru yeniden kur (uygulama yeniden açılmış gibi), kaldığı yerden sürdür | Geçti |
| `duplicate_names_are_unique` | Aynı adlı iki indirme `ad (2).bin` olur | Geçti |

### Birim testleri

- `Content-Disposition` ayrıştırma (`filename*`, tırnak içinde `;`, dizin kaçışı engeli)
- Dosya adı güvenliği (Windows'a ayrılmış adlar, geçersiz karakterler)
- Joker eşleştirme, site kuralları
- Hız sınırlayıcı (100 KB/sn'de 2 saniye ±%25)
- yt-dlp biçim listesinin kalite seçeneklerine dönüşmesi
- Toplu indirme deseni

### Diğer otomatik denetimler

`npm --prefix ui run check` (svelte-check) 142 dosyada 0 hata ve 0 uyarı verdi.

`web-ext lint` (Firefox eklentisi) 0 hata ve 0 uyarı verdi.

## Gerçek uygulama ile uçtan uca test

Komut: `tests/e2e/run.sh`. Gerçek IDC uygulaması, gerçek `idc-host` köprüsü ve eklentisi yüklü
Chromium birlikte çalıştırıldı.

| Senaryo | Beklenen | Sonuç |
|---|---|---|
| Normal dosyaya tıklama (8 MB) | Tarayıcı indirmesi iptal edilir, IDC çok bağlantıyla indirir | Geçti: SHA-256 eşleşti, tarayıcı listesi temiz |
| HttpOnly oturum çerezi gerektiren dosya | IDC çerezle indirir | Geçti: 5 isteğin hepsinde çerez vardı |
| İndirme yöneticisini engelleyen tek kullanımlık bağlantı | IDC reddeder, tarayıcı indirmeyi sürdürür | Geçti: tarayıcıda tamamlandı (2 MB) |
| Uygulama kapalıyken tıklama | Köprü uygulamayı başlatır, IDC indirir | Geçti (1,1 sn) |
| Dosya bilgisi penceresi açıkken yakalama | Pencere açılır; boyut, sürdürme desteği ve kategori dolu gelir | Geçti (ekran görüntüsüyle) |
| İlerleme penceresi | Hız, kalan süre ve parça çubuğu canlı güncellenir | Geçti (ekran görüntüsüyle) |

## Video

| Senaryo | Sonuç |
|---|---|
| Araç yöneticisi: yt-dlp ve deno'nun GitHub'dan indirilmesi, ffmpeg'in sistemde bulunması | Geçti |
| HLS akışı (`test-streams.mux.dev`): 5 çözünürlüğün listelenmesi, 184p'nin 8 paralel parçayla indirilmesi | Geçti |
| YouTube | Bulut makinesinin IP'si YouTube tarafından "bot doğrulaması" ile engellendi |

YouTube engeli bu ortama özgüdür. Ev bağlantısında ve eklenti tarayıcı çerezlerini aktardığında
bu engel beklenmez; yine de YouTube indirmesi gerçek bir tarayıcıyla burada denenemedi.

## Masaüstü ortamları (Linux)

| Ortam | Nasıl | Sonuç |
|---|---|---|
| XFCE 4.18 (xfwm4 + xfce4-panel) | Kurulu `.deb` gerçek pencere yöneticisi ve panel altında çalıştırıldı | Pencere dekorasyonu, görev çubuğu girişi ve tepsi simgesi çalıştı. Kapatınca tepsiye küçüldü; ikinci çalıştırma aynı örneği öne getirdi |
| KDE | `--diagnose` ve `--integrate` komutları `XDG_CURRENT_DESKTOP=KDE` ile ve gerçek `kwriteconfig5`/`kreadconfig5` araçlarıyla çalıştırıldı | KDE algılandı, tepsi destekli. Konqueror `DownloadManager` değeri yazıldı ve geri okundu. Başlangıçta çalıştırma ve köprü kayıtları oluştu |
| GNOME | `--diagnose`, `XDG_CURRENT_DESKTOP=GNOME` | GNOME algılandı. AppIndicator uzantısı olmadığı için tepsi uyarısı verildi. `idc://` işleyicisi `mimeapps.list` üzerinden kaydedildi |
| Pencere yöneticisi olmayan X | Xvfb | İlk çalıştırma sihirbazı, ana pencere ve diyaloglar çalıştı |

**Sınırlama:** Plasma ve GNOME Shell oturumları bu sunucuda tam olarak açılamadı. KDE ve GNOME
için algılama ve entegrasyon adımları gerçek araçlarla denendi, ama bu ortamlarda pencere ve tepsi
görünümü denenmedi. Wayland oturumu da denenmedi; bu oturumda pencere konumlandırma bilinen bir
Wayland kısıtıdır.

## Paketler

| Paket | Sonuç |
|---|---|
| `.deb` | Derlendi ve bu makineye kuruldu. Kurulum betiği köprüyü Chrome, Chromium, Edge, Brave, Vivaldi, Opera ve Firefox için sistem genelinde kaydetti. `/usr/bin/idc-host` cevap verdi |
| `.rpm` | Derlendi (içerik `.deb` ile aynı) |
| AppImage | Derlendi (83 MB). Köprü, kalıcı bir yola kopyalanarak kaydedilir |
| Windows NSIS / taşınabilir zip | Bu makinede yalnız çapraz derleme denetimi yapıldı. Tüm crate'ler `x86_64-pc-windows-gnu` hedefi için uyarısız derlendi. Kurulum paketi GitHub Actions'ta (`windows-latest`) üretilir; Windows'ta çalışma testi yapılmadı |

## Elle denenecekler (sürümden önce önerilen)

- Windows 10/11'de NSIS kurulumu, Chrome ve Edge ile yakalama, kayıt defteri girişleri,
  kapat/uyku seçenekleri
- Firefox ve Zen'de `.xpi` kurulumu, erken yakalama yolu (`webRequest` blocking)
- Gerçek KDE Plasma ve GNOME oturumunda tepsi ve pencere davranışı
- Evdeki bir bilgisayarda YouTube kalite listesi ve indirme
